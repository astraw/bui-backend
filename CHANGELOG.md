# Change Log

All user visible changes to this project will be documented in this file.
This project adheres to [Semantic Versioning](http://semver.org/), as described
for Rust libraries in [RFC #1105](https://github.com/rust-lang/rfcs/blob/master/text/1105-api-evolution.md)

## unreleased

### Changed

* Update to tokio reform (`tokio` 0.1) from `tokio-core`. This is associated
  with and API change in which an external tokio Executor is passed in to
  `create_bui_app_inner()` on which all tasks are spawned.
* Update to `hyper` 0.12.
* Drop use of raii-change-tracker crate but use a derivative, now included
  as `bui_backend::change_tracker`. The new `ChangeTracker` type allows changing
  the owned value using closures and notifies listeners just after the closure
  completes.
* `walkdir`, `includedir`, and `includedir_codegen` crates only used when the
  `bundle_files` feature is used.
* Drop elm frontend in `bui-demo`.
* Simplify wire format for callback data to contain only a JSON payload.

### Added

* Make compile-time error more readable when compiling codegen
  crate without required feature flag.

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

[0.6.0]: https://github.com/astraw/bui-backend/compare/bui-backend/0.5.0...bui-backend/0.6.0
[0.5.0]: https://github.com/astraw/bui-backend/compare/bui-backend/0.4.1...bui-backend/0.5.0
[0.4.1]: https://github.com/astraw/bui-backend/compare/bui-backend/0.4.0...bui-backend/0.4.1
[0.4.0]: https://github.com/astraw/bui-backend/compare/bui-backend/0.3.0...bui-backend/0.4.0
[0.3.0]: https://github.com/astraw/bui-backend/compare/bui-backend/0.2.1...bui-backend/0.3.0
[0.2.1]: https://github.com/astraw/bui-backend/compare/bui-backend/0.2.0...bui-backend/0.2.1
[0.2.0]: https://github.com/astraw/bui-backend/compare/bui-backend/0.1.1...bui-backend/0.2.0
[0.1.1]: https://github.com/astraw/bui-backend/compare/bui-backend/0.1.0...bui-backend/0.1.1
