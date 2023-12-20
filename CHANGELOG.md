# Change Log

All user visible changes to this project will be documented in this file.
This project adheres to [Semantic Versioning](http://semver.org/), as described
for Rust libraries in [RFC #1105](https://github.com/rust-lang/rfcs/blob/master/text/1105-api-evolution.md)

## [0.15.0] - 2023-12-20

### Changed

* Update to `hyper` 1.0.

## [0.14.0] - 2022-04-24

### Changed

* Callbacks are handled by implementing the CallbackHandler trait rather than a
  closure.
* Update dependencies.

## [0.13.0] - 2022-01-31

### Changed

* Update some dependencies in mostly-insignificant but backwards incompatible
  ways requiring a breaking version bump.

## [0.12.0] - 2021-09-06

### Changed

* Requires explicit tokio runtime handle

* Requires two step construction process to start BUI server.

### Added

* Enable custom HTTP request handlers

## [0.11.0] - 2020-12-25

### Changed

* Update to `tokio` 1.0 and `hyper` 0.14.

## [0.10.0] - 2020-10-27

### Changed

* Update to `tokio` 0.3.

## [0.9.0] - 2020-10-04

### Changed

* Update bui-backend-codegen crate to 0.9.0.
* Update dependencies (parking_lot,includedir,stream-cancel,jsonwebtoken,cookie).

### Added

* Restore yew demo

## [0.8.0] - 2019-12-26

### Changed

* Require rust 1.39 and use async/await and rust 2018 edition.
* Update to `tokio` 0.2.
* Update to `hyper` 0.13.
* Use the `async-change-tracker` crate instead of `raii-change-tracker`. The new
  `ChangeTracker` type allows changing the owned value using closures and
  notifies listeners just after the closure completes.
* `walkdir`, `includedir`, and `includedir_codegen` crates only used when the
  `bundle_files` feature is used.
* Drop elm, yew and stdweb frontends in `bui-demo`. Add seed frontend.
* Simplify wire format for callback data to contain only a JSON payload.
* Automatically serialize/deserialize wire data frontend within bui-backend.
  Previously, this had to be done in client code. This is automatic in the
  backend and can also be done automatically in rust frontends if the
  `bui-backend-types` crate is used, as shown in the demo.

### Added

* Make compile-time error more readable when compiling codegen
  crate without required feature flag.

(There was no 0.7 release. This version number was used for internal testing.)

## [0.6.0] - 2018-04-19

### Changed

* create a new `Error` type which implements `failure::Fail` trait
  and replace a panic-on-error with returning `Result<_,Error>`.

## [0.5.0] - 2018-04-12

### Changed

* change api in `highlevel::create_bui_app_inner()` to accept
  `Arc<Mutex<DataTracker<T>>>`. This allows creating the shared
  data store in a different thread than the thread running the
  BUI backend.

## [0.4.1] - 2018-04-03

### Fixed

* `bui-backend-codegen` (v 0.1.1) works correctly on Windows.

## [0.4.0] - 2018-03-28

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

[0.15.0]: https://github.com/astraw/bui-backend/compare/bui-backend/0.14.0...bui-backend/0.15.0
[0.14.0]: https://github.com/astraw/bui-backend/compare/bui-backend/0.13.0...bui-backend/0.14.0
[0.13.0]: https://github.com/astraw/bui-backend/compare/bui-backend/0.12.0...bui-backend/0.13.0
[0.12.0]: https://github.com/astraw/bui-backend/compare/bui-backend/0.11.0...bui-backend/0.12.0
[0.11.0]: https://github.com/astraw/bui-backend/compare/bui-backend/0.10.0...bui-backend/0.11.0
[0.10.0]: https://github.com/astraw/bui-backend/compare/bui-backend/0.9.0...bui-backend/0.10.0
[0.9.0]: https://github.com/astraw/bui-backend/compare/bui-backend/0.8.0...bui-backend/0.9.0
[0.8.0]: https://github.com/astraw/bui-backend/compare/bui-backend/0.6.0...bui-backend/0.8.0
[0.6.0]: https://github.com/astraw/bui-backend/compare/bui-backend/0.5.0...bui-backend/0.6.0
[0.5.0]: https://github.com/astraw/bui-backend/compare/bui-backend/0.4.1...bui-backend/0.5.0
[0.4.1]: https://github.com/astraw/bui-backend/compare/bui-backend/0.4.0...bui-backend/0.4.1
[0.4.0]: https://github.com/astraw/bui-backend/compare/bui-backend/0.3.0...bui-backend/0.4.0
[0.3.0]: https://github.com/astraw/bui-backend/compare/bui-backend/0.2.1...bui-backend/0.3.0
[0.2.1]: https://github.com/astraw/bui-backend/compare/bui-backend/0.2.0...bui-backend/0.2.1
[0.2.0]: https://github.com/astraw/bui-backend/compare/bui-backend/0.1.1...bui-backend/0.2.0
[0.1.1]: https://github.com/astraw/bui-backend/compare/bui-backend/0.1.0...bui-backend/0.1.1
