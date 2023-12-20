//! Helpers for writing browser user interfaces (BUIs).
use crate::lowlevel::{BuiService, EventChunkSender};
use bui_backend_types::{ConnectionKey, SessionKey};

use async_change_tracker::ChangeTracker;

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::mpsc;

use parking_lot::RwLock;
use uuid::Uuid;

use serde::Serialize;

use bui_backend_types::AccessToken;

use crate::access_control;
use crate::lowlevel::NewEventStreamConnection;
use crate::Error;

// ------

/// The type of possible connect event, either connect or disconnect.
#[derive(Debug)]
pub enum ConnectionEventType {
    /// A connection event with sink for event stream messages to the connected client.
    Connect(EventChunkSender),
    /// A disconnection event.
    Disconnect,
}

/// State associated with connection or disconnection.
#[derive(Debug)]
pub struct ConnectionEvent {
    /// The type of connection for this event.
    pub typ: ConnectionEventType,
    /// Identifier for the connecting session (one ber browser).
    pub session_key: SessionKey,
    /// Identifier for the connection (one ber tab).
    pub connection_key: ConnectionKey,
    /// The path being requested (starts with `BuiService::events_prefix`).
    pub path: String,
}

// ------

/// Maintain state within a BUI application.
pub struct BuiAppInner<T, CB> {
    i_shared_arc: Arc<RwLock<ChangeTracker<T>>>,
    i_txers: Arc<RwLock<HashMap<ConnectionKey, (SessionKey, EventChunkSender, String)>>>,
    i_bui_server: BuiService<CB>,
    auth: access_control::AccessControl,
    local_addr: std::net::SocketAddr,
}

impl<'a, T, CB> BuiAppInner<T, CB> {
    /// Get reference counted reference to the underlying data store.
    pub fn shared_arc(&self) -> &Arc<RwLock<ChangeTracker<T>>> {
        &self.i_shared_arc
    }

    /// Get reference to to the underlying `BuiService`.
    pub fn bui_service(&self) -> &BuiService<CB> {
        &self.i_bui_server
    }

    /// Get our local IP address.
    pub fn local_addr(&self) -> &std::net::SocketAddr {
        &self.local_addr
    }

    /// Get our access token.
    pub fn token(&self) -> AccessToken {
        self.auth.token()
    }

    /// Attempt to get our URL.
    ///
    /// This may fail if, for example, the locally known IP address is
    /// not the IP address that users will connect to.
    pub fn guess_url_with_token(&self) -> String {
        match self.auth.token() {
            AccessToken::NoToken => format!("http://{}", self.local_addr),
            AccessToken::PreSharedToken(ref tok) => {
                format!("http://{}/?token={}", self.local_addr, tok)
            }
        }
    }
}

/// Generate a random token
pub fn generate_valid_token() -> String {
    let my_uuid = Uuid::new_v4();
    format!("{}", my_uuid)
}

/// Generate a random token and return access control information. Requires JWT secret.
pub fn generate_random_auth(
    addr: std::net::SocketAddr,
    secret: Vec<u8>,
) -> Result<access_control::AccessControl, Error> {
    generate_auth_with_token(addr, secret, generate_valid_token())
}

/// Return access control information given a token and a JWT secret.
pub fn generate_auth_with_token(
    addr: std::net::SocketAddr,
    secret: Vec<u8>,
    token: String,
) -> Result<access_control::AccessControl, Error> {
    let access_token = AccessToken::PreSharedToken(token);
    let info = access_control::AccessInfo::new(addr, access_token, secret)?;
    Ok(access_control::AccessControl::WithToken(info))
}

