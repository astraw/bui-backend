use ::hyper;
use ::tokio_executor;

/// Possible errors
#[derive(Fail, Debug)]
pub enum Error {
    /// A wrapped error from the hyper crate
    #[fail(display = "{}", _0)]
    Hyper(#[cause] hyper::Error),
    #[fail(display = "tokio_executor::SpawnError")]
    TokioSpawn,
    #[fail(display = "rx event")]
    RxEvent,
}

impl From<hyper::Error> for Error {
    fn from(orig: hyper::Error) -> Error {
        Error::Hyper(orig)
    }
}

impl From<tokio_executor::SpawnError> for Error {
    fn from(_orig: tokio_executor::SpawnError) -> Error {
        Error::TokioSpawn
    }
}
