//! Implements a web server for a Browser User Interface (BUI).
use {serde_json, std, futures, jsonwebtoken};
#[cfg(feature = "bundle_files")]
use includedir;
use failure::Fail;
use http;
use hyper;

use hyper::{Method, StatusCode};
use hyper::{Request, Response};
use hyper::header::ACCEPT;
use uuid::Uuid;

use futures::{Future, Stream, Sink};
use futures::sync::mpsc;

use std::sync::{Arc, Mutex};

use Error;

#[cfg(feature = "serve_files")]
use std::io::Read;

// ---------------------------

/// Alias for `Uuid` indicating that sessions are tracked
/// by keys of this type (one per client browser).
pub type SessionKeyType = Uuid;

/// The claims validated using JSON Web Tokens.
#[derive(Serialize, Deserialize, Debug, Clone)]
struct JwtClaims {
    key: SessionKeyType,
}

/// Callback data from a connected client.
#[derive(Clone, Debug)]
pub struct CallbackDataAndSession {
    /// The name of the callback sent from the client.
    pub name: String,
    /// The arguments of the callback sent from the client.
    pub args: serde_json::Value,
    /// The session key associated with the client.
    pub session_key: SessionKeyType,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct WireCallbackData {
    name: String,
    args: serde_json::Value,
}

/// Configuration settings for `BuiService`.
///
/// Defaults can be loaded using the function `get_default_config()`
/// generated by the `bui_backend_codegen` crate.
#[derive(Clone)]
pub struct Config {
    /// Location of the files to be served.
    pub serve_filepath: &'static std::path::Path,
    /// The bundled files.
    #[cfg(feature = "bundle_files")]
    pub bundled_files: &'static includedir::Files,
    /// The number of messages in the event stream channel before blocking.
    pub channel_size: usize,
    /// The name of the cookie stored in the clients browser.
    pub cookie_name: String,
}

/// Wrapper around `hyper::Chunk` to enable sending data to clients.
pub type EventChunkSender = mpsc::Sender<hyper::Chunk>;
// pub type EventChunkSender = mpsc::Sender<std::result::Result<hyper::Chunk, hyper::Error>>;

/// Wrap the sender to each connected event stream listener.
pub struct NewEventStreamConnection {
    /// A sink for messages send to each connection (one per client tab).
    pub chunk_sender: EventChunkSender,
    /// Identifier for each connected session (one per client browser).
    pub session_key: SessionKeyType,
    /// Identifier for each connection (one per client tab).
    pub connection_key: ConnectionKeyType,
    /// The path being requested (starts with `BuiService::events_prefix`).
    pub path: String,
}

type NewConnectionSender = mpsc::Sender<NewEventStreamConnection>;

/// Alias for `u32` to identify each connected event stream listener (one per client tab).
pub type ConnectionKeyType = u32;

/// Handle HTTP requests and coordinate responses to data updates.
///
/// Implements `hyper::server::Service` to act as HTTP server and handle requests.
#[derive(Clone)]
pub struct BuiService {
    config: Config,
    callback_senders: Arc<Mutex<Vec<mpsc::Sender<CallbackDataAndSession>>>>,
    next_connection_key: Arc<Mutex<ConnectionKeyType>>,
    jwt_secret: Arc<Mutex<Option<Vec<u8>>>>,
    tx_new_connection: NewConnectionSender,
    events_prefix: String,
}

impl BuiService {
    fn fullpath(&self, path: &str) -> String {
        assert!(path.starts_with("/")); // security check
        let path = std::path::PathBuf::from(path)
            .strip_prefix("/")
            .unwrap()
            .to_path_buf();
        assert!(!path.starts_with("..")); // security check

        let base = std::path::PathBuf::from(self.config.serve_filepath);
        let result = base.join(path);
        result.into_os_string().into_string().unwrap()
    }

    #[cfg(feature = "bundle_files")]
    fn get_file_content(&self, file_path: &str) -> Option<Vec<u8>> {
        let fullpath = self.fullpath(file_path);
        let r = self.config.bundled_files.get(&fullpath);
        match r {
            Ok(s) => Some(s.into_owned()),
            Err(_) => None,
        }
    }

    #[cfg(feature = "serve_files")]
    fn get_file_content(&self, file_path: &str) -> Option<Vec<u8>> {
        let fullpath = self.fullpath(file_path);
        let mut file = match std::fs::File::open(&fullpath) {
            Ok(f) => f,
            Err(e) => {
                error!("requested path {:?}, but got error {:?}", file_path, e);
                return None;
            }
        };
        let mut contents = Vec::new();
        match file.read_to_end(&mut contents) {
            Ok(_) => {}
            Err(e) => {
                error!("when reading path {:?}, got error {:?}", file_path, e);
                return None;
            }
        }
        Some(contents)
    }

