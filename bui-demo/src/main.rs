//! # demonstration of bui-backend
//!
//! This is a self contained demonstration of
//! [bui-backend](https://github.com/astraw/bui-backend), a Browser User Interface
//! (BUI) application framework in Rust. The backend server is written in Rust and
//! possible frontends written in Rust web assembly (wasm) using the
//! [seed](https://crates.io/crates/seed) framework, and plain Javascript. When
//! using the Rust frontend, this is a demo application with frontend and backend
//! written in Rust that can be shipped as a single file that has a browser-based
//! UI.
//!
//! The demo is a mockup for a backend application which can record data at a given
//! filename and the recording is controlled via the browser user interface.
//!
//! # Running
//!
//! To run with default features `bundle_files` and `frontend_js` (webserver files
//! are bundled into executable, plain Javascript frontend):
//!
//!     # from the bui-demo directory
//!     cargo run
//!
//!     # Now point your browser to http://localhost:3410
//!
//! To run with other options:
//!
//!     # Use Rust seed frontend, all files bundled into executable:
//!     # The following line requires building the Rust yew frontend (see below).
//!     cargo run --no-default-features --features "bundle_files frontend_seed"
//!
//!     # or
//!
//!     # Use Rust seed frontend, files served from filesystem for frontend development:
//!     # The following line requires building the Rust yew frontend (see below).
//!     cargo run --no-default-features --features "serve_files frontend_seed"
//!
//!     # or
//!
//!     # Use JS frontend, files served from filesystem for frontend development:
//!     cargo run --no-default-features --features "serve_files frontend_js"
//!
//! # Building the Rust seed frontend
//!
//! Frontend is built with `wasm-pack`. (In development, `wasm-pack 0.8.11` was
//! used.) Install from https://rustwasm.github.io/wasm-pack/installer/ .
//!
//!     cd frontend_seed && ./build.sh
//!

use std::net::ToSocketAddrs;
use std::sync::Arc;

use parking_lot::RwLock;

use async_change_tracker::ChangeTracker;
use bui_backend::highlevel::{create_bui_app_inner, BuiAppInner};
use bui_backend::AccessControl;
use bui_backend_types::CallbackDataAndSession;

use bui_demo_data::{Callback, Shared};

#[derive(Debug)]
struct Error {
    kind: ErrorKind,
}

#[derive(Debug)]
enum ErrorKind {
    BuiBackend(bui_backend::Error),
    Raw(String),
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self { kind }
    }
}

impl From<bui_backend::Error> for Error {
    fn from(orig: bui_backend::Error) -> Self {
        let kind = ErrorKind::BuiBackend(orig);
        Self { kind }
    }
}

// Include the files to be served and define `fn get_default_config()`.
include!(concat!(env!("OUT_DIR"), "/public.rs")); // Despite slash, this works on Windows.

/// The structure that holds our app data
struct MyApp {
    inner: BuiAppInner<Shared, Callback>,
}

fn address(matches: &clap::ArgMatches) -> std::net::SocketAddr {
    let address = matches.value_of("address").unwrap();
    address.to_socket_addrs().unwrap().next().unwrap()
}

fn is_loopback(addr_any: &std::net::SocketAddr) -> bool {
    match addr_any {
        &std::net::SocketAddr::V4(addr) => addr.ip().is_loopback(),
        &std::net::SocketAddr::V6(addr) => addr.ip().is_loopback(),
    }
}

/// Parse the JWT secret from command-line args or environment variables.
fn jwt_secret(matches: &clap::ArgMatches, required: bool) -> Result<Vec<u8>, Error> {
    match matches
        .value_of("JWT_SECRET")
        .map(|s| s.into())
        .or(std::env::var("JWT_SECRET").ok())
        .map(|s| s.into_bytes())
    {
        Some(secret) => Ok(secret),
        None => {
            if required {
                Err(ErrorKind::Raw(format!(
                    "The --jwt-secret argument must be passed or the JWT_SECRET environment \
                variable must be set when not using loopback interface."
                ))
                .into())
            } else {
                // insecure secret when using loopback interface
                Ok(b"jwt_secret".to_vec())
            }
        }
    }
}

