//! Implements a web server for a Browser User Interface (BUI).

use std::pin::Pin;

use http;
use hyper;
#[cfg(feature = "bundle_files")]
use includedir;
use {futures, jsonwebtoken, serde, serde_json, std};

use hyper::header::ACCEPT;
use hyper::{Method, StatusCode};

use futures::channel::mpsc;
use futures::Future;

use parking_lot::Mutex;
use std::sync::Arc;

use crate::access_control;
use bui_backend_types::{AccessToken, CallbackDataAndSession, ConnectionKey, SessionKey};

#[cfg(feature = "serve_files")]
use std::io::Read;

use serde::{Deserialize, Serialize};

// ---------------------------
const JSON_TYPE: &'static str = "application/json";
const JSON_NULL: &'static str = "null";

/// The claims validated using JSON Web Tokens.
#[derive(Serialize, Deserialize, Debug, Clone)]
struct JwtClaims {
    key: SessionKey,
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
    #[cfg_attr(docsrs, doc(cfg(feature = "bundle_files")))]
    pub bundled_files: &'static includedir::Files,
    /// The number of messages in the event stream channel before blocking.
    pub channel_size: usize,
    /// The name of the cookie stored in the clients browser.
    pub cookie_name: String,
}

/// Wrapper around `hyper::body::Bytes` to enable sending data to clients.
pub type EventChunkSender = mpsc::Sender<hyper::body::Bytes>;

/// Wrap the sender to each connected event stream listener.
pub struct NewEventStreamConnection {
    /// A sink for messages send to each connection (one per client tab).
    pub chunk_sender: EventChunkSender,
    /// Identifier for each connected session (one per client browser).
    pub session_key: SessionKey,
    /// Identifier for each connection (one per client tab).
    pub connection_key: ConnectionKey,
    /// The path being requested (starts with `BuiService::events_prefix`).
    pub path: String,
}

type NewConnectionSender = mpsc::Sender<NewEventStreamConnection>;

pub(crate) type CallbackFnType<CB> =
    Box<dyn Fn(CallbackDataAndSession<CB>) -> futures::future::Ready<Result<(), ()>> + Send>;
pub(crate) type CbFuncArc<CB> = Arc<Mutex<Option<CallbackFnType<CB>>>>;

/// Handle HTTP requests and coordinate responses to data updates.
///
/// Implements `hyper::server::Service` to act as HTTP server and handle requests.
#[derive(Clone)]
pub struct BuiService<CB>
where
    CB: serde::de::DeserializeOwned + Clone + Send,
{
    config: Config,
    callback_listener: CbFuncArc<CB>,
    next_connection_key: Arc<Mutex<ConnectionKey>>,
    jwt_secret: Vec<u8>,
    encoding_key: jsonwebtoken::EncodingKey,
    valid_token: AccessToken,
    tx_new_connection: NewConnectionSender,
    events_prefix: String,
    mime_types: conduit_mime_types::Types,
}

fn _test_bui_service_is_clone<CB>()
where
    CB: serde::de::DeserializeOwned + Clone + Send,
{
    // Compile-time test to ensure BuiService implements Clone trait.
    fn implements<T: Clone>() {}
    implements::<BuiService<CB>>();
}

fn _test_bui_service_is_send<CB>()
where
    CB: serde::de::DeserializeOwned + Clone + Send,
{
    // Compile-time test to ensure BuiService implements Send trait.
    fn implements<T: Send>() {}
    implements::<BuiService<CB>>();
}

fn _test_callback_data_is_send<CB>()
where
    CB: serde::de::DeserializeOwned + Clone + Send,
{
    // Compile-time test to ensure CallbackDataAndSession implements Send trait.
    fn implements<T: Send>() {}
    implements::<CallbackDataAndSession<CB>>();
}