    /// Get the event stream path prefix.
    pub fn events_prefix(&self) -> &str {
        &self.events_prefix
    }

    fn get_next_connection_key(&self) -> ConnectionKeyType {
        let mut nk = self.next_connection_key.lock().unwrap();
        let result = *nk;
        *nk += 1;
        result
    }

    /// Get a stream of callback events.
    pub fn add_callback_listener(&mut self,
                                 channel_size: usize)
                                 -> mpsc::Receiver<CallbackDataAndSession> {
        let (tx, rx) = mpsc::channel(channel_size);
        {
            let mut cb_tx_vec = self.callback_senders.lock().unwrap();
            cb_tx_vec.push(tx);
        }
        rx
    }

    fn handle_callback(&self,
        req: Request<hyper::Body>,
        session_key: SessionKeyType,
        )
        -> Box<Future<Item=Response<hyper::Body>, Error=hyper::Error> + Send>
    {

        let cbsenders = self.callback_senders.clone();

        // fold all chunks into one Vec<u8>
        let all_chunks_future = req.into_body()
            .fold(vec![], |mut buf, chunk| {
                    buf.extend_from_slice(&*chunk);
                futures::future::ok::<_, hyper::Error>(buf)
            });

        // parse data
        let fut = all_chunks_future
            .and_then(move |data: Vec<u8>| {
                match serde_json::from_slice::<WireCallbackData>(&data) {
                    Ok(data) => {
                        {
                            // valid data, parse it
                            let mut cb_tx_vec = cbsenders.lock().unwrap();
                            let mut restore_tx = Vec::new();

                            let cmd_name = data.name.clone();
                            let args = CallbackDataAndSession {
                                name: data.name,
                                args: data.args,
                                session_key: session_key,
                            };
                            for tx in cb_tx_vec.drain(..) {
                                // TODO can we somehow do this without waiting?
                                match tx.send(args.clone()).wait() {
                                    Ok(t) => restore_tx.push(t),
                                    Err(e) => {
                                        // listener failed
                                        warn!("when sending callback {:?}, error: {:?}",
                                                cmd_name,
                                                e);
                                    }
                                };
                            }

                            for tx in restore_tx.into_iter() {
                                cb_tx_vec.push(tx);
                            }

                        }
                        let resp = Response::builder()
                            .body(hyper::Body::empty()).expect("response");
                        futures::future::ok(resp)
                    }
                    Err(e) => {
                        error!("Failed parsing JSON to WireCallbackData: {:?}", e);
                        let resp = Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(hyper::Body::empty()).expect("response");
                        futures::future::ok(resp)
                    }
                }
            });
        Box::new(fut)
    }

