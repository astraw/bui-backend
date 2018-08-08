#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate parking_lot;
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
extern crate tokio;
extern crate tokio_timer;
extern crate tokio_executor;
extern crate bui_demo_data;

use failure::Error;

use std::net::ToSocketAddrs;
use std::sync::Arc;

use parking_lot::RwLock;
use tokio_executor::Executor;

use bui_backend::change_tracker::ChangeTracker;
use bui_backend::highlevel::{BuiAppInner, create_bui_app_inner};
use bui_backend::lowlevel::CallbackDataAndSession;

use futures::{Future, Stream};
use bui_demo_data::{Shared, Callback};

// Include the files to be served and define `fn get_default_config()`.
include!(concat!(env!("OUT_DIR"), "/public.rs")); // Despite slash, this does work on Windows.

/// The structure that holds our app data
struct MyApp {
    inner: BuiAppInner<Shared, Callback>,
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
    fn new(executor: &mut Executor, secret: &[u8], addr: &std::net::SocketAddr, config: Config) -> Result<Self, Error> {

        // Create our shared state.
        let shared_store = Arc::new(RwLock::new(ChangeTracker::new(Shared {
                                                is_recording: false,
                                                counter: 0,
                                                name: "".into(),
                                            })));

        // Create `inner`, which takes care of the browser communication details for us.
        let chan_size = 10;
        let (_, mut inner) = create_bui_app_inner(executor, Some(secret),
            shared_store, &addr, config, chan_size, "/events")?;

        // Make a clone of our shared state Arc which will be moved into our callback handler.
        let tracker_arc2 = inner.shared_arc().clone();

        // Create a Stream to handle callbacks from clients.
        let callback_rx_future = inner
            .add_callback_listener(10) // max number of callbacks to buffer
            .for_each(move |msg: CallbackDataAndSession<Callback>| {

                // This closure is the callback handler called whenever the
                // client browser sends us something.

                // Get access to our shared state so we can modify it based on
                // the browser's callback.
                let mut shared = tracker_arc2.write();

                match msg.payload {
                    Callback::SetIsRecording(bool_value) => {
                        // Update our shared store with the value received.
                        shared.modify(|shared| shared.is_recording = bool_value);
                    },
                    Callback::SetName(name) => {
                        // Update our shared store with the value received.
                        shared.modify(|shared| shared.name = name);
                    },
                }
                futures::future::ok(())
            });

        // Add our future into the event loop created by hyper.
        executor.spawn(Box::new(callback_rx_future)).map_err(|e| {
            failure::err_msg(format!("spawn error: {:?}", e))
        })?;

        // Return our app.
        Ok(MyApp { inner: inner })
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

    let mut runtime = tokio::runtime::Runtime::new().expect("runtime");

    // Create our app.
    let mut exec = runtime.executor();
    let my_app = MyApp::new(&mut exec, &secret, &http_server_addr,
        config)?;

    // Clone our shared data to move it into a closure later.
    let tracker_arc = my_app.inner.shared_arc().clone();

    // Create a stream to call our closure every second.
    let interval_stream = tokio_timer::Interval::new(
        std::time::Instant::now(), std::time::Duration::from_millis(1000));

    let stream_future = interval_stream
        .for_each(move |_| {
                    // This closure is called once a second. Update a counter
                    // in our shared data store.
                    let mut shared_store = tracker_arc.write();
                    shared_store.modify(|shared| {
                        shared.counter += 1;
                    });
                    Ok(())
                  })
        .map_err(|e| {
                     error!("interval error {:?}", e);
                     ()
                 });

    println!("Listening on http://{}", http_server_addr);

    // Run our app.
    runtime.block_on(stream_future).unwrap();

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
