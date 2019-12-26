# bui-backend

bui-backend - Brower User Interfaces (BUIs) with Tokio

[![Version][version-img]][version-url] [![Status][status-img]][status-url]
[![Doc][doc-img]][doc-url]


[status-img]: https://travis-ci.org/astraw/bui-backend.svg?branch=master
[status-url]: https://travis-ci.org/astraw/bui-backend
[bui-demo]: https://github.com/astraw/bui-backend/tree/master/bui-demo
[doc-img]: https://docs.rs/bui-backend/badge.svg
[doc-url]: https://docs.rs/bui-backend/
[version-img]: https://img.shields.io/crates/v/bui-backend.svg
[version-url]: https://crates.io/crates/bui-backend

This library enables an application to serve a [Browser User Interface
(BUI)](https://en.wikipedia.org/wiki/Browser_user_interface). The browser
becomes your GUI. The API is based on futures and reactively pushes state to
the browser. Assets can be served from the filesystem or bundled in the
executable. The server provides an "escape hatch" to allow server-client
communication outside of bui-backend. [The demo][bui-demo] includes a Rust
web assembly (wasm) frontend using the yew framework, a Rust web assembly
(wasm) frontend using stdweb, and a plain Javascript frontend. Together,
this lets you ship an application written in Rust as a single file with a
browser-based UI.

The operating principle is that the server runs an HTTP server (based on
[hyper](https://hyper.rs)) to which the browser connects. The initial page
tells the browser to open a connection to a [Server Sent
Events](https://html.spec.whatwg.org/multipage/server-sent-events.html)
endpoint and the server can subsequently push updates to the browser.
Additionally, the server listens for POST callbacks on another endpoint. All
data is encoded as JSON.

##### Features

 - Uses the [`async-change-tracker::ChangeTracker`](../async_change_tracker/struct.ChangeTracker.html)
   type to ensure that server state changes are reactively sent to all
   connected frontends.
 - To keep things simple, server state is shared with all connected clients.
 - Session keys (per browser) and connection keys (per tab) are maintained
   and allow taking control of communication using pre-established event
   stream. (This is an "escape hatch" to break out of the bui-backend
   abstractions as required by some use cases.)
 - Written in asyncronous rust using async/await.
 - Uses [Serde JSON](https://crates.io/crates/serde_json).
 - Compile-time choice between bundling served files into executable (with
   `bundle_files` feature) or reading files from disk (`serve_files`).

##### Demo

 A demo is available with frontends written in Rust web assembly (plain wasm
 or yew framework) and Javascript. (Use [`bui-demo`][bui-demo] with
 `frontend_stdweb`, `frontend_yew`, or `frontend_js` feature.)

##### Potential improvements

 - Add example with user login.
 - Send minimal differences when state changes, likely by improving
   [`async-change-tracker`](../async_change_tracker/index.html).
 - Implement more sophisticated state-sharing allowing partial views and
   minimal updates.
 - Use
   [`ReadableStream`](https://developer.mozilla.org/en-US/docs/Web/API/ReadableStream)
   instead of [`Server Sent
   Events`](https://caniuse.com/#search=EventSource).
 - Add a websocket transport option as an alternative to Server Sent Events.
 - Your idea here.

##### Security warning

Due to its nature, the program listens and responds to client connections
from the network. If you expose your program to untrusted network
connections, ensure that code within any callback handlers you write is safe
when handling malicious input.

##### Other crates in this repository

- `codegen` - Buildtime codegen support for bui-backend.
- `bui-demo` - Example program with Rust and Javascript frontends.

### License

Licensed under either of

* Apache License, Version 2.0, (./LICENSE-APACHE or
  http://www.apache.org/licenses/LICENSE-2.0)
* MIT license (./LICENSE-MIT or http://opensource.org/licenses/MIT) at your
  option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

### Code of conduct

Anyone who interacts with bui-backend in any space including but not limited
to this GitHub repository is expected to follow our [code of
conduct](https://github.com/astraw/bui-backend/blob/master/code_of_conduct.md).

### Operational overview

```
                      HTTP responses
                      & event stream
+--server----------+ +--------------> +--web-browser-------------------------+
|app binary written|                  | frontend, optionally in written      |
|with bui_backend  | <--------------+ | in rust with support from bui_backend|
+------------------+  HTTP requests   +--------------------------------------+
```
<!-- ASCII art drawn with http://asciiflow.com/ -->

### Example

For a full example, see [the
demo](https://github.com/astraw/bui-backend/tree/master/bui-demo).

### Usage

This example assumes you have the following filesystem layout in the crate
for the application binary that will run the webserver:

```
.
├── build.rs        # Bundles frontend files or specifies serving from disk.
├── Cargo.toml      # Normal Cargo.toml manifest file.
├── frontend_js     # Your frontend files are in this directory. bui_backend
│   ├── index.html  #   also includes some assistance for writing frontends
│   └── js          #   in rust, such as automatic serialization.
│       └── main.js
└── src             # The source for your application binary is here.
    └── main.rs
```

In this example, we assume you have files to serve for a frontend (e.g.
`index.html`) in the directory `frontend_js`. You must create a file
`build.rs` which will:
 * compile the files in this directory into your application's binary if you
   use the default compilation features or specified the `bundle_files`
   cargo feature (recommended for deployment),
 * attempt to access the files in this directory at runtime if you use the
   `serve_files` cargo feature (recommended for frontend development),
 * or throw a compile time error if you do not specify exactly one of
   `bundle_files` and `serve_files`.

In the `Cargo.toml` file for your backend application, add the following
lines:
```toml
[dependencies]
bui-backend = "0.7"
bui-backend-types = "0.7"

[build-dependencies]
bui-backend-codegen = "0.1.4"
```

Now, here is the example `build.rs` file:
```rust
extern crate bui_backend_codegen;

fn main() {
    bui_backend_codegen::codegen("frontend_js", "public.rs").expect("codegen failed");
}
```

Finally, in your `main.rs` file:
```rust
// Include the files to be served and define `fn get_default_config()`.
include!(concat!(env!("OUT_DIR"), "/public.rs")); // Despite slash, this works on Windows.
```

### Building the documentation

```
RUSTDOCFLAGS='--cfg=docsrs -Dwarnings' cargo +nightly doc --open --features "bui-backend-types/uuid-v4"
```

### Testing

```
cargo +nightly test --features "bui-backend-types/uuid-v4"
```

### Regnerate README.md

```
cargo readme > README.md
```

License: MIT/Apache-2.0
