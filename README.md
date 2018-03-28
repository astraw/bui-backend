# bui-backend - Brower User Interfaces (BUIs) with Tokio [![Version][version-img]][version-url] [![Status][status-img]][status-url] [![Doc][doc-img]][doc-url]

**This is a work in progress. The API will likely evolve somewhat. The docs
need work.**

This library, [bui-backend](https://github.com/astraw/bui-backend), enables an
application to serve a [Browser User Interface
(BUI)](https://en.wikipedia.org/wiki/Browser_user_interface). The browser
becomes your GUI. The API is based on futures and reactively pushes state to the
browser. Assets can be served from the filesystem or bundled in the executable.
The server provides an "escape hatch" to allow server-client communication
outside of bui-backend. [The demo][bui-demo] includes a Rust web assembly (wasm)
frontend using the yew framework, a Rust web assembly (wasm) frontend using
stdweb, plain Javascript frontend and an Elm frontend. Together, this lets you
ship an application written in Rust as a single file with a browser-based UI.

The operating principle is that the server runs an HTTP server (based on
[hyper](https://hyper.rs)) to which the browser connects. The initial page tells
the browser to open a connection to a [Server Sent
Events](https://html.spec.whatwg.org/multipage/server-sent-events.html) endpoint
and the server can subsequently push updates to the browser. Additionally, the
server listens for POST callbacks on another endpoint. All data is encoded as
JSON.

#### Features

 - Uses [raii-change-tracker](https://crates.io/crates/raii-change-tracker) to
   ensure that server state changes are reactively sent to all connected
   frontends.
 - To keep things simple, server state is shared with all connected clients.
 - Session keys (per browser) and connection keys (per tab) are maintained and
   allow taking control of communication using pre-established event stream.
   (This is an "escape hatch" to break out of the bui-backend abstractions as
   required by some use cases.)
 - Written in async style using
   [futures](https://github.com/rust-lang-nursery/futures-rs).
 - Uses [Serde JSON](https://crates.io/crates/serde_json).
 - Compile-time choice between bundling served files into executable (with
   `bundle_files` feature) or reading files from disk (`serve_files`).

#### Demo

 A demo is available with frontends written in Rust web assembly (plain wasm or
 yew framework), Javascript and Elm. (Use [`bui-demo`][bui-demo] with
 `frontend_stdweb`, `frontend_yew`, `frontend_js`, or `frontend_elm`
 feature.)

#### Potential improvements

 - Add example with user login.
 - Implement more sophisticated state-sharing allowing partial views and
   minimal updates.
 - When [`ReadableStream`](https://caniuse.com/#search=ReadableStream) is more
   widely supported, use it (instead of [`Server Sent
   Events`](https://caniuse.com/#search=EventSource)).
 - Your idea here.

#### Security warning

Due to its nature, the program listens and responds to client connections from
the network. If you expose your program to untrusted network connections, ensure
that code within any callback handlers you write is safe when handling malicious
input.

#### Other crates in this repository

- `codegen` - Buildtime codegen support for bui-backend.
- `bui-demo` - Example program with Rust, Javascript and Elm frontends.

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
[bui-demo]: https://github.com/astraw/bui-backend/tree/master/bui-demo
[doc-img]: https://docs.rs/bui-backend/badge.svg
[doc-url]: https://docs.rs/bui-backend/
[version-img]: https://img.shields.io/crates/v/bui-backend.svg
[version-url]: https://crates.io/crates/bui-backend
