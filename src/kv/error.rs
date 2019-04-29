use std::fmt;

/// An error encountered while working with structured data.
#[derive(Clone, Debug)]
pub struct Error {
    msg: &'static str,
}

impl Error {
    /// Create an error from the given message.
    pub fn msg(msg: &'static str) -> Self {
        Error {
            msg: msg,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.msg.fmt(f)
    }
}

impl From<fmt::Error> for Error {
    fn from(_: fmt::Error) -> Self {
        Error::msg("formatting failed")
    }
}

impl From<Error> for fmt::Error {
    fn from(_: Error) -> Self {
        fmt::Error
    }
}

#[cfg(feature = "std")]
mod std_support {
    use super::*;
    use std::error;

    impl error::Error for Error {
        fn description(&self) -> &str {
            "key values error"
        }
    }
}
