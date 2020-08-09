use std::{error, fmt};

use super::{cast, Inner};
use crate::kv;
use crate::kv::value::{self, Slot, ToValue};

impl<'v> kv::Value<'v> {
    /// Get a value from an error.
    pub fn capture_error<T>(value: &'v T) -> Self
    where
        T: error::Error + 'static,
    {
        kv::Value {
            inner: Inner::Error {
                value,
                type_id: Some(cast::type_id::<T>()),
            },
        }
    }

    /// Try get an error from this value.
    pub fn to_error<'a>(&'a self) -> Option<impl Error + 'a> {
        struct RefError<'a>(&'a dyn Error);

        impl<'a> Error for RefError<'a> {
            fn source(&self) -> Option<&(dyn Error + 'static)> {
                self.0.source()
            }

            // NOTE: Once backtraces are stable, add them here too
        }

        impl<'a> fmt::Debug for RefError<'a> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                fmt::Debug::fmt(self.0, f)
            }
        }

        impl<'a> fmt::Display for RefError<'a> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                fmt::Display::fmt(self.0, f)
            }
        }

        match self.inner {
            Inner::Error { value, .. } => Some(RefError(value)),
            _ => None,
        }
    }
}

impl<'s, 'f> Slot<'s, 'f> {
    /// Fill the slot with an error.
    ///
    /// The given value doesn't need to satisfy any particular lifetime constraints.
    ///
    /// # Panics
    ///
    /// Calling more than a single `fill` method on this slot will panic.
    pub fn fill_error<T>(&mut self, value: T) -> Result<(), value::Error>
    where
        T: error::Error,
    {
        self.fill(|visitor| visitor.error(&value))
    }
}

pub(in kv::value) use self::error::Error;

impl<'v> ToValue for dyn error::Error + 'v {
    fn to_value(&self) -> kv::Value {
        kv::Value::from(self)
    }
}

impl<'v> From<&'v (dyn error::Error)> for kv::Value<'v> {
    fn from(value: &'v (dyn error::Error)) -> kv::Value<'v> {
        kv::Value {
            inner: Inner::Error {
                value,
                type_id: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io;

    #[test]
    fn error_capture() {
        let err = io::Error::from(io::ErrorKind::Other);

        assert_eq!(
            err.to_string(),
            kv::Value::capture_error(&err)
                .to_error()
                .expect("invalid value")
                .to_string()
        );
    }

    #[test]
    fn error_downcast() {
        let err = io::Error::from(io::ErrorKind::Other);

        assert!(kv::Value::capture_error(&err)
            .downcast_ref::<io::Error>()
            .is_some());
    }
}
