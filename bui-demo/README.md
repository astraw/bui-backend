# bui-demo - simple demonstration of bui-backend

This is a self containted demonstration of
[bui-backend](https://github.com/astraw/bui-backend), a Browser User Interface
(BUI) application framework in Rust. The backend server is written in Rust and
possible frontends written in Rust web assembly (wasm), plain Javascript and
Elm. Using the Rust frontend, this is a demo application with frontend and
backend written in Rust that can be shipped as a single file that has a
browser-based UI.

The demo is a mockup for a backend application which can record data at a given
filename and the recording is controlled via the broswer user interface.

![Screenshot][screenshot-img]

## Running

To run with default features `bundle_files` and `frontend_js` (webserver files
are bundled into executable, plain Javascript frontend):

    # from the bui-demo directory
    cargo run -- --jwt-secret=abc123

    # Now point your browser to http://127.0.0.1:3410

To run with other options:

    # Use Rust wasm frontend, all files bundled into executable:
    # The following line requires building the Rust wasm frontend (see below).
    cargo run --no-default-features --features "bundle_files frontend_rust_wasm"

    # or

    # Use Rust wasm frontend, files served from filesystem for frontend development:
    # The following line requires building the Rust wasm frontend (see below).
    cargo run --no-default-features --features "serve_files frontend_elm"

    # or

    # Use JS frontend, files served from filesystem for frontend development:
    cargo run --no-default-features --features "serve_files frontend_js"

    # or

    # Use Elm frontend, files served from filesystem for frontend development:
    # The following line requires building the Elm frontend (see below).
    cargo run --no-default-features --features "serve_files frontend_elm"

    # or

    # Use Elm frontend, all files bundled into executable:
    # The following line requires building the Elm frontend (see below).
    cargo run --no-default-features --features "bundle_files frontend_elm"

## Building the Elm frontend

    cd frontend_elm && make

## Building the Rust wasm frontend

    cd frontend_rust_wasm && ./build.sh

[screenshot-img]: bui-demo.png
