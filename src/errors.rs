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
    Hyper(hyper::Error),
    /// An error that occurred with an event stream.
    #[error("rx event")]
    RxEvent,
}

impl From<hyper::Error> for Error {
    fn from(orig: hyper::Error) -> Error {
        Error::Hyper(orig)
    }
}
