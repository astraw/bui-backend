# Change Log

All user visible changes to this project will be documented in this file.
This project adheres to [Semantic Versioning](http://semver.org/), as described
for Rust libraries in [RFC #1105](https://github.com/rust-lang/rfcs/blob/master/text/1105-api-evolution.md)

## unreleased

### Changed

* Add `frontend_yew` demo based on the yew framework.
* Rename existing rust wasm frontend to `frontend_stdweb`.
* Upgrade `frontend_stdweb` demo to `stdweb` 0.4.
* ConnectionKeyType is now u32 (not usize).
* Remove dependency on `error_chain` in main crate and demo uses
  [`failure`](https://crates.io/crates/failure) crate.
* Update all outdated dependencies.
* Updated documentation to specify more exactly how to build demo rust wasm
  frontend.

## [0.3.0] - 2017-12-31

### Changed

* EventSource messages specify "bui_backend" stream and do not encapsulate
  messages in a JSON message whose outer layer is type `EventStreamMessage`.
  This is a breaking API change as it requires clients to change their message
  parsing. The `EventStreamMessage` type has been removed.
* Updated all example frontends (Rust, JS, Elm) to better handle EventSource
  Web API events and readyState.

## [0.2.1] - 2017-12-28

### Added

* Implmented new Rust wasm (Web Assembly) frontend demo.

### Changed

* Update to jsonwebtoken 3.
* All frontend demos also send name to server on "Enter" keypress.
* Demo backend CLI supports changing host and port.
* Demo backend CLI uses default JWT secret when run on loopback.

### Fixed

* Remove compiler warnings

## [0.2.0] - 2017-09-17

### Changed

* Make event URL path configurable and send events whenever prefix used
* Do not use deprecated futures .boxed() methods and BoxFuture type.
* Update to error-chain 0.11

### Fixed

* Remove compiler warnings

## [0.1.1] - 2017-09-16

### Added

* The demo checks if the browser supports EventSource and shows error if not.

### Fixed

* bui-backend permits file path configuration to be specified as an absolute
  path.

## 0.1.0 - 2017-08-13

* Initial release

[0.3.0]: https://github.com/astraw/bui-backend/compare/bui-backend/0.2.1...bui-backend/0.3.0
[0.2.1]: https://github.com/astraw/bui-backend/compare/bui-backend/0.2.0...bui-backend/0.2.1
[0.2.0]: https://github.com/astraw/bui-backend/compare/bui-backend/0.1.1...bui-backend/0.2.0
[0.1.1]: https://github.com/astraw/bui-backend/compare/bui-backend/0.1.0...bui-backend/0.1.1
