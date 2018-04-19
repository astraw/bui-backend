use ::hyper;

/// Possible errors
#[derive(Fail, Debug)]
pub enum Error {
    /// A wrapped error from the hyper crate
    #[fail(display = "{}", _0)]
    Hyper(#[cause] hyper::Error),
}

impl From<hyper::Error> for Error {
    fn from(orig: hyper::Error) -> Error {
        Error::Hyper(orig)
    }
}
