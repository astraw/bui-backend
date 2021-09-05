//! Helpers for writing browser user interfaces (BUIs).
use crate::lowlevel::{BuiService, EventChunkSender};
use bui_backend_types::{ConnectionKey, SessionKey};

use {futures, hyper, serde, serde_json, std};

use async_change_tracker::ChangeTracker;

use std::collections::HashMap;
use std::sync::Arc;

use futures::{channel::mpsc, future::FutureExt, sink::SinkExt, stream::StreamExt};
use parking_lot::RwLock;
use uuid::Uuid;

use serde::Serialize;

use hyper::server::conn::AddrStream;
use hyper::service::make_service_fn;

use bui_backend_types::AccessToken;

use crate::access_control;
use crate::lowlevel::{CallbackFnType, NewEventStreamConnection};
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
pub struct BuiAppInner<T, CB>
where
    T: Clone + PartialEq + Serialize + Send,
    CB: 'static + serde::de::DeserializeOwned + Clone + Send,
{
    i_shared_arc: Arc<RwLock<ChangeTracker<T>>>,
    i_txers: Arc<RwLock<HashMap<ConnectionKey, (SessionKey, EventChunkSender, String)>>>,
    i_bui_server: BuiService<CB>,
    auth: access_control::AccessControl,
    local_addr: std::net::SocketAddr,
}

impl<'a, T, CB> BuiAppInner<T, CB>
where
    T: Clone + PartialEq + Serialize + Send + 'static,
    CB: serde::de::DeserializeOwned + Clone + Send + 'static,
{
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

    /// Register a function to be called when the user makes a callback.
    pub fn set_callback_listener(&mut self, f: CallbackFnType<CB>) -> Option<CallbackFnType<CB>> {
        self.i_bui_server.set_callback_listener(f)
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
    shutdown_rx: Option<tokio::sync::oneshot::Receiver<()>>,
    auth: &access_control::AccessControl,
    shared_arc: Arc<RwLock<ChangeTracker<T>>>,
    event_name: Option<String>,
    rx_conn: mpsc::Receiver<NewEventStreamConnection>,
    bui_server: BuiService<CB>,
) -> Result<(mpsc::Receiver<ConnectionEvent>, BuiAppInner<T, CB>), Error>
where
    T: Clone + PartialEq + Serialize + 'static + Send + Sync + Unpin,
    CB: serde::de::DeserializeOwned + Clone + Send + 'static + Unpin,
{
    let (quit_trigger, valve) = stream_cancel::Valve::new();

    let bui_server: BuiService<CB> = bui_server; // type annotation

    // This line is just to annotate the type
    let rx_conn: mpsc::Receiver<NewEventStreamConnection> = rx_conn;

    let b2 = bui_server.clone();

    type MyError = std::io::Error; // anything that implements std::error::Error and Send

    let new_service = make_service_fn(move |socket: &AddrStream| {
        let _remote_addr = socket.remote_addr();
        let b3 = b2.clone();
        async move { Ok::<_, MyError>(b3.clone()) }
    });

    let addr = auth.bind_addr();

    // this will fail unless there is a reactor already
    let bound = async { hyper::Server::try_bind(&addr) }.await?;

    let server = bound.serve(new_service);

    let local_addr = server.local_addr();

    let log_and_swallow_err = |r| match r {
        Ok(_) => {}
        Err(e) => {
            error!("{} ({}:{})", e, file!(), line!());
        }
    };

    if let Some(shutdown_rx) = shutdown_rx {
        let graceful = server.with_graceful_shutdown(async move {
            shutdown_rx.await.ok();
            quit_trigger.cancel();
        });
        tokio::spawn(Box::pin(graceful.map(log_and_swallow_err)));
    } else {
        quit_trigger.disable();
        tokio::spawn(Box::pin(server.map(log_and_swallow_err)));
    };

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
    let mut new_conn_tx2 = new_conn_tx.clone();
    let event_name2 = event_name.clone();

    let mut rx_conn_valve = valve.wrap(rx_conn);

    let handle_connections_fut = async move {
        while let Some(conn_info) = rx_conn_valve.next().await {
            let chunk_sender = conn_info.chunk_sender;
            let mut chunk_sender: EventChunkSender = chunk_sender; // type annotation only
            let ckey = conn_info.session_key;
            let connection_key = conn_info.connection_key;

            // send current value on initial connect
            let hc: hyper::body::Bytes = {
                let shared = shared_arc.write();
                create_event_source_msg(&shared.as_ref(), event_name2.as_ref().map(|x| x.as_str()))
                    .into()
            };

            let typ = ConnectionEventType::Connect(chunk_sender.clone());
            let session_key = ckey.clone();
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

    tokio::spawn(Box::pin(handle_connections_fut));

    // --- push changes

    let shared_store2 = inner.i_shared_arc.clone();
    let txers = inner.i_txers.clone();
    // Create a Stream to handle updates to our shared store.
    let change_listener = {
        let rx = {
            let shared = shared_store2.write();
            shared.get_changes(10) // capacity of channel is 10 changes
        };
        let mut rx_valve = valve.wrap(rx);
        async move {
            while let Some((_old, new_value)) = rx_valve.next().await {
                // We need to hold the loc on txers only briefly, so we do this.
                let sources_drain = {
                    let mut sources = txers.write();
                    sources.drain().collect::<Vec<_>>()
                };

                let mut restore = vec![];

                let event_source_msg =
                    create_event_source_msg(&new_value, event_name.as_ref().map(|x| x.as_str()));

                for (connection_key, (session_key, mut tx, path)) in sources_drain {
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
                            let mut nct = new_conn_tx.clone();
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
    tokio::spawn(Box::pin(change_listener));

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
