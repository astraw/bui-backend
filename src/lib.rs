//! Create a Browser User Interface (BUI)
//!
//!
//!
//! ```notrust
//!                       HTTP responses
//!                       & event stream
//! +--server----------+ +--------------> +--web-browser-------------------------+
//! |app binary written|                  | frontend, optionally in written      |
//! |with bui_backend  | <--------------+ | in rust with support from bui_backend|
//! +------------------+  HTTP requests   +--------------------------------------+
//! ```
// # ASCII art above drawn with http://asciiflow.com/
//!
//!
//!
//! ## Example
//!
//! For a full example, see [the
//! demo](https://github.com/astraw/bui-backend/tree/master/bui-demo).
//!
//! ## Usage
//!
//! This example assumes you have the following filesystem layout in the crate
//! for the application binary that will run the webserver:
//!
//! ```notrust
//! .
//! ├── build.rs        # Bundles frontend files or specifies serving from disk.
//! ├── Cargo.toml      # Normal Cargo.toml manifest file.
//! ├── frontend_js     # Your frontend files are in this directory. bui_backend
//! │   ├── index.html  #   also includes some assistance for writing frontends
//! │   └── js          #   in rust, such as automatic serialization.
//! │       └── main.js
//! └── src             # The source for your application binary is here.
//!     └── main.rs
//! ```
//!
//! In this example, we assume you have files to serve for a frontend (e.g.
//! `index.html`) in the directory `frontend_js`. You must create a file
//! `build.rs` which will:
//!  * compile the files in this directory into your application's binary if you
//!    use the default compilation features or specified the `bundle_files`
//!    cargo feature (recommended for deployment),
//!  * attempt to access the files in this directory at runtime if you use the
//!    `serve_files` cargo feature (recommended for frontend development),
//!  * or throw a compile time error if you do not specify exactly one of
//!    `bundle_files` and `serve_files`.
//!
//! In the `Cargo.toml` file for your backend application, add the following
//! lines:
//! ```toml
//! [dependencies]
//! bui-backend = "0.7"
//! bui-backend-types = "0.7"
//!
//! [build-dependencies]
//! bui-backend-codegen = "0.1.4"
//! ```
//!
//! Now, here is the example `build.rs` file:
//! ```rust
//! extern crate bui_backend_codegen;
//!
//! fn main() {
//!     bui_backend_codegen::codegen("frontend_js", "public.rs").expect("codegen failed");
//! }
//! ```
//!
//! Finally, in your `main.rs` file:
//! ```rust
//! // Include the files to be served and define `fn get_default_config()`.
//! include!(concat!(env!("OUT_DIR"), "/public.rs")); // Despite slash, this works on Windows.
//! ```


#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate log;
#[cfg(feature = "bundle_files")]
extern crate includedir;
extern crate serde;
extern crate serde_json;
extern crate futures;
extern crate tokio_executor;
extern crate http;
extern crate hyper;
extern crate jsonwebtoken;
extern crate uuid;
extern crate failure;
extern crate parking_lot;
extern crate chrono;
#[macro_use]
extern crate failure_derive;

extern crate bui_backend_types;

mod errors;
pub use errors::Error;

pub mod change_tracker;
pub mod lowlevel;
pub mod highlevel;
