//! Helpers for writing browser user interfaces (BUIs).
//!
//! The API in this module is likely to change as ergonomics get better.
use lowlevel::{BuiService, ConnectionKeyType, SessionKeyType, EventChunkSender,
               CallbackDataAndSession, Config, launcher};
use {std, hyper, serde, serde_json, futures};

use change_tracker::DataTracker;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::net::SocketAddr;
use futures::{Future, Sink, Stream};
use futures::sync::mpsc;
use tokio_executor::Executor;
use serde::Serialize;

use ::Error;

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
}

// ------

/// Maintain state within a BUI application.
pub struct BuiAppInner<T>
    where T: Clone + PartialEq + Serialize + Send
{
    i_shared_arc: Arc<Mutex<DataTracker<T>>>,
    i_txers: Arc<Mutex<HashMap<ConnectionKeyType, (SessionKeyType, EventChunkSender)>>>,
    i_bui_server: BuiService,
}

impl<T> BuiAppInner<T>
    where T: Clone + PartialEq + Serialize + Send + 'static
{
    /// Get reference counted reference to the underlying data store.
    pub fn shared_arc(&self) -> &Arc<Mutex<DataTracker<T>>> {
        &self.i_shared_arc
    }

    /// Get reference to to the underlying `BuiService`.
    pub fn bui_service(&self) -> &BuiService {
        &self.i_bui_server
    }

    /// Get a stream of callback events.
    pub fn add_callback_listener(&mut self,
                                 channel_size: usize)
                                 -> mpsc::Receiver<CallbackDataAndSession> {
        self.i_bui_server.add_callback_listener(channel_size)
    }
}

/// Factory function to create a new BUI application.
pub fn create_bui_app_inner<T>(my_executor: &mut Executor,
                               jwt_secret: Option<&[u8]>,
                               shared_arc: Arc<Mutex<DataTracker<T>>>,
                               addr: &SocketAddr,
                               config: Config,
                               chan_size: usize,
                               events_path: &str)
                               -> Result<(mpsc::Receiver<ConnectionEvent>, BuiAppInner<T>), Error>
    where T: Clone + PartialEq + Serialize + 'static + Send,
{
    let (rx_conn, bui_server) = launcher(config, jwt_secret, chan_size, events_path);

    let b2 = bui_server.clone();

    let new_service = move || ->  Result<_,hyper::Error> { Ok(b2.clone()) };

    let server = hyper::Server::bind(&addr)
        .serve(new_service);

    my_executor.spawn(Box::new(server.map_err(|e| {
        eprintln!("server error: {}", e);
    })))?;

    let inner = BuiAppInner {
        i_shared_arc: shared_arc,
        i_txers: Arc::new(Mutex::new(HashMap::new())),
        i_bui_server: bui_server,
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
        match nct.send(ConnectionEvent {
                           typ,
                           session_key,
                           connection_key,
                       })
                  .wait() {
            Ok(_tx) => {}
            Err(e) => {
                info!("failed sending ConnectionEvent. probably no listener. {:?}",
                      e);
            }
        };

        // TODO: get rid of wait here?
        match chunk_sender.send(hc).wait() {
            Ok(chunk_sender) => {
                let mut txer_guard = txers2.lock().unwrap();
                txer_guard.insert(connection_key, (ckey, chunk_sender));
                futures::future::ok(())
            }
            Err(e) => {
                error!("failed to send value on initial connect: {:?}", e);
                futures::future::err(())
            }
        }
    });
    my_executor.spawn(Box::new(handle_connections))?;

    // --- push changes

    let shared_store2 = inner.i_shared_arc.clone();
    let txers = inner.i_txers.clone();
    // Create a Stream to handle updates to our shared store.
    let change_listener = {
        let rx = {
            let mut shared = shared_store2.lock().unwrap();
            shared.get_changes()
        };
        let rx = rx.for_each(move |x| {
            let (_old, new_value) = x;
            {
                let mut sources = txers.lock().unwrap();
                let mut restore = vec![];

                let event_source_msg = create_event_source_msg(&new_value);

                for (connection_key, (session_key, tx)) in sources.drain() {

                    let chunk = event_source_msg.clone().into();

                    match tx.send(chunk).wait() { // TODO: can I really wait here?
                        Ok(tx) => {
                            restore.push((connection_key, (session_key, tx)));
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
    let send_fut: Box<Future<Item=_,Error=_>+Send> = Box::new(change_listener);
    my_executor.spawn(send_fut)?;

    Ok((new_conn_rx, inner))
}

fn create_event_source_msg<T: serde::Serialize>(value: &T) -> String {
    let buf = serde_json::to_string(&value).expect("encode");
    format!("event: bui_backend\ndata: {}\n\n", buf)
}