impl<CB> BuiService<CB>
where
    CB: serde::de::DeserializeOwned + Clone + Send,
{
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
                warn!("requested path {:?}, but got error {:?}", file_path, e);
                return None;
            }
        };
        let mut contents = Vec::new();
        match file.read_to_end(&mut contents) {
            Ok(_) => {}
            Err(e) => {
                warn!("when reading path {:?}, got error {:?}", file_path, e);
                return None;
            }
        }
        Some(contents)
    }

    /// Get the event stream path prefix.
    pub fn events_prefix(&self) -> &str {
        &self.events_prefix
    }

    fn get_next_connection_key(&self) -> ConnectionKey {
        let mut nk = self.next_connection_key.lock();
        let result = nk.clone();
        nk.0 = nk.0 + 1;
        result
    }

    /// Get a stream of callback events.
    pub fn set_callback_listener(&mut self, f: CallbackFnType<CB>) -> Option<CallbackFnType<CB>> {
        let mut cbl = self.callback_listener.lock();
        let previous = cbl.take();
        cbl.get_or_insert(f);
        previous
    }

    fn do_set_cookie_x(
        &self,
        resp: http::response::Builder,
    ) -> (http::response::Builder, SessionKey) {
        // There was no valid client key in the HTTP header, so generate a
        // new one and set it on client.
        let session_key = SessionKey::new();
        let claims = JwtClaims { key: session_key };

        let token = {
            jsonwebtoken::encode(
                &jsonwebtoken::Header::default(),
                &claims,
                &self.encoding_key,
            )
            .unwrap()
        };
        let mut c = cookie::Cookie::new(self.config.cookie_name.clone(), token);
        c.set_http_only(true);
        let resp = resp.header(
            hyper::header::SET_COOKIE,
            hyper::header::HeaderValue::from_str(&c.to_string()).unwrap(),
        );
        (resp, session_key)
    }
}

async fn handle_req<CB>(
    mut self_: BuiService<CB>,
    req: http::Request<hyper::Body>,
    mut resp: http::response::Builder,
    login_info: ValidLogin,
) -> Result<http::Response<hyper::Body>, http::Error>
where
    CB: serde::de::DeserializeOwned + Clone + Send,
{
    // TODO: convert this to be async yield when blocking on IO operations.
    let session_key = match login_info {
        ValidLogin::NeedsSessionKey => {
            let (resp2, session_key) = self_.do_set_cookie_x(resp);
            resp = resp2;
            session_key
        }
        ValidLogin::ExistingSession(k) => k,
    };

    let resp_final = match (req.method(), req.uri().path()) {
        (&Method::GET, path) => {
            let path = if path == "/" { "/index.html" } else { path };

            if path.starts_with(&self_.events_prefix) {
                // Quality value parsing disabled with the following hack
                // until this is addressed:
                // https://github.com/hyperium/http/issues/213

                let mut accepts_event_stream = false;
                for value in req.headers().get_all(ACCEPT).iter() {
                    if value
                        .to_str()
                        .expect("to_str()")
                        .contains("text/event-stream")
                    {
                        accepts_event_stream = true;
                    }
                }

                if accepts_event_stream {
                    let connection_key = self_.get_next_connection_key();
                    let (tx_event_stream, rx_event_stream) =
                        mpsc::channel(self_.config.channel_size);

                    {
                        let conn_info = NewEventStreamConnection {
                            chunk_sender: tx_event_stream,
                            session_key: session_key,
                            connection_key: connection_key,
                            path: path.to_string(),
                        };

                        use futures::sink::SinkExt;
                        let send_future = self_.tx_new_connection.send(conn_info);
                        match send_future.await {
                            Ok(()) => {}
                            Err(e) => {
                                error!("failed to send new connection info: {:?}", e);
                                // should we panic here?
                            }
                        };
                    }

                    resp = resp.header(
                        hyper::header::CONTENT_TYPE,
                        hyper::header::HeaderValue::from_str("text/event-stream")
                            .expect("from_str"),
                    );

                    use futures::stream::StreamExt;
                    let rx_event_stream2 =
                        rx_event_stream.map(|chunk| Ok::<_, hyper::Error>(chunk));
                    resp.body(hyper::Body::wrap_stream(rx_event_stream2))?
                // resp.body( hyper::Body::wrap_stream( rx_event_stream.map_err(|_| Error::RxEvent.compat() ) ) )?
                } else {
                    let estr = format!(
                        "Event request does not specify \
                        'Accept' or does not accept the required \
                        'text/event-stream'"
                    );
                    warn!("{}", estr);
                    let e = ErrorsBackToBrowser { errors: vec![estr] };
                    let body_str = serde_json::to_string(&e).unwrap();
                    resp = resp.status(StatusCode::BAD_REQUEST);
                    resp.body(body_str.into())?
                }
            } else {
                // TODO read file asynchronously
                match self_.get_file_content(path) {
                    Some(buf) => {
                        let path = std::path::Path::new(path);
                        let mime_type = match path.extension().map(|x| x.to_str()).unwrap_or(None) {
                            Some("wasm") => Some("application/wasm"),
                            Some(ext) => self_.mime_types.get_mime_type(ext),
                            None => None,
                        };

                        if let Some(mime_type) = mime_type {
                            resp = resp.header(
                                hyper::header::CONTENT_TYPE,
                                hyper::header::HeaderValue::from_str(mime_type).expect("from_str"),
                            );
                        }

                        resp.body(buf.into())?
                    }
                    None => {
                        resp = resp.status(StatusCode::NOT_FOUND);
                        resp.body(hyper::Body::empty())?
                    }
                }
            }
        }
        _ => {
            resp = resp.status(StatusCode::NOT_FOUND);
            resp.body(hyper::Body::empty())?
        }
    };
    Ok(resp_final)
}

