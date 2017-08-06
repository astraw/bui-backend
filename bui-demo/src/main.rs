#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate raii_change_tracker;
extern crate bui_backend;
extern crate includedir;
extern crate phf;
extern crate serde_json;
extern crate clap;
extern crate hyper;
extern crate dotenv;
extern crate futures;
extern crate serde;
extern crate tokio_core;

use raii_change_tracker::DataTracker;
use bui_backend::errors::Result;
use bui_backend::highlevel::{BuiAppInner, create_bui_app_inner};

use futures::{Future, Stream};

include!(concat!(env!("OUT_DIR"), "/public.rs")); // Despite slash, this does work on Windows.

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
struct Shared {
    is_recording: bool,
    counter: usize,
    name: String,
}

fn jwt_secret(matches: &clap::ArgMatches) -> Result<Vec<u8>> {
    matches
        .value_of("JWT_SECRET")
        .map(|s| s.into())
        .or(std::env::var("JWT_SECRET").ok())
        .map(|s| s.into_bytes())
        .ok_or_else(|| {
                        "The --jwt-secret argument must be passed or the JWT_SECRET environment \
                  variable must be set."
                                .into()
                    })
}

struct MyApp {
    inner: BuiAppInner<Shared>,
}

impl MyApp {
    fn new(secret: &[u8], http_server_addr: &str, config: Config) -> Self {
        let shared_store = DataTracker::new(Shared {
                                                is_recording: false,
                                                counter: 0,
                                                name: "".into(),
                                            });

        let chan_size = 10;

        let addr = http_server_addr.parse().unwrap();

        let (_, mut inner) = create_bui_app_inner(&secret, shared_store, &addr, config, chan_size);

        // --- handle callbacks from any connected client

        let tracker_arc2 = inner.shared_arc().clone();
        // Create a Stream to handle callbacks from clients.
        let callback_rx_future = inner
            .add_callback_listener(10) // max number of callbacks to buffer
            .for_each(move |msg| {
            let mut shared = tracker_arc2.lock().unwrap();
            match msg.name.as_ref() {
                "set_is_recording" => {
                    // Take generic json value and convert it to a bool.
                    match serde_json::from_value::<bool>(msg.args) {
                        Ok(bool_value) => {
                            // Update our shared store with the value received.
                            shared.as_tracked_mut().is_recording = bool_value;
                        },
                        Err(e) => {
                            error!("could not cast json value to bool: {:?}", e);
                        },
                    };
                },
                "set_name" => {
                    // Take generic json value and convert it to a String.
                    match serde_json::from_value::<String>(msg.args) {
                        Ok(name) => {
                            // Update our shared store with the value received.
                            shared.as_tracked_mut().name = name;
                        },
                        Err(e) => {
                            error!("could not cast json value to String: {:?}", e);
                        },
                    };
                },
                name => {
                    error!("callback with unknown name: {:?}", name);
                },
            }
            futures::future::ok(())
        });
        inner.hyper_server().handle().spawn(callback_rx_future);

        MyApp { inner: inner }

    }

    fn handle(&self) -> tokio_core::reactor::Handle {
        self.inner.hyper_server().handle()
    }

    fn run(self) -> std::result::Result<(), hyper::Error> {
        self.inner.into_hyper_server().run()
    }
}

fn run() -> Result<()> {
    dotenv::dotenv().ok();
    env_logger::init().unwrap();

    let matches = clap::App::new("CARGO_PKG_NAME")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(clap::Arg::with_name("JWT_SECRET")
                 .long("jwt-secret")
                 .help("Specifies the JWT secret. Falls back to the JWT_SECRET \
                environment variable if unspecified.")
                 .global(true)
                 .takes_value(true))
        .get_matches();
    let secret = jwt_secret(&matches)?;

    let http_server_addr = "127.0.0.1:3410";
    let config = get_default_config();

    let my_app = MyApp::new(&secret, &http_server_addr, config);

    let tracker_arc = my_app.inner.shared_arc().clone();

    let handle = my_app.handle();
    let interval_stream: tokio_core::reactor::Interval =
        tokio_core::reactor::Interval::new(std::time::Duration::from_millis(1000), &handle)
            .unwrap();

    let stream_future = interval_stream
        .for_each(move |_| {
                      let mut shared_store = tracker_arc.lock().unwrap();
                      let mut shared = shared_store.as_tracked_mut();
                      shared.counter += 1;
                      Ok(())
                  })
        .map_err(|e| {
                     error!("interval error {:?}", e);
                     ()
                 });
    my_app.handle().spawn(stream_future);

    println!("Listening on http://{}", http_server_addr);
    my_app.run()?;
    Ok(())
}

quick_main!(run);