impl MyApp {
    /// Create our app
    async fn new(auth: AccessControl, config: Config) -> Result<Self, Error> {
        // fn new(auth: AccessControl, config: Config) -> Result<Self, Error> {

        // Create our shared state.
        let shared_store = Arc::new(RwLock::new(ChangeTracker::new(Shared {
            is_recording: false,
            counter: 0,
            name: "".into(),
        })));

        let chan_size = 10;
        let (rx_conn, bui_server) =
            bui_backend::lowlevel::launcher(config, &auth, chan_size, "/events", None);

        let handle = tokio::runtime::Handle::current();

        // Create `inner`, which takes care of the browser communication details for us.
        let (_, mut inner) = create_bui_app_inner(
            handle,
            None,
            &auth,
            shared_store,
            Some("bui_backend".to_string()),
            rx_conn,
            bui_server,
        )
        .await?;

        // Make a clone of our shared state Arc which will be moved into our callback handler.
        let tracker_arc2 = inner.shared_arc().clone();

        // Create a Stream to handle callbacks from clients.
        inner.set_callback_listener(Box::new(move |msg: CallbackDataAndSession<Callback>| {
            // This closure is the callback handler called whenever the
            // client browser sends us something.

            // Get access to our shared state so we can modify it based on
            // the browser's callback.
            let mut shared = tracker_arc2.write();

            match msg.payload {
                Callback::SetIsRecording(bool_value) => {
                    // Update our shared store with the value received.
                    shared.modify(|shared| shared.is_recording = bool_value);
                }
                Callback::SetName(name) => {
                    // Update our shared store with the value received.
                    shared.modify(|shared| shared.name = name);
                }
            }
            futures::future::ok(())
        }));

        // Return our app.
        Ok(MyApp { inner })
    }
}

fn display_qr_url(url: &str) {
    use qrcodegen::{QrCode, QrCodeEcc};
    use std::io::{stdout, Write};

    let qr = QrCode::encode_text(&url, QrCodeEcc::Low).unwrap();

    let stdout = stdout();
    let mut stdout_handle = stdout.lock();
    writeln!(stdout_handle).unwrap();
    for y in 0..qr.size() {
        write!(stdout_handle, " ").unwrap();
        for x in 0..qr.size() {
            write!(
                stdout_handle,
                "{}",
                if qr.get_module(x, y) { "██" } else { "  " }
            )
            .unwrap();
        }
        writeln!(stdout_handle).unwrap();
    }
    writeln!(stdout_handle).unwrap();
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Set environment variables from `.env` file, if it exists.
    dotenv::dotenv().ok();

    // Setup logging based on level in RUST_LOG environment variable.
    env_logger::init();

    // Parse our command-line arguments.
    let matches = clap::App::new("CARGO_PKG_NAME")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(
            clap::Arg::new("JWT_SECRET")
                .long("jwt-secret")
                .help(
                    "Specifies the JWT secret. Falls back to the JWT_SECRET \
                environment variable if unspecified.",
                )
                .global(true)
                .takes_value(true),
        )
        .arg(
            clap::Arg::new("address")
                .long("address")
                .help("Bind the server to this address")
                .default_value("localhost:3410")
                .value_name("ADDRESS")
                .takes_value(true),
        )
        .get_matches();

    let http_server_addr = address(&matches);

    // Get our JWT secret.
    let required = !is_loopback(&http_server_addr);
    let secret = jwt_secret(&matches, required)?;

    // This `get_default_config()` function is created by bui_backend_codegen
    // and is pulled in here by the `include!` macro above.
    let config = get_default_config();

    let auth = if http_server_addr.ip().is_loopback() {
        AccessControl::Insecure(http_server_addr)
    } else {
        bui_backend::highlevel::generate_random_auth(http_server_addr, secret)?
    };

    // // Create our app.

    let my_app = MyApp::new(auth, config).await?;

    // Clone our shared data to move it into a closure later.
    let tracker_arc = my_app.inner.shared_arc().clone();

    // Create a stream to call our closure every second.
    let mut interval_stream = tokio::time::interval(std::time::Duration::from_millis(1000));

    let stream_future = async move {
        loop {
            // This is the main loop of the app. Here we do nothing except
            // update a counter periodically.

            // Wait for the next update time to arrive ...
            interval_stream.tick().await;

            // ... and modify our counter.
            let mut shared_store = tracker_arc.write();
            shared_store.modify(|shared| {
                shared.counter += 1;
            });
        }
    };

    let maybe_url = my_app.inner.guess_url_with_token();
    println!(
        "Depending on IP address resolution, you may be able to login \
        with this url: {}",
        maybe_url
    );
    println!("This same URL as a QR code:");
    display_qr_url(&maybe_url);

    // Run our app.
    stream_future.await;

    Ok(())
}
