# bui-demo - simple demonstration of bui-backend

This is a self containted demonstration of
[bui-backend](https://github.com/astraw/bui-backend). The application is written
in Rust and two possible frontends as written in plain Javascript and Elm.

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

    cargo run --no-default-features --features "serve_files frontend_js" -- --jwt-secret=abc123

    # or

    # The following line requires building the Elm frontend (see below).
    cargo run --no-default-features --features "serve_files frontend_elm" -- --jwt-secret=abc123

    # or

    # The following line requires building the Elm frontend (see below).
    cargo run --no-default-features --features "bundle_files frontend_elm" -- --jwt-secret=abc123

## Building the Elm frontend

    cd frontend_elm && make

[screenshot-img]: bui-demo.png
