//! Error and result types.
error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }
    foreign_links {
        HyperError(::hyper::Error) #[doc = "A Hyper error."];
    }
}
