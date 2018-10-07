#[macro_use]
extern crate serde_derive;
extern crate uuid;

use uuid::Uuid;

/// Identifier for each session (one per client browser).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct SessionKey(pub Uuid);

impl SessionKey {
    pub fn new() -> Self {
        SessionKey(Uuid::new_v4())
    }
}

/// Identifier for each connected event stream listener (one per client tab).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ConnectionKey(pub u32);

/// Callback data from a connected client.
#[derive(Clone, Debug)]
pub struct CallbackDataAndSession<T> {
    /// The callback data sent from the client.
    pub payload: T,
    /// The session key associated with the client.
    pub session_key: SessionKey,
}
