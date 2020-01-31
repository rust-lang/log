//! Structured values.

use std::fmt;

mod impls;
mod internal;

#[cfg(test)]
pub(in kv) mod test;

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

/// A type that requires extra work to convert into a [`Value`](struct.Value.html).
///
/// This trait is a more advanced initialization API than [`ToValue`](trait.ToValue.html).
/// It's intended for erased values coming from other logging frameworks that may need
/// to perform extra work to determine the concrete type to use.
pub trait Fill {
    /// Fill a value.
    fn fill(&self, slot: &mut Slot) -> Result<(), Error>;
}

impl<'a, T> Fill for &'a T
where
    T: Fill + ?Sized,
{
    fn fill(&self, slot: &mut Slot) -> Result<(), Error> {
        (**self).fill(slot)
    }
}

/// A value slot to fill using the [`Fill`](trait.Fill.html) trait.
pub struct Slot<'a> {
    filled: bool,
    visitor: &'a mut dyn Visitor,
}

impl<'a> fmt::Debug for Slot<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Slot").finish()
    }
}

impl<'a> Slot<'a> {
    fn new(visitor: &'a mut dyn Visitor) -> Self {
        Slot {
            visitor,
            filled: false,
        }
    }

    /// Fill the slot with a value.
    ///
    /// The given value doesn't need to satisfy any particular lifetime constraints.
    ///
    /// # Panics
    ///
    /// Calling `fill` more than once will panic.
    pub fn fill(&mut self, value: Value) -> Result<(), Error> {
        assert!(!self.filled, "the slot has already been filled");
        self.filled = true;

        value.visit(self.visitor)
    }
}

/// A value in a structured key-value pair.
pub struct Value<'v> {
    inner: Inner<'v>,
}

impl<'v> Value<'v> {
    /// Get a value from an internal `Visit`.
    fn from_primitive(value: Primitive<'v>) -> Self {
        Value {
            inner: Inner::Primitive(value),
        }
    }

    /// Get a value from a fillable slot.
    pub fn from_fill<T>(value: &'v T) -> Self
    where
        T: Fill,
    {
        Value {
            inner: Inner::Fill(value),
        }
    }

    /// Try coerce the value into a borrowed string.
    pub fn as_str(&self) -> Option<&str> {
        self.inner.as_str()
    }

    /// Try coerce the value into a `u8`.
    pub fn as_u8(&self) -> Option<u8> {
        self.inner.as_u64().map(|v| v as u8)
    }

    /// Try coerce the value into a `u16`.
    pub fn as_u16(&self) -> Option<u16> {
        self.inner.as_u64().map(|v| v as u16)
    }

    /// Try coerce the value into a `u32`.
    pub fn as_u32(&self) -> Option<u32> {
        self.inner.as_u64().map(|v| v as u32)
    }

    /// Try coerce the value into a `u64`.
    pub fn as_u64(&self) -> Option<u64> {
        self.inner.as_u64()
    }

    /// Try coerce the value into a `i8`.
    pub fn as_i8(&self) -> Option<i8> {
        self.inner.as_i64().map(|v| v as i8)
    }

    /// Try coerce the value into a `i16`.
    pub fn as_i16(&self) -> Option<i16> {
        self.inner.as_i64().map(|v| v as i16)
    }

    /// Try coerce the value into a `i32`.
    pub fn as_i32(&self) -> Option<i32> {
        self.inner.as_i64().map(|v| v as i32)
    }

    /// Try coerce the value into a `i64`.
    pub fn as_i64(&self) -> Option<i64> {
        self.inner.as_i64()
    }

    /// Try coerce the value into a `f32`.
    pub fn as_f32(&self) -> Option<f32> {
        self.inner.as_f64().map(|v| v as f32)
    }

    /// Try coerce the value into a `f64`.
    pub fn as_f64(&self) -> Option<f64> {
        self.inner.as_f64()
    }

    /// Try coerce the vlaue into a `char`.
    pub fn as_char(&self) -> Option<char> {
        self.inner.as_char()
    }

    /// Try coerce the vlaue into a `bool`.
    pub fn as_bool(&self) -> Option<bool> {
        self.inner.as_bool()
    }

    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        self.inner.visit(visitor)
    }
}

#[cfg(feature = "std")]
mod std_support {
    use super::*;

    use std::borrow::Cow;

    impl<'v> Value<'v> {
        /// Try coerce the value into an owned or borrowed string.
        pub fn to_str(&self) -> Option<Cow<str>> {
            self.inner.to_str()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn primtive_coercion() {
            assert_eq!(
                "a string",
                "a string"
                    .to_owned()
                    .to_value()
                    .as_str()
                    .expect("invalid value")
            );
            assert_eq!(
                "a string",
                &*"a string".to_value().to_str().expect("invalid value")
            );
            assert_eq!(
                "a string",
                &*"a string"
                    .to_owned()
                    .to_value()
                    .to_str()
                    .expect("invalid value")
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_value() {
        struct TestFill;

        impl Fill for TestFill {
            fn fill(&self, slot: &mut Slot) -> Result<(), Error> {
                let dbg: &dyn fmt::Debug = &1;

                slot.fill(Value::from_debug(&dbg))
            }
        }

        assert_eq!("1", Value::from_fill(&TestFill).to_string());
    }

    #[test]
    #[should_panic]
    fn fill_multiple_times_panics() {
        struct BadFill;

        impl Fill for BadFill {
            fn fill(&self, slot: &mut Slot) -> Result<(), Error> {
                slot.fill(42.into())?;
                slot.fill(6789.into())?;

                Ok(())
            }
        }

        let _ = Value::from_fill(&BadFill).to_string();
    }

    #[test]
    fn primitive_coercion() {
        assert_eq!(
            "a string",
            "a string".to_value().as_str().expect("invalid value")
        );
        assert_eq!(
            "a string",
            Some("a string").to_value().as_str().expect("invalid value")
        );

        assert_eq!(1u8, 1u64.to_value().as_u8().expect("invalid value"));
        assert_eq!(1u16, 1u64.to_value().as_u16().expect("invalid value"));
        assert_eq!(1u32, 1u64.to_value().as_u32().expect("invalid value"));
        assert_eq!(1u64, 1u64.to_value().as_u64().expect("invalid value"));

        assert_eq!(-1i8, -1i64.to_value().as_i8().expect("invalid value"));
        assert_eq!(-1i16, -1i64.to_value().as_i16().expect("invalid value"));
        assert_eq!(-1i32, -1i64.to_value().as_i32().expect("invalid value"));
        assert_eq!(-1i64, -1i64.to_value().as_i64().expect("invalid value"));

        assert!(1f32.to_value().as_f32().is_some(), "invalid value");
        assert!(1f64.to_value().as_f64().is_some(), "invalid value");

        assert_eq!('a', 'a'.to_value().as_char().expect("invalid value"));
        assert_eq!(true, true.to_value().as_bool().expect("invalid value"));
    }
}
