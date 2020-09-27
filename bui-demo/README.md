# bui-demo

## demonstration of bui-backend

This is a self contained demonstration of
[bui-backend](https://github.com/astraw/bui-backend), a Browser User Interface
(BUI) application framework in Rust. The backend server is written in Rust and
possible frontends written in Rust web assembly (wasm) using the
[seed](https://crates.io/crates/seed) framework, and plain Javascript. When
using the Rust frontend, this is a demo application with frontend and backend
written in Rust that can be shipped as a single file that has a browser-based
UI.

The demo is a mockup for a backend application which can record data at a given
filename and the recording is controlled via the browser user interface.

## Running

To run with default features `bundle_files` and `frontend_js` (webserver files
are bundled into executable, plain Javascript frontend):

    # from the bui-demo directory
    cargo run

    # Now point your browser to http://localhost:3410

To run with other options:

    # Use Rust seed frontend, all files bundled into executable:
    # The following line requires building the Rust yew frontend (see below).
    cargo run --no-default-features --features "bundle_files frontend_seed"

    # or

    # Use Rust seed frontend, files served from filesystem for frontend development:
    # The following line requires building the Rust yew frontend (see below).
    cargo run --no-default-features --features "serve_files frontend_seed"

    # or

    # Use JS frontend, files served from filesystem for frontend development:
    cargo run --no-default-features --features "serve_files frontend_js"

## Building the Rust seed frontend

Frontend is built with `wasm-pack`. (In development, `wasm-pack 0.8.11` was
used.) Install from https://rustwasm.github.io/wasm-pack/installer/ .

    cd frontend_seed && ./build.sh

## Building the Rust yew frontend

    cd frontend_yew && ./build.sh

License: MIT/Apache-2.0
