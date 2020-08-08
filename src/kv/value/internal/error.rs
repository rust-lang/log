use std::error;

use super::Inner;
use crate::kv;
use crate::kv::value::{self, Slot, ToValue};

impl<'v> kv::Value<'v> {
    /// Get a value from an error.
    pub fn capture_error<T>(value: &'v T) -> Self
    where
        T: error::Error + 'static,
    {
        kv::Value {
            inner: Inner::Error(value),
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
            inner: Inner::Error(value),
        }
    }
}
