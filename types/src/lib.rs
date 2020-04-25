//! types shared between frontend and backend of the `bui-backend` crate
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

use serde::{Deserialize, Serialize};

/// Identifier for each session (one per client browser).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct SessionKey(pub uuid::Uuid);

#[cfg(feature = "uuid-v4")]
impl SessionKey {
    /// Create a new SessionKey
    #[cfg_attr(docsrs, doc(cfg(feature = "uuid-v4")))]
    pub fn new() -> Self {
        SessionKey(uuid::Uuid::new_v4())
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

/// A token which can be required to gain access to HTTP API
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum AccessToken {
    /// No token needed (access must be controlled via other means).
    NoToken,
    /// A pre-shared token to gain access.
    PreSharedToken(String),
}

impl AccessToken {
    /// Check if input string matches.
    pub fn does_match(&self, test_str: &str) -> bool {
        match self {
            &AccessToken::NoToken => true,
            &AccessToken::PreSharedToken(ref s) => s == test_str,
        }
    }
}
