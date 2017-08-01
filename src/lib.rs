/// A library for creating cross-platform user interfaces based on the browser.
///
/// Uses
/// [server-sent-events](https://html.spec.whatwg.org/multipage/server-sent-events.html)
/// to push changes to connected clients. Unfortunately [some
/// browsers](http://caniuse.com/#feat=eventsource) do not support this.
#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate log;
#[macro_use]
extern crate error_chain;
extern crate raii_change_tracker;
extern crate includedir;
extern crate serde;
extern crate serde_json;
extern crate futures;
extern crate hyper;
extern crate jsonwebtoken;
extern crate uuid;

pub mod errors;
pub mod lowlevel;
pub mod highlevel;
