//! Structured values.

mod fill;
mod impls;
mod internal;

#[cfg(test)]
pub(in kv) mod test;

pub use self::fill::{Fill, Slot};
pub use kv::Error;

use self::internal::{Inner, Primitive, Visitor};

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
        Value { inner: self.inner }
    }
}

/// A value in a structured key-value pair.
///
/// # Capturing values
///
/// There are a few ways to capture a value:
///
/// - Using the `Value::capture_*` methods.
/// - Using the `Value::from_*` methods.
/// - Using the `ToValue` trait.
/// - Using the standard `From` trait.
/// - Using the `Fill` API.
///
/// ## Using the `Value::capture_*` methods
///
/// `Value` offers a few constructor methods that capture values of different kinds.
/// These methods require a `T: 'static` to support downcasting.
///
/// ```
/// use log::kv::Value;
///
/// let value = Value::capture_debug(&42i32);
///
/// assert_eq!(Some(42), value.to_i32());
/// ```
///
/// ## Using the `Value::from_*` methods
///
/// `Value` offers a few constructor methods that capture values of different kinds.
/// These methods don't require `T: 'static`, but can't support downcasting.
///
/// ```
/// use log::kv::Value;
///
/// let value = Value::from_debug(&42i32);
///
/// assert_eq!(None, value.to_i32());
/// ```
///
/// ## Using the `ToValue` trait
///
/// The `ToValue` trait can be used to capture values generically.
/// It's the bound used by `Source`.
///
/// ```
/// # use log::kv::ToValue;
/// let value = 42i32.to_value();
///
/// assert_eq!(Some(42), value.to_i32());
/// ```
///
/// ```
/// # use std::fmt::Debug;
/// use log::kv::ToValue;
///
/// let value = (&42i32 as &dyn Debug).to_value();
///
/// assert_eq!(None, value.to_i32());
/// ```
///
/// ## Using the standard `From` trait
///
/// Standard types that implement `ToValue` also implement `From`.
///
/// ```
/// use log::kv::Value;
///
/// let value = Value::from(42i32);
///
/// assert_eq!(Some(42), value.to_i32());
/// ```
///
/// ```
/// # use std::fmt::Debug;
/// use log::kv::Value;
///
/// let value = Value::from(&42i32 as &dyn Debug);
///
/// assert_eq!(None, value.to_i32());
/// ```
///
/// ## Using the `Fill` API
///
/// The `Fill` trait is a way to bridge APIs that may not be directly
/// compatible with other constructor methods.
///
/// ```
/// use log::kv::value::{Value, Slot, Fill, Error};
///
/// struct FillSigned;
///
/// impl Fill for FillSigned {
///     fn fill(&self, slot: &mut Slot) -> Result<(), Error> {
///         slot.fill_any(42i32)
///     }
/// }
///
/// let value = Value::from_fill(&FillSigned);
///
/// assert_eq!(Some(42), value.to_i32());
/// ```
///
/// ```
/// # use std::fmt::Debug;
/// use log::kv::value::{Value, Slot, Fill, Error};
///
/// struct FillDebug;
///
/// impl Fill for FillDebug {
///     fn fill(&self, slot: &mut Slot) -> Result<(), Error> {
///         slot.fill_debug(&42i32 as &dyn Debug)
///     }
/// }
///
/// let value = Value::from_fill(&FillDebug);
///
/// assert_eq!(None, value.to_i32());
/// ```
pub struct Value<'v> {
    inner: Inner<'v>,
}

impl<'v> Value<'v> {
    /// Get a value from a type implementing `ToValue`.
    pub fn from_any<T>(value: &'v T) -> Self
    where
        T: ToValue,
    {
        value.to_value()
    }

    /// Get a value from an internal primitive.
    fn from_primitive<T>(value: T) -> Self
    where
        T: Into<Primitive<'v>>,
    {
        Value {
            inner: Inner::Primitive(value.into()),
        }
    }

    /// Visit the value using an internal visitor.
    fn visit<'a>(&'a self, visitor: &mut dyn Visitor<'a>) -> Result<(), Error> {
        self.inner.visit(visitor)
    }
}
