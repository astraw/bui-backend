//! Helpers for writing browser user interfaces (BUIs).
//!
//! The API in this module is likely to change as ergonomics get better.
use lowlevel::{BuiService, ConnectionKeyType, SessionKeyType, EventChunkSender,
               CallbackDataAndSession, Config, launcher, NewBuiService};
use {std, hyper, serde, serde_json, futures};

use hyper::server::Http;

use raii_change_tracker::DataTracker;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::net::SocketAddr;
use futures::{Future, Sink, Stream};
use futures::sync::mpsc;
use serde::Serialize;

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
    pub session_key: SessionKeyType,
    /// Identifier for the connection (one ber tab).
    pub connection_key: ConnectionKeyType,
    /// The path being requested (starts with `BuiService::events_prefix`).
    pub path: String,
}

// ------

/// Maintain state within a BUI application.
pub struct BuiAppInner<T>
    where T: Clone + PartialEq + Serialize
{
    i_shared_arc: Arc<Mutex<DataTracker<T>>>,
    i_txers: Arc<Mutex<HashMap<ConnectionKeyType, (SessionKeyType, EventChunkSender, String)>>>,
    i_bui_server: BuiService,
    i_hyper_server: hyper::Server<NewBuiService, hyper::Body>,
}

impl<T> BuiAppInner<T>
    where T: Clone + PartialEq + Serialize + 'static
{
    /// Get reference counted reference to the underlying data store.
    pub fn shared_arc(&self) -> &Arc<Mutex<DataTracker<T>>> {
        &self.i_shared_arc
    }

    /// Get reference to to the underlying `BuiService`.
    pub fn bui_service(&self) -> &BuiService {
        &self.i_bui_server
    }

    /// Get reference to the underlying hyper server.
    pub fn hyper_server(&self) -> &hyper::Server<NewBuiService, hyper::Body> {
        &self.i_hyper_server
    }

    /// Drop self and return only the underlying hyper server.
    pub fn into_hyper_server(self) -> hyper::Server<NewBuiService, hyper::Body> {
        self.i_hyper_server
    }

    /// Get a stream of callback events.
    pub fn add_callback_listener(&mut self,
                                 channel_size: usize)
                                 -> mpsc::Receiver<CallbackDataAndSession> {
        self.i_bui_server.add_callback_listener(channel_size)
    }
}

/// Factory function to create a new BUI application.
pub fn create_bui_app_inner<T>(jwt_secret: &[u8],
                               shared_store: DataTracker<T>,
                               addr: &SocketAddr,
                               config: Config,
                               chan_size: usize,
                               events_prefix: &str)
                               -> (mpsc::Receiver<ConnectionEvent>, BuiAppInner<T>)
    where T: Clone + PartialEq + Serialize + 'static
{
    let (rx_conn, bui_server) = launcher(config, &jwt_secret, chan_size, events_prefix);

    let b2 = bui_server.clone();

    let mbc = NewBuiService::new(Box::new(move || Ok(b2.clone())));
    let hyper_server = Http::new().bind(&addr, mbc).unwrap();

    let inner = BuiAppInner {
        i_shared_arc: Arc::new(Mutex::new(shared_store)),
        i_txers: Arc::new(Mutex::new(HashMap::new())),
        i_bui_server: bui_server,
        i_hyper_server: hyper_server,
    };

    // --- handle_connections future
    let (new_conn_tx, new_conn_rx) = mpsc::channel(1); // TODO chan_size

    let shared_arc = inner.i_shared_arc.clone();
    let txers2 = inner.i_txers.clone();
    let new_conn_tx2 = new_conn_tx.clone();
    let handle_connections = rx_conn.for_each(move |conn_info| {

        let chunk_sender = conn_info.chunk_sender;
        let ckey = conn_info.session_key;
        let connection_key = conn_info.connection_key;

        // send current value on initial connect
        let hc: hyper::Chunk = {
            let shared = shared_arc.lock().unwrap();
            create_event_source_msg(&shared.as_ref()).into()
        };

        let nct = new_conn_tx2.clone();
        let typ = ConnectionEventType::Connect(chunk_sender.clone());
        let session_key = ckey.clone();
        let path = conn_info.path.clone();
        match nct.send(ConnectionEvent {
                           typ,
                           session_key,
                           connection_key,
                           path,
                       })
                  .wait() {
            Ok(_tx) => {}
            Err(e) => {
                info!("failed sending ConnectionEvent. probably no listener. {:?}",
                      e);
            }
        };

        match chunk_sender.send(Ok(hc)).wait() {
            Ok(chunk_sender) => {
                let mut txer_guard = txers2.lock().unwrap();
                txer_guard.insert(connection_key, (ckey, chunk_sender, conn_info.path));
                futures::future::ok(())
            }
            Err(e) => {
                error!("failed to send value on initial connect: {:?}", e);
                futures::future::err(())
            }
        }
    });
    inner.i_hyper_server.handle().spawn(handle_connections);


    // --- push changes

    let shared_store2 = inner.i_shared_arc.clone();
    let txers = inner.i_txers.clone();
    // Create a Stream to handle updates to our shared store.
    let change_listener = {
        let rx = {
            let mut shared = shared_store2.lock().unwrap();
            shared.add_listener()
        };
        let rx = rx.for_each(move |x| {
            let (_old, new_value) = x;
            {
                let mut sources = txers.lock().unwrap();
                let mut restore = vec![];

                let event_source_msg = create_event_source_msg(&new_value);

                for (connection_key, (session_key, tx, path)) in sources.drain() {

                    let chunk = event_source_msg.clone().into();

                    match tx.send(Ok(chunk)).wait() {
                        Ok(tx) => {
                            restore.push((connection_key, (session_key, tx, path)));
                        }
                        Err(e) => {
                            info!("Failed to send data to event stream, client \
                                    probably disconnected. {:?}",
                                  e);
                            let nct = new_conn_tx.clone();
                            let typ = ConnectionEventType::Disconnect;
                            let ce = ConnectionEvent {
                                typ,
                                session_key,
                                connection_key,
                                path,
                            };
                            match nct.send(ce).wait() {
                                Ok(_tx) => {}
                                Err(e) => {
                                    info!("Failed to send ConnectionEvent, \
                                    probably no listener. {:?}",
                                          e);
                                }
                            };

                        }
                    };
                }
                for (connection_key, element) in restore.into_iter() {
                    sources.insert(connection_key, element);
                }
            }
            let res: std::result::Result<(), ()> = Ok(());
            res
        });
        rx
    };
    inner.i_hyper_server.handle().spawn(change_listener);

    (new_conn_rx, inner)
}

fn create_event_source_msg<T: serde::Serialize>(value: &T) -> String {
    let buf = serde_json::to_string(&value).expect("encode");
    format!("event: bui_backend\ndata: {}\n\n", buf)
}