async fn handle_callback<CB>(
    cbfunc: CbFuncArc<CB>,
    session_key: bui_backend_types::SessionKey,
    resp0: http::response::Builder,
    req: http::Request<hyper::Body>,
) -> Result<http::Response<hyper::Body>, hyper::Error>
where
    CB: 'static + serde::de::DeserializeOwned + Clone + Send,
{
    // fold all chunks into one Vec<u8>
    let body = req.into_body();
    use futures::stream::StreamExt;
    let chunks: Vec<Result<hyper::body::Bytes, hyper::Error>> = body.collect().await;
    use std::iter::FromIterator;
    let chunks: Result<Vec<hyper::body::Bytes>, hyper::Error> =
        Result::from_iter(chunks.into_iter());
    let chunks: Vec<hyper::body::Bytes> = chunks?;

    let data: Vec<u8> = chunks.into_iter().fold(vec![], |mut buf, chunk| {
        trace!("got chunk: {}", String::from_utf8_lossy(&chunk));
        buf.extend_from_slice(&*chunk);
        buf
    });

    // parse data

    // Here we convert from a Vec<u8> JSON buf to our
    // generic type `CB` whose definition can be shared
    // between backend and frontend if using a Rust frontend.
    // (If not using a rust frontend, the payload should be
    // constructed such that this conversion succeeds.
    match serde_json::from_slice::<CB>(&data) {
        Ok(payload) => {
            // valid data, parse it

            // Here create a future if we have a callback function but do not
            // only hold the lock briefly. Release the lock before awaiting the
            // result.
            let opt_fut = {
                let opt_this_cbfunc = cbfunc.lock();

                if let Some(ref this_cbfunc) = *opt_this_cbfunc {
                    let my_cb: &CallbackFnType<CB> = this_cbfunc;
                    let args = CallbackDataAndSession {
                        payload,
                        session_key,
                    };

                    Some(my_cb(args))
                } else {
                    None
                }
            };

            if let Some(fut) = opt_fut {
                let x = fut.await;

                // Send the payload to callback.
                let r0 = match x {
                    Ok(()) => {
                        let resp = resp0
                            .header(hyper::header::CONTENT_TYPE, JSON_TYPE)
                            .body(JSON_NULL.into())
                            .expect("response");
                        resp
                    }
                    Err(e) => {
                        error!("internal server error: {:?}", e);
                        let resp = resp0
                            .header(hyper::header::CONTENT_TYPE, JSON_TYPE)
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(JSON_NULL.into())
                            .expect("response");
                        resp
                    }
                };
                Ok(r0)
            } else {
                error!("no callback handler set");
                let resp = resp0
                    .header(hyper::header::CONTENT_TYPE, JSON_TYPE)
                    .body(JSON_NULL.into())
                    .expect("response");
                Ok(resp)
            }
        }
        Err(e) => Ok(on_json_parse_err(e)),
    }
}

