# bui-backend - Brower User Interfaces (BUIs) with Tokio [![Status][status-img]][status-url]

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
 - To keep things simple, server state is shared with all connected clients.
 - Session keys (per browser) and connection keys (per tab) are maintained and
   allow taking control of communication using pre-established event stream.
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

#### Other crates in this repository

- `codegen` - Buildtime codegen support for bui-backend.
- `bui-demo` - Example program with Javascript and Elm frontends.

## License

Licensed under either of

* Apache License, Version 2.0,
  (./LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license (./LICENSE-MIT or http://opensource.org/licenses/MIT)
  at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

## Code of conduct

Anyone who interacts with bui-backend in any space including but not
limited to this GitHub repository is expected to follow our [code of
conduct](https://github.com/astraw/bui-backend/blob/master/code_of_conduct.md).

[status-img]: https://travis-ci.org/astraw/bui-backend.svg?branch=master
[status-url]: https://travis-ci.org/astraw/bui-backend
