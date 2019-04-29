//! Structured values.

use std::fmt;

mod internal;
mod impls;

use kv::Error;

use self::internal::{Inner, Visit, Visitor};

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
    inner: Inner<'v>,
}

impl<'v> Value<'v> {
    /// Get a value from an internal `Visit`.
    fn from_internal<T>(value: &'v T) -> Self
    where
        T: Visit,
    {
        Value {
            inner: Inner::Internal(value),
        }
    }

    /// Get a value from a debuggable type.
    pub fn from_debug<T>(value: &'v T) -> Self
    where
        T: fmt::Debug,
    {
        Value {
            inner: Inner::Debug(value),
        }
    }

    /// Get a  value from a displayable type.
    pub fn from_display<T>(value: &'v T)  -> Self
    where
        T: fmt::Display,
    {
        Value {
            inner: Inner::Display(value),
        }
    }

    fn visit(&self, visitor: &mut Visitor) -> Result<(), Error> {
        self.inner.visit(visitor)
    }
}

impl<'v> fmt::Debug for Value<'v> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.visit(&mut self::internal::FmtVisitor(f))?;

        Ok(())
    }
}

impl<'v> fmt::Display for Value<'v> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.visit(&mut self::internal::FmtVisitor(f))?;

        Ok(())
    }
}
