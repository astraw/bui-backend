#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate raii_change_tracker;
extern crate bui_backend;
#[cfg(feature = "bundle_files")]
extern crate includedir;
#[cfg(feature = "bundle_files")]
extern crate phf;
extern crate serde_json;
extern crate clap;
extern crate hyper;
extern crate dotenv;
extern crate futures;
extern crate tokio_core;
extern crate bui_demo_data;

use failure::Error;

use std::net::ToSocketAddrs;

use raii_change_tracker::DataTracker;
use bui_backend::highlevel::{BuiAppInner, create_bui_app_inner};

use futures::{Future, Stream};
use bui_demo_data::Shared;

// Include the files to be served and define `fn get_default_config()`.
include!(concat!(env!("OUT_DIR"), "/public.rs")); // Despite slash, this does work on Windows.

/// The structure that holds our app data
struct MyApp {
    inner: BuiAppInner<Shared>,
}

fn address( matches: &clap::ArgMatches ) -> std::net::SocketAddr {
    let host = matches.value_of( "host" ).unwrap();
    let port = matches.value_of( "port" ).unwrap();
    format!( "{}:{}", host, port ).to_socket_addrs().unwrap().next().unwrap()
}

fn is_loopback(addr_any: &std::net::SocketAddr) -> bool {
    match addr_any {
        &std::net::SocketAddr::V4(addr) => addr.ip().is_loopback(),
        &std::net::SocketAddr::V6(addr) => addr.ip().is_loopback(),
    }
}

/// Parse the JWT secret from command-line args or environment variables.
fn jwt_secret(matches: &clap::ArgMatches, required: bool) -> Result<Vec<u8>,Error> {
    match matches
        .value_of("JWT_SECRET")
        .map(|s| s.into())
        .or(std::env::var("JWT_SECRET").ok())
        .map(|s| s.into_bytes())
    {
        Some(secret) => Ok(secret),
        None => {
            if required {
                Err(format_err!("The --jwt-secret argument must be passed or the JWT_SECRET environment \
                variable must be set when not using loopback interface."))
            } else {
                Ok(b"jwt_secret".to_vec())
            }
        }
    }
}

impl MyApp {
    /// Create our app
    fn new(secret: &[u8], addr: &std::net::SocketAddr, config: Config) -> Self {

        // Create our shared state.
        let shared_store = DataTracker::new(Shared {
                                                is_recording: false,
                                                counter: 0,
                                                name: "".into(),
                                            });

        // Create `inner`, which takes care of the browser communication details for us.
        let chan_size = 10;
        let (_, mut inner) =
            create_bui_app_inner(&secret, shared_store, &addr, config, chan_size, "/events");

        // Make a clone of our shared state Arc which will be moved into our callback handler.
        let tracker_arc2 = inner.shared_arc().clone();

        // Create a Stream to handle callbacks from clients.
        let callback_rx_future = inner
            .add_callback_listener(10) // max number of callbacks to buffer
            .for_each(move |msg| {

                // This closure is the callback handler called whenever the
                // client browser sends us something.

                // Get access to our shared state so we can modify it based on
                // the browser's callback.
                let mut shared = tracker_arc2.lock().unwrap();

                // All callbacks have the `name` field.
                match msg.name.as_ref() {
                    "set_is_recording" => {
                        // All callbacks also have the `args` field. Here, take
                        // generic json value and convert it to a bool.
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
                        // Take the generic `args` and convert it to a String.
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
                        // This is an error case. Log it. (And do not take down the server.)
                        error!("callback with unknown name: {:?}", name);
                    },
                }
                futures::future::ok(())
            });

        // Add our future into the event loop created by hyper.
        inner.hyper_server().handle().spawn(callback_rx_future);

        // Return our app.
        MyApp { inner: inner }
    }

    /// Get a handle to our event loop.
    fn handle(&self) -> tokio_core::reactor::Handle {
        self.inner.hyper_server().handle()
    }

    /// Consume self and run forever.
    fn run(self) -> std::result::Result<(), hyper::Error> {
        self.inner.into_hyper_server().run()
    }
}

fn run() -> Result<(),Error> {

    // Set environment variables from `.env` file, if it exists.
    dotenv::dotenv().ok();

    // Setup logging based on level in RUST_LOG environment variable.
    env_logger::init();

    // Parse our command-line arguments.
    let matches = clap::App::new("CARGO_PKG_NAME")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(clap::Arg::with_name("JWT_SECRET")
                 .long("jwt-secret")
                 .help("Specifies the JWT secret. Falls back to the JWT_SECRET \
                environment variable if unspecified.")
                 .global(true)
                 .takes_value(true))
        .arg(clap::Arg::with_name( "host" )
                .long( "host" )
                .help( "Bind the server to this address")
                .default_value("localhost")
                .value_name( "HOST" )
                .takes_value( true ))
        .arg(clap::Arg::with_name( "port" )
                .long( "port" )
                .help( "Bind the server to this port, default 3410" )
                .default_value("3410")
                .value_name( "PORT" )
                .takes_value( true )
        )
        .get_matches();

    let http_server_addr = address(&matches);

    // Get our JWT secret.
    let required = !is_loopback(&http_server_addr);
    let secret = jwt_secret(&matches, required)?;

    // This `get_default_config()` function is created by bui_backend_codegen
    // and is pulled in here by the `include!` macro above.
    let config = get_default_config();

    // Create our app.
    let my_app = MyApp::new(&secret, &http_server_addr, config);

    // Clone our shared data to move it into a closure later.
    let tracker_arc = my_app.inner.shared_arc().clone();

    // Get a handle to our event loop.
    let handle = my_app.handle();

    // Create a stream to call our closure every second.
    let interval_stream: tokio_core::reactor::Interval =
        tokio_core::reactor::Interval::new(std::time::Duration::from_millis(1000), &handle)
            .unwrap();

    let stream_future = interval_stream
        .for_each(move |_| {
                      // This closure is called once a second. Update a counter
                      // in our shared data store.
                      let mut shared_store = tracker_arc.lock().unwrap();
                      let mut shared = shared_store.as_tracked_mut();
                      shared.counter += 1;
                      Ok(())
                  })
        .map_err(|e| {
                     error!("interval error {:?}", e);
                     ()
                 });

    // Put our stream into our event loop.
    my_app.handle().spawn(stream_future);

    println!("Listening on http://{}", http_server_addr);

    // Run our app.
    my_app.run()?;
    Ok(())
}

fn main() {
    match run() {
        Ok(()) => {},
        Err(e) => {
            error!("{}, {}", e.cause(), e.backtrace());
            std::process::exit(1);
        }
    }
}
