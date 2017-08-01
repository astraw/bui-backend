error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }
    errors {
        BuiError(msg: String) {
            description("BuiError")
            display("BuiError: {}", msg)
        }
    }
    foreign_links {
        HyperError(::hyper::Error);
    }
}
