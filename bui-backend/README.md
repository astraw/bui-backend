# bui-backend - Brower User Interfaces (BUIs) with Tokio

**This is a work in progress. The API will likely evolve somewhat. The docs
need work.**

This crate implements support for writing a backend for a [Browser User Interface
(BUI)](https://en.wikipedia.org/wiki/Browser_user_interface) in Rust.

The operating principle is that the server runs an HTTP server (based on
[hyper](https://hyper.rs)) to which the browser connects. The initial page tells
the browser to open a connection to a Server Sent Events endpoint and the server
can subsequently push updates to the browser. Additionally, the server listens
for callbacks POSTED to a different endpoint. All data is encoded as JSON.

#### Features

 - Uses [raii-change-tracker](https://crates.io/crates/raii-change-tracker) to
   ensure that server state changes are reactively sent to all connected
   frontends.
 - Demo frontends written in Javascript and Elm. (Use `bui-demo` with
   `frontend_js` or `frontend_elm` feature.)
 - Written in async style using
   [futures-rs](https://github.com/alexcrichton/futures-rs).
 - Uses [Serde JSON](https://crates.io/crates/serde_json).
 - Compile-time choice between bundling served files into executable (with
   `bundle_files` feature) or reading files from disk (`serve_files`).

#### Potential improvements

 - Add example with
   [rust-webplatform](https://github.com/rust-webplatform/rust-webplatform)
   frontend.
 - Add example with [domafic](https://github.com/cramertj/domafic-rs) frontend.
 - Add example with user login.

#### Security warning

Due to its nature, the program listens and responds to client connections from
the network. If you expose your program to untrusted network connections, ensure
that code within any callback handlers you write is safe when handling malicious
input.