    fn handle_req(&self, req: &hyper::Request<hyper::Body>, mut resp: http::response::Builder, session_key: &SessionKeyType) -> Result<http::Response<hyper::Body>, http::Error> {
        let resp_final = match (req.method(), req.uri().path()) {
            (&Method::GET, path) => {

                let path = if path == "/" { "/index.html" } else { path };

                if path.starts_with(&self.events_prefix) {

                    // Quality value parsing disabled with the following hack
                    // until this is addressed:
                    // https://github.com/hyperium/http/issues/213

                    let mut accepts_event_stream = false;
                    for value in req.headers().get_all(ACCEPT).iter() {
                        if value.to_str().expect("to_str()").contains("text/event-stream") {
                            accepts_event_stream = true;
                        }
                    }

                    if accepts_event_stream {
                        let connection_key = self.get_next_connection_key();
                        let (tx_event_stream, rx_event_stream) =
                            mpsc::channel(self.config.channel_size);

                        {
                            let tx_new_conn = self.tx_new_connection.clone();
                            let conn_info = NewEventStreamConnection {
                                chunk_sender: tx_event_stream,
                                session_key: *session_key,
                                connection_key: connection_key,
                                path: path.to_string(),
                            };

                            match tx_new_conn.send(conn_info).wait() {
                                Ok(_tx) => {} // Cloned above, so don't need to keep _tx here.
                                Err(e) => {
                                    error!("failed to send new connection info: {:?}", e);
                                    // should we panic here?
                                }
                            };
                        }

                        resp.header(
                            hyper::header::CONTENT_TYPE,
                            hyper::header::HeaderValue::from_str("text/event-stream").expect("from_str"));

                        // resp.header(hyper::header::ContentType(mime::TEXT_EVENT_STREAM))
                            // .with_body(rx_event_stream);
                        resp.body( hyper::Body::wrap_stream( rx_event_stream.map_err(|_| Error::RxEvent.compat() ) ) )?
                    } else {
                        error!("Event request does specify 'Accept' or does \
                            not accept the required 'text/event-stream'");
                        resp.status(StatusCode::BAD_REQUEST);
                        resp.body(hyper::Body::empty())?
                    }
                } else {
                    // TODO read file asynchronously
                    match self.get_file_content(path) {
                        Some(buf) => {
                            let len = buf.len();
                            let body = hyper::Body::from(buf);
                            resp.header(hyper::header::CONTENT_LENGTH,
                                format!("{}",len).as_bytes());
                            resp.body(body)?
                        }
                        None => {
                            resp.status(StatusCode::NOT_FOUND);
                            resp.body(hyper::Body::empty())?
                        }
                    }
                }
            }
            _ => {
                resp.status(StatusCode::NOT_FOUND);
                resp.body(hyper::Body::empty())?
            }
        };
        Ok(resp_final)
    }

}

fn get_client_key(map: &hyper::HeaderMap<hyper::header::HeaderValue>,
                  cookie_name: &str,
                  jwt_secret: &[u8])
                  -> Option<SessionKeyType> {
    let mut result = None;
    let valid_start = format!("{}=", cookie_name);
    for cookie in map.get_all(hyper::header::COOKIE).iter() {
        match cookie.to_str() {
            Ok(k) => {
                if k.starts_with(&valid_start) {
                    let encoded = &k[valid_start.len()..];
                    match jsonwebtoken::decode::<JwtClaims>(&encoded,
                                                            jwt_secret,
                                                            &jsonwebtoken::Validation::default())
                                    .map(|token| token.claims.key) {
                        Ok(k) => {result = Some(k)},
                        Err(e) => {
                            warn!("client passed token {:?}, resulting in error: {:?}", k, e);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("cookie not converted to str: {:?}", e);
            }
        }
    }
    result
}

impl hyper::service::Service for BuiService {
    type ReqBody = hyper::Body;
    type ResBody = hyper::Body;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Response<Self::ResBody>, Error=Self::Error>+Send>;

    fn call(&mut self, req: Request<Self::ReqBody>) -> Self::Future {
        // Parse cookies.
        let opt_client_key = {
            if let Some(ref jwt_secret) = *self.jwt_secret.lock().unwrap() {
                get_client_key(&req.headers(), &self.config.cookie_name, &*jwt_secret)
            } else {
                None
            }
        };

        trace!("got request from key {:?}: {:?}", opt_client_key, req);

        if req.method() == &Method::POST {
            if req.uri().path() == "/callback" {
                let session_key = if let Some(session_key) = opt_client_key {
                    session_key
                } else {
                    error!("no client key in callback");
                    let resp = Response::builder()
                        // .header(hyper::header::CONTENT_TYPE, "text/plain")
                        .status(StatusCode::BAD_REQUEST)
                        .body(hyper::Body::empty()).expect("response");
                    return Box::new(futures::future::ok(resp));
                };

                return self.handle_callback( req, session_key );
            }
        }

        let mut resp = Response::builder();

        let session_key = if let Some(key) = opt_client_key {
            key
        } else {
            // There was no valid client key in the HTTP header, so generate a
            // new one and set it on client.
            let session_key = Uuid::new_v4();
            let claims = JwtClaims { key: session_key.clone() };


            if let Some(ref jwt_secret) = *self.jwt_secret.lock().unwrap() {
                let token = {
                    jsonwebtoken::encode(&jsonwebtoken::Header::default(), &claims, &*jwt_secret)
                        .unwrap()
                };
                let cookie = format!("{}={}", self.config.cookie_name, token);
                resp.header(
                    hyper::header::SET_COOKIE,
                    hyper::header::HeaderValue::from_str(&cookie).unwrap());
            }
            session_key
        };

        let resp_final = self.handle_req(&req, resp, &session_key)
            .expect("handle_req"); // todo map err
        Box::new(futures::future::ok(resp_final))
    }
}

/// Create a stream of connection events and a `BuiService`.
pub fn launcher(config: Config,
                jwt_secret: Option<&[u8]>,
                channel_size: usize,
                events_prefix: &str)
                -> (mpsc::Receiver<NewEventStreamConnection>, BuiService) {
    let next_connection_key = Arc::new(Mutex::new(0));

    let callback_senders = Arc::new(Mutex::new(Vec::new()));

    let jwt_secret = jwt_secret.map(|x| x.to_vec());
    let (tx_new_connection, rx_new_connection) = mpsc::channel(channel_size);

    let service = BuiService {
        config: config,
        callback_senders: callback_senders,
        next_connection_key: next_connection_key,
        jwt_secret: Arc::new(Mutex::new(jwt_secret.clone())),
        tx_new_connection: tx_new_connection,
        events_prefix: events_prefix.to_string(),
    };

    (rx_new_connection, service)
}
