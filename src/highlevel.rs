//! Helpers for writing browser user interfaces (BUIs).
/// The API in this module is likely to change as ergonomics get better.
use {BuiService, ConnectionKeyType, SessionKeyType, EventChunkSender, CallbackArgReceiver, Config,
     launcher};
use {std, hyper, serde_json, futures};

use hyper::server::{Http, NewService, Request, Response};

use raii_change_tracker::DataTracker;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::net::SocketAddr;
use futures::{Future, Sink, Stream};
use futures::sync::mpsc;
use serde::Serialize;

pub struct NewBuiService {
    value: Box<Fn() -> std::result::Result<BuiService, std::io::Error> + Send + Sync>,
}

impl NewService for NewBuiService {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Instance = BuiService;

    fn new_service(&self) -> std::result::Result<Self::Instance, std::io::Error> {
        (self.value)()
    }
}

// ------

#[derive(Debug)]
pub enum ConnectionEventType {
    Connect(EventChunkSender),
    Disconnect,
}

#[derive(Debug)]
pub struct ConnectionEvent {
    pub typ: ConnectionEventType,
    pub session_key: SessionKeyType,
    pub connection_key: ConnectionKeyType,
}

#[derive(Serialize)]
struct EventStreamMessage<'a, T>
    where T: 'a + Serialize
{
    bui_backend: &'a T,
}

// ------

pub struct BuiAppInner<T>
    where T: Clone + PartialEq + Serialize // + Deserialize + 'static
{
    i_shared_arc: Arc<Mutex<DataTracker<T>>>,
    i_txers: Arc<Mutex<HashMap<ConnectionKeyType, (SessionKeyType, EventChunkSender)>>>,
    i_bui_server: BuiService,
    i_hyper_server: hyper::Server<NewBuiService, hyper::Body>,
}

impl<T> BuiAppInner<T>
    where T: Clone + PartialEq + Serialize + 'static
{
    pub fn new(jwt_secret: &[u8],
               shared_store: DataTracker<T>,
               addr: &SocketAddr,
               config: Config,
               chan_size: usize)
               -> (mpsc::Receiver<ConnectionEvent>, BuiAppInner<T>) {
        let (rx_conn, bui_server) = launcher(config, &jwt_secret, chan_size);

        let b2 = bui_server.clone();

        let mbc = NewBuiService { value: Box::new(move || Ok(b2.clone())) };
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
                let msg = EventStreamMessage { bui_backend: shared.as_ref() };
                let buf = serde_json::to_string(&msg).expect("encode");
                let buf = format!("data: {}\n\n", buf);
                buf.into()
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

            match chunk_sender.send(Ok(hc)).wait() {
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

                    for (connection_key, (session_key, tx)) in sources.drain() {

                        let hc: hyper::Chunk = {
                            let msg = EventStreamMessage { bui_backend: &new_value };
                            let buf = serde_json::to_string(&msg).expect("encode");
                            let buf = format!("data: {}\n\n", buf);
                            buf.into()
                        };

                        match tx.send(Ok(hc)).wait() {
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
        inner.i_hyper_server.handle().spawn(change_listener);

        (new_conn_rx, inner)
    }

    pub fn shared_arc(&self) -> &Arc<Mutex<DataTracker<T>>> {
        &self.i_shared_arc
    }

    pub fn bui_service(&self) -> &BuiService {
        &self.i_bui_server
    }

    pub fn hyper_server(&self) -> &hyper::Server<NewBuiService, hyper::Body> {
        &self.i_hyper_server
    }

    pub fn into_hyper_server(self) -> hyper::Server<NewBuiService, hyper::Body> {
        self.i_hyper_server
    }

    pub fn add_callback_listener(&mut self, channel_size: usize) -> CallbackArgReceiver {
        self.i_bui_server.add_callback_listener(channel_size)
    }
}