/// Factory function to create a new BUI application.
pub async fn create_bui_app_inner<'a, T, CB>(
    handle: tokio::runtime::Handle,
    mut shutdown_rx: Option<tokio::sync::oneshot::Receiver<()>>,
    auth: &access_control::AccessControl,
    shared_arc: Arc<RwLock<ChangeTracker<T>>>,
    event_name: Option<String>,
    rx_conn: mpsc::Receiver<NewEventStreamConnection>,
    bui_server: BuiService<CB>,
) -> Result<(mpsc::Receiver<ConnectionEvent>, BuiAppInner<T, CB>), Error>
where
    T: Clone + Serialize + 'static + Send + Sync,
    CB: serde::de::DeserializeOwned + Clone + Send + 'static,
{
    let (quit_trigger, valve) = stream_cancel::Valve::new();
    let rx_conn = tokio_stream::wrappers::ReceiverStream::new(rx_conn);

    let mut rx_conn_valve = valve.wrap(rx_conn);

    if let Some(shutdown_rx) = shutdown_rx.take() {
        handle.spawn(async move {
            shutdown_rx.await.unwrap();
            // Cancel the stream. (The receiver will receive end-of-stream.)
            quit_trigger.cancel();
        });
    } else {
        // Allow dropping `quit_trigger` without canceling stream.
        quit_trigger.disable();
    }

    let bui_server2 = bui_server.clone();

    let addr = auth.bind_addr();
    let listener = tokio::net::TcpListener::bind(addr).await?;

    let local_addr = listener.local_addr()?;
    let handle2 = handle.clone();

    handle.spawn(async move {
        loop {
            let (socket, _remote_addr) = listener.accept().await.unwrap();
            let bui_server = bui_server2.clone();

            // Spawn a task to handle the connection. That way we can multiple connections
            // concurrently.
            handle2.spawn(async move {
                // Hyper has its own `AsyncRead` and `AsyncWrite` traits and doesn't use tokio.
                // `TokioIo` converts between them.
                let socket = hyper_util::rt::TokioIo::new(socket);
                let bui_server = bui_server.clone();

                let hyper_service = hyper::service::service_fn(
                    move |request: hyper::Request<hyper::body::Incoming>| {
                        use hyper::service::Service;
                        // Do we need to call `poll_ready`????
                        bui_server.call(request)
                    },
                );

                // `server::conn::auto::Builder` supports both http1 and http2.
                //
                // `TokioExecutor` tells hyper to use `tokio::spawn` to spawn tasks.
                if let Err(err) = hyper_util::server::conn::auto::Builder::new(
                    hyper_util::rt::TokioExecutor::new(),
                )
                // `serve_connection_with_upgrades` is required for websockets. If you don't need
                // that you can use `serve_connection` instead.
                .serve_connection_with_upgrades(socket, hyper_service)
                .await
                {
                    eprintln!("failed to serve connection: {err:#}");
                }
            });
        }
    });

    let inner = BuiAppInner {
        i_shared_arc: shared_arc,
        i_txers: Arc::new(RwLock::new(HashMap::new())),
        i_bui_server: bui_server,
        auth: auth.clone(),
        local_addr,
    };

    // --- handle connections
    let (new_conn_tx, new_conn_rx) = mpsc::channel(5); // TODO chan_size

    let shared_arc = inner.i_shared_arc.clone();
    let txers2 = inner.i_txers.clone();
    let new_conn_tx2 = new_conn_tx.clone();
    let event_name2: Option<String> = event_name.clone();

    let handle_connections_fut = async move {
        while let Some(conn_info) = futures::StreamExt::next(&mut rx_conn_valve).await {
            let chunk_sender = conn_info.chunk_sender;
            let chunk_sender: EventChunkSender = chunk_sender; // type annotation only
            let ckey = conn_info.session_key;
            let connection_key = conn_info.connection_key;

            // send current value on initial connect
            let hc: hyper::body::Bytes = {
                let shared = shared_arc.write();
                create_event_source_msg(&shared.as_ref(), event_name2.as_deref()).into()
            };

            let typ = ConnectionEventType::Connect(chunk_sender.clone());
            let session_key = ckey;
            let path = conn_info.path.clone();
            let path2 = conn_info.path.clone();

            match new_conn_tx2
                .send(ConnectionEvent {
                    typ,
                    session_key,
                    connection_key,
                    path,
                })
                .await
            {
                Ok(()) => {}
                Err(e) => {
                    info!(
                        "failed sending ConnectionEvent. probably no listener. {:?}",
                        e
                    );
                }
            };

            match chunk_sender.send(hc).await {
                Ok(()) => {
                    let mut txer_guard = txers2.write();
                    txer_guard.insert(connection_key, (ckey, chunk_sender, path2));
                }
                Err(e) => {
                    error!("failed to send value on initial connect: {:?}", e);
                }
            }
        }
    };

    handle.spawn(Box::pin(handle_connections_fut));

    // --- push changes

    let shared_store2 = inner.i_shared_arc.clone();
    let txers = inner.i_txers.clone();
    // Create a Stream to handle updates to our shared store.
    let change_listener = {
        let mut rx = {
            let shared = shared_store2.write();
            shared.get_changes(10) // capacity of channel is 10 changes
        };
        async move {
            while let Some((_old, new_value)) = futures::StreamExt::next(&mut rx).await {
                // We need to hold the loc on txers only briefly, so we do this.
                let sources_drain = {
                    let mut sources = txers.write();
                    sources.drain().collect::<Vec<_>>()
                };

                let mut restore = vec![];

                let event_source_msg = create_event_source_msg(&new_value, event_name.as_deref());

                for (connection_key, (session_key, tx, path)) in sources_drain {
                    let chunk = event_source_msg.clone().into();
                    match tx.send(chunk).await {
                        Ok(()) => {
                            restore.push((connection_key, (session_key, tx, path)));
                        }
                        Err(e) => {
                            info!(
                                "Failed to send data to event stream, client \
                                    probably disconnected. {:?}",
                                e
                            );
                            let nct = new_conn_tx.clone();
                            let typ = ConnectionEventType::Disconnect;
                            let ce = ConnectionEvent {
                                typ,
                                session_key,
                                connection_key,
                                path,
                            };
                            match nct.send(ce).await {
                                Ok(()) => {}
                                Err(e) => {
                                    info!(
                                        "Failed to send ConnectionEvent, \
                                    probably no listener. {:?}",
                                        e
                                    );
                                }
                            };
                        }
                    };
                }
                for (connection_key, element) in restore.into_iter() {
                    let mut sources = txers.write();
                    sources.insert(connection_key, element);
                }
            }
        }
    };
    handle.spawn(Box::pin(change_listener));

    Ok((new_conn_rx, inner))
}

fn create_event_source_msg<T: serde::Serialize>(value: &T, event_name: Option<&str>) -> String {
    let buf = serde_json::to_string(&value).expect("encode");
    if let Some(event_name) = event_name {
        format!("event: {}\ndata: {}\n\n", event_name, buf)
    } else {
        format!("data: {}\n\n", buf)
    }
}
