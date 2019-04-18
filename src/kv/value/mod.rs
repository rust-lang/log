//! Structured values.

use std::fmt;

mod any;
mod backend;
mod impls;

use kv::KeyValueError;

use self::any::{Any, FromAnyFn};

/// A type that can be converted into a [`Value`](struct.Value.html).
pub trait ToValue {
    /// Perform the conversion.
    fn to_value(&self) -> Value;
}

impl<'a, T> ToValue for &'a T
where
    T: ToValue + ?Sized,
{
    fn to_value(&self) -> Value {
        (**self).to_value()
    }
}

impl<'v> ToValue for Value<'v> {
    fn to_value(&self) -> Value {
        Value {
            inner: self.inner,
        }
    }
}

/// A value in a structured key-value pair.
pub struct Value<'v> {
    inner: Any<'v>,
}

impl<'v> Value<'v> {
    // FIXME: The `from_any` API is intended to be public.
    // Its implications need discussion first though.
    fn from_any<T>(v: &'v T, from: FromAnyFn<T>) -> Self {
        Value {
            inner: Any::new(v, from)
        }
    }

    /// Get a value from a formattable type.
    pub fn from_debug<T>(value: &'v T) -> Self
    where
        T: fmt::Debug,
    {
        Self::from_any(value, |from, value| from.debug(value))
    }
}

impl<'v> fmt::Debug for Value<'v> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.visit(&mut self::backend::FmtBackend(f))?;

        Ok(())
    }
}

impl<'v> fmt::Display for Value<'v> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.visit(&mut self::backend::FmtBackend(f))?;

        Ok(())
    }
}
