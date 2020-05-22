//! Integration between `Value` and `std::fmt`.
//!
//! This module allows any `Value` to implement the `fmt::Debug` and `fmt::Display` traits,
//! and for any `fmt::Debug` or `fmt::Display` to be captured as a `Value`.

use std::fmt;

use super::{Inner, Visitor, cast};
use crate::kv;
use crate::kv::value::{Error, Slot};

impl<'v> kv::Value<'v> {
    /// Get a value from a debuggable type.
    pub fn from_debug<T>(value: &'v T) -> Self
    where
        T: fmt::Debug + 'static,
    {
        // If the value is a primitive type, then cast it here, avoiding needing to erase its value
        // This makes `Value`s produced by `from_debug` more useful, which is worthwhile since the
        // trait appears in the standard library so is widely implemented
        kv::Value {
            inner: if let Some(primitive) = cast::into_primitive(value) {
                Inner::Primitive(primitive)
            } else {
                Inner::Debug(value)
            },
        }
    }

    /// Get a value from a displayable type.
    pub fn from_display<T>(value: &'v T) -> Self
    where
        T: fmt::Display + 'static,
    {
        // If the value is a primitive type, then cast it here, avoiding needing to erase its value
        // This makes `Value`s produced by `from_display` more useful, which is worthwhile since the
        // trait appears in the standard library so is widely implemented
        kv::Value {
            inner: if let Some(primitive) = cast::into_primitive(value) {
                Inner::Primitive(primitive)
            } else {
                Inner::Display(value)
            },
        }
    }
}

impl<'s, 'f> Slot<'s, 'f> {
    /// Fill the slot with a debuggable value.
    ///
    /// The given value doesn't need to satisfy any particular lifetime constraints.
    ///
    /// # Panics
    ///
    /// Calling more than a single `fill` method on this slot will panic.
    pub fn fill_debug<T>(&mut self, value: T) -> Result<(), Error>
    where
        T: fmt::Debug,
    {
        self.fill(|visitor| visitor.debug(&value))
    }

    /// Fill the slot with a displayable value.
    ///
    /// The given value doesn't need to satisfy any particular lifetime constraints.
    ///
    /// # Panics
    ///
    /// Calling more than a single `fill` method on this slot will panic.
    pub fn fill_display<T>(&mut self, value: T) -> Result<(), Error>
    where
        T: fmt::Display,
    {
        self.fill(|visitor| visitor.display(&value))
    }
}

pub(in kv::value) use self::fmt::{Arguments, Debug, Display};

impl<'v> fmt::Debug for kv::Value<'v> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.visit(&mut FmtVisitor(f))?;

        Ok(())
    }
}

impl<'v> fmt::Display for kv::Value<'v> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.visit(&mut FmtVisitor(f))?;

        Ok(())
    }
}

struct FmtVisitor<'a, 'b: 'a>(&'a mut fmt::Formatter<'b>);

impl<'a, 'b: 'a, 'v> Visitor<'v> for FmtVisitor<'a, 'b> {
    fn debug(&mut self, v: &dyn fmt::Debug) -> Result<(), Error> {
        v.fmt(self.0)?;

        Ok(())
    }

    fn u64(&mut self, v: u64) -> Result<(), Error> {
        self.debug(&format_args!("{:?}", v))
    }

    fn i64(&mut self, v: i64) -> Result<(), Error> {
        self.debug(&format_args!("{:?}", v))
    }

    fn f64(&mut self, v: f64) -> Result<(), Error> {
        self.debug(&format_args!("{:?}", v))
    }

    fn bool(&mut self, v: bool) -> Result<(), Error> {
        self.debug(&format_args!("{:?}", v))
    }

    fn char(&mut self, v: char) -> Result<(), Error> {
        self.debug(&format_args!("{:?}", v))
    }

    fn str(&mut self, v: &str) -> Result<(), Error> {
        self.debug(&format_args!("{:?}", v))
    }

    fn none(&mut self) -> Result<(), Error> {
        self.debug(&format_args!("None"))
    }

    #[cfg(feature = "kv_unstable_sval")]
    fn sval(&mut self, v: &dyn super::sval::Value) -> Result<(), Error> {
        super::sval::fmt(self.0, v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kv::value::test::Token;

    #[test]
    fn fmt_cast() {
        assert_eq!(
            42u32,
            kv::Value::from_debug(&42u64)
                .to_u32()
                .expect("invalid value")
        );

        assert_eq!(
            "a string",
            kv::Value::from_display(&"a string")
                .to_borrowed_str()
                .expect("invalid value")
        );
    }

    #[test]
    fn fmt_capture() {
        assert_eq!(kv::Value::from_debug(&1u16).to_token(), Token::U64(1));
        assert_eq!(kv::Value::from_display(&1u16).to_token(), Token::U64(1));

        assert_eq!(kv::Value::from_debug(&Some(1u16)).to_token(), Token::U64(1));
    }
}
