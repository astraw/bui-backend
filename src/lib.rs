//! This library, [bui-backend](https://github.com/astraw/bui-backend), enables
//! an application to serve a [Browser User Interface
//! (BUI)](https://en.wikipedia.org/wiki/Browser_user_interface). The browser
//! becomes your GUI. The API is based on futures and reactively pushes state to
//! the browser. Assets can be served from the filesystem or bundled in the
//! executable. The server provides an "escape hatch" to allow server-client
//! communication outside of bui-backend. [The demo][bui-demo] includes a Rust
//! web assembly (wasm), plain Javascript frontend and an Elm frontend.
//! Together, this lets you ship an application written in Rust as a single file
//! with a browser-based UI.
//!
//! The operating principle is that the server runs an HTTP server (based on
//! [hyper](https://hyper.rs)) to which the browser connects. The initial page
//! tells the browser to open a connection to a [Server Sent
//! Events](https://html.spec.whatwg.org/multipage/server-sent-events.html)
//! endpoint and the server can subsequently push updates to the browser.
//! Additionally, the server listens for POST callbacks on another endpoint. All
//! data is encoded as JSON.
//!
//! #### Features
//!
//!  - Uses [raii-change-tracker](https://crates.io/crates/raii-change-tracker)
//!    to ensure that server state changes are reactively sent to all connected
//!    frontends.
//!  - To keep things simple, server state is shared with all connected clients.
//!  - Session keys (per browser) and connection keys (per tab) are maintained
//!    and allow taking control of communication using pre-established event
//!    stream. (This is an "escape hatch" to break out of the bui-backend
//!    abstractions as required by some use cases.)
//!  - Demo frontends written in Rust web assembly (wasm), Javascript and Elm. (Use
//!    [`bui-demo`][bui-demo] with `frontend_stdweb`, `frontend_js`,
//!    or `frontend_elm` feature.)
//!  - Written in async style using
//!    [futures](https://github.com/rust-lang-nursery/futures-rs).
//!  - Uses [Serde JSON](https://crates.io/crates/serde_json).
//!  - Compile-time choice between bundling served files into executable (with
//!    `bundle_files` feature) or reading files from disk (`serve_files`).
//!
//! #### Security warning
//!
//! Due to its nature, the program listens and responds to client connections
//! from the network. If you expose your program to untrusted network
//! connections, ensure that code within any callback handlers you write is safe
//! when handling malicious input.
//!
//! [bui-demo]: https://github.com/astraw/bui-backend/tree/master/bui-demo
// #![deny(missing_docs)]

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate log;
extern crate change_tracker;
#[cfg(feature = "bundle_files")]
extern crate includedir;
extern crate serde;
extern crate serde_json;
extern crate futures;
extern crate tokio;
extern crate tokio_executor;
extern crate hyper;
extern crate jsonwebtoken;
extern crate uuid;
extern crate failure;
#[macro_use]
extern crate failure_derive;

mod errors;
pub use errors::Error;

pub mod lowlevel;
pub mod highlevel;
