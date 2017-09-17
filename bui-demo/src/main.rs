#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate error_chain;

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

use raii_change_tracker::DataTracker;
use bui_backend::errors::Result;
use bui_backend::highlevel::{BuiAppInner, create_bui_app_inner};

use futures::{Future, Stream};

include!(concat!(env!("OUT_DIR"), "/public.rs")); // Despite slash, this does work on Windows.

/// The state that is automatically updated in the browser whenever it changes on the server.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
struct Shared {
    is_recording: bool,
    counter: usize,
    name: String,
}

/// Parse the JWT secret from command-line args or environment variables.
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

/// The structure that holds our app data
struct MyApp {
    inner: BuiAppInner<Shared>,
}

impl MyApp {
    /// Create our app
    fn new(secret: &[u8], http_server_addr: &str, config: Config) -> Self {

        // Create our shared state.
        let shared_store = DataTracker::new(Shared {
                                                is_recording: false,
                                                counter: 0,
                                                name: "".into(),
                                            });

        // Create `inner`, which takes care of the browser communication details for us.
        let chan_size = 10;
        let addr = http_server_addr.parse().unwrap();
        let (_, mut inner) =
            create_bui_app_inner(&secret, shared_store, &addr, config, chan_size, "/events");

        // Make a clone of our shared state which will be moved into our callback handler.
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

    /// Geat a handle to our event loop.
    fn handle(&self) -> tokio_core::reactor::Handle {
        self.inner.hyper_server().handle()
    }

    /// Consume self and run forever.
    fn run(self) -> std::result::Result<(), hyper::Error> {
        self.inner.into_hyper_server().run()
    }
}

fn run() -> Result<()> {

    // Set environment variables from `.env` file, if it exists.
    dotenv::dotenv().ok();

    // Setup logging based on level in RUST_LOG environment variable.
    env_logger::init().unwrap();

    // Parse our command-line arguments.
    let matches = clap::App::new("CARGO_PKG_NAME")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(clap::Arg::with_name("JWT_SECRET")
                 .long("jwt-secret")
                 .help("Specifies the JWT secret. Falls back to the JWT_SECRET \
                environment variable if unspecified.")
                 .global(true)
                 .takes_value(true))
        .get_matches();

    // Get our JWT secret.
    let secret = jwt_secret(&matches)?;

    let http_server_addr = "127.0.0.1:3410";

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

quick_main!(run);
