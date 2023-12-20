use ::hyper;
use thiserror::Error;

/// Possible errors
#[derive(Error, Debug)]
pub enum Error {
    /// A non-local IP address requires a token
    #[error("non-localhost address requires pre-shared token")]
    NonLocalhostRequiresPreSharedToken,
    /// A wrapped error from the hyper crate
    #[error("hyper error `{0}`")]
    Hyper(#[from] hyper::Error),

    /// A wrapped error from std::io
    #[error("IO error `{0}`")]
    Io(#[from] std::io::Error),

    /// An error that occurred with an event stream.
    #[error("rx event")]
    RxEvent,
}