fn on_json_parse_err(e: serde_json::Error) -> http::Response<hyper::Body> {
    let estr = format!("Failed parsing JSON: {}", e);
    warn!("{}", estr);
    let e = ErrorsBackToBrowser { errors: vec![estr] };
    let body_str = serde_json::to_string(&e).unwrap();
    http::Response::builder()
        .header(hyper::header::CONTENT_TYPE, JSON_TYPE)
        .status(StatusCode::BAD_REQUEST)
        .body(body_str.into())
        .expect("response")
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ErrorsBackToBrowser {
    errors: Vec<String>,
}

/// User can login either with URL query param (no session key provided) or
/// with cookie which includes the session key.
#[derive(Debug)]
enum ValidLogin {
    ExistingSession(SessionKey),
    NeedsSessionKey,
}

fn get_session_key<'a>(
    map: &hyper::HeaderMap<hyper::header::HeaderValue>,
    query_pairs: url::form_urlencoded::Parse,
    cookie_name: &str,
    decoding_key: &jsonwebtoken::DecodingKey<'a>,
    valid_token: &AccessToken,
) -> Result<ValidLogin, ErrorsBackToBrowser> {
    use std::borrow::Cow;

    let mut errors = Vec::new();

    // first check for token in URI
    for (key, value) in query_pairs {
        debug!("got query pair {}, {}", key, value);
        if key == Cow::Borrowed("token") {
            if valid_token.does_match(&value) {
                return Ok(ValidLogin::NeedsSessionKey);
            } else {
                warn!("incorrect token in URI: {}", value);
                let estr = format!("incorrect token in URI");
                errors.push(estr);
            }
        }
    }

    // if no token there, check cookie.

    for cookie in map.get_all(hyper::header::COOKIE).iter() {
        match cookie.to_str() {
            Ok(cookie_str) => {
                let res_c = cookie::Cookie::parse(cookie_str.to_string());
                match res_c {
                    Ok(c) => {
                        if c.name() == cookie_name {
                            let encoded = c.value();
                            debug!("jwt_encoded = {}", encoded);
                            let validation = jsonwebtoken::Validation {
                                validate_exp: false,
                                ..Default::default()
                            };
                            match jsonwebtoken::decode::<JwtClaims>(
                                &encoded,
                                decoding_key,
                                &validation,
                            )
                            .map(|token| token.claims.key)
                            {
                                Ok(k) => return Ok(ValidLogin::ExistingSession(k)),
                                Err(e) => {
                                    warn!("client passed token in cookie {:?}, resulting in error: {:?}", c, e);
                                    let estr = format!("{}: {:?}", e, e);
                                    errors.push(estr);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let estr = format!("cookie not parsed: {:?}", e);
                        warn!("{}", estr);
                        errors.push(estr);
                    }
                }
            }
            Err(e) => {
                let estr = format!("cookie not converted to str: {:?}", e);
                warn!("{}", estr);
                errors.push(estr);
            }
        }
    }

    // If we are here, we got no (valid) session key.
    debug!("no (valid) session key found");
    match valid_token {
        &AccessToken::NoToken => {
            debug!("no token needed, will give new session key");
            Ok(ValidLogin::NeedsSessionKey)
        }
        _ => {
            errors.push("no valid session key".to_string());
            Err(ErrorsBackToBrowser { errors })
        }
    }
}

impl<CB> tower_service::Service<http::Request<hyper::Body>> for BuiService<CB>
where
    CB: 'static + serde::de::DeserializeOwned + Clone + Send,
{
    type Response = http::Response<hyper::Body>;
    type Error = hyper::Error;

    // should Self::Future also implement Unpin??
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<hyper::Body>) -> Self::Future {
        let decoding_key = jsonwebtoken::DecodingKey::from_secret(&self.jwt_secret);
        // Parse cookies.
        let res_session_key = {
            let query = req.uri().query();
            debug!("parsing query {:?}", query);

            let pairs = url::form_urlencoded::parse(query.unwrap_or("").as_bytes());

            get_session_key(
                &req.headers(),
                pairs,
                &self.config.cookie_name,
                &decoding_key,
                &self.valid_token,
            )
        };

        debug!(
            "got request from session key {:?}: {:?}",
            res_session_key, req
        );

        if req.method() == &Method::POST {
            if req.uri().path() == "/callback" {
                let login_info = match res_session_key {
                    Ok(login_info) => login_info,
                    Err(errors) => {
                        warn!("no (valid) session key in callback");
                        let body_str = serde_json::to_string(&errors).unwrap();
                        let resp = http::Response::builder()
                            .header(hyper::header::CONTENT_TYPE, JSON_TYPE)
                            .status(StatusCode::BAD_REQUEST)
                            .body(body_str.into())
                            .expect("response");
                        return Box::pin(futures::future::ok(resp));
                    }
                };

                let mut resp0 = http::Response::builder();
                let session_key = match login_info {
                    ValidLogin::NeedsSessionKey => {
                        let (resp2, session_key) = self.do_set_cookie_x(resp0);
                        resp0 = resp2;
                        session_key
                    }
                    ValidLogin::ExistingSession(k) => k,
                };

                return Box::pin(handle_callback(
                    self.callback_listener.clone(),
                    session_key,
                    resp0,
                    req,
                ));
            }
        }

        let resp = http::Response::builder();

        let login_info = match res_session_key {
            Ok(login_info) => login_info,
            Err(_errors) => {
                let estr = format!("No (valid) token in request.");
                let errors = ErrorsBackToBrowser { errors: vec![estr] };

                let body_str = serde_json::to_string(&errors).unwrap();
                let resp = http::Response::builder()
                    .header(hyper::header::CONTENT_TYPE, JSON_TYPE)
                    .status(StatusCode::BAD_REQUEST)
                    .body(body_str.into())
                    .expect("response");
                return Box::pin(futures::future::ok(resp));
            }
        };

        use futures::FutureExt;
        let resp_final = handle_req(self.clone(), req, resp, login_info).map(|r| match r {
            Ok(x) => Ok(x),
            Err(_e) => unimplemented!(),
        });

        Box::pin(resp_final)
    }
}

/// Create a stream of connection events and a `BuiService`.
pub fn launcher<CB>(
    config: Config,
    auth: &access_control::AccessControl,
    channel_size: usize,
    events_prefix: &str,
) -> (mpsc::Receiver<NewEventStreamConnection>, BuiService<CB>)
where
    CB: serde::de::DeserializeOwned + Clone + Send,
{
    let next_connection_key = Arc::new(Mutex::new(ConnectionKey(0)));

    let (tx_new_connection, rx_new_connection) = mpsc::channel(channel_size);

    let service = BuiService {
        config: config,
        callback_listener: Arc::new(Mutex::new(None)),
        next_connection_key: next_connection_key,
        jwt_secret: auth.jwt_secret().to_vec(),
        encoding_key: jsonwebtoken::EncodingKey::from_secret(auth.jwt_secret()),
        valid_token: auth.token().clone(),
        tx_new_connection: tx_new_connection,
        events_prefix: events_prefix.to_string(),
        mime_types: conduit_mime_types::Types::new().expect("mime type init"),
    };

    (rx_new_connection, service)
}
