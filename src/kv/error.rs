use std::fmt;

/// An error encountered while working with structured data.
#[derive(Clone, Debug)]
pub struct KeyValueError {
    msg: &'static str,
}

impl KeyValueError {
    /// Create an error from the given message.
    pub fn msg(msg: &'static str) -> Self {
        KeyValueError {
            msg: msg,
        }
    }
}

impl fmt::Display for KeyValueError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.msg.fmt(f)
    }
}

impl From<fmt::Error> for KeyValueError {
    fn from(_: fmt::Error) -> Self {
        KeyValueError::msg("formatting failed")
    }
}

impl From<KeyValueError> for fmt::Error {
    fn from(_: KeyValueError) -> Self {
        fmt::Error
    }
}

#[cfg(feature = "std")]
mod std_support {
    use super::*;
    use std::error;

    impl error::Error for KeyValueError {
        fn description(&self) -> &str {
            "key_values error"
        }
    }
}
