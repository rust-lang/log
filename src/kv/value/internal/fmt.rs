//! Integration between `Value` and `std::fmt`.
//!
//! This module allows any `Value` to implement the `fmt::Debug` and `fmt::Display` traits,
//! and for any `fmt::Debug` or `fmt::Display` to be captured as a `Value`.

use std::fmt;

use super::{cast, Inner, Visitor};
use crate::kv;
use crate::kv::value::{Error, Slot, ToValue};

impl<'v> kv::Value<'v> {
    /// Get a value from a debuggable type.
    ///
    /// This method will attempt to capture the given value as a well-known primitive
    /// before resorting to using its `Debug` implementation.
    pub fn capture_debug<T>(value: &'v T) -> Self
    where
        T: fmt::Debug + 'static,
    {
        cast::try_from_primitive(value).unwrap_or(kv::Value {
            inner: Inner::Debug(value),
        })
    }

    /// Get a value from a debuggable type.
    pub fn from_debug<T>(value: &'v T) -> Self
    where
        T: fmt::Debug,
    {
        kv::Value {
            inner: Inner::Debug(value),
        }
    }

    /// Get a value from a displayable type.
    ///
    /// This method will attempt to capture the given value as a well-known primitive
    /// before resorting to using its `Display` implementation.
    pub fn capture_display<T>(value: &'v T) -> Self
    where
        T: fmt::Display + 'static,
    {
        cast::try_from_primitive(value).unwrap_or(kv::Value {
            inner: Inner::Display(value),
        })
    }

    /// Get a value from a displayable type.
    pub fn from_display<T>(value: &'v T) -> Self
    where
        T: fmt::Display,
    {
        kv::Value {
            inner: Inner::Display(value),
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
        struct DebugVisitor<'a, 'b: 'a>(&'a mut fmt::Formatter<'b>);

        impl<'a, 'b: 'a, 'v> Visitor<'v> for DebugVisitor<'a, 'b> {
            fn debug(&mut self, v: &dyn fmt::Debug) -> Result<(), Error> {
                fmt::Debug::fmt(v, self.0)?;

                Ok(())
            }

            fn display(&mut self, v: &dyn fmt::Display) -> Result<(), Error> {
                fmt::Display::fmt(v, self.0)?;

                Ok(())
            }

            fn u64(&mut self, v: u64) -> Result<(), Error> {
                fmt::Debug::fmt(&v, self.0)?;

                Ok(())
            }

            fn i64(&mut self, v: i64) -> Result<(), Error> {
                fmt::Debug::fmt(&v, self.0)?;

                Ok(())
            }

            fn f64(&mut self, v: f64) -> Result<(), Error> {
                fmt::Debug::fmt(&v, self.0)?;

                Ok(())
            }

            fn bool(&mut self, v: bool) -> Result<(), Error> {
                fmt::Debug::fmt(&v, self.0)?;

                Ok(())
            }

            fn char(&mut self, v: char) -> Result<(), Error> {
                fmt::Debug::fmt(&v, self.0)?;

                Ok(())
            }

            fn str(&mut self, v: &str) -> Result<(), Error> {
                fmt::Debug::fmt(&v, self.0)?;

                Ok(())
            }

            fn none(&mut self) -> Result<(), Error> {
                self.debug(&format_args!("None"))
            }

            #[cfg(feature = "kv_unstable_sval")]
            fn sval(&mut self, v: &dyn super::sval::Value) -> Result<(), Error> {
                super::sval::fmt(self.0, v)
            }
        }

        self.visit(&mut DebugVisitor(f)).map_err(|_| fmt::Error)?;

        Ok(())
    }
}

impl<'v> fmt::Display for kv::Value<'v> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        struct DisplayVisitor<'a, 'b: 'a>(&'a mut fmt::Formatter<'b>);

        impl<'a, 'b: 'a, 'v> Visitor<'v> for DisplayVisitor<'a, 'b> {
            fn debug(&mut self, v: &dyn fmt::Debug) -> Result<(), Error> {
                fmt::Debug::fmt(v, self.0)?;

                Ok(())
            }

            fn display(&mut self, v: &dyn fmt::Display) -> Result<(), Error> {
                fmt::Display::fmt(v, self.0)?;

                Ok(())
            }

            fn u64(&mut self, v: u64) -> Result<(), Error> {
                fmt::Display::fmt(&v, self.0)?;

                Ok(())
            }

            fn i64(&mut self, v: i64) -> Result<(), Error> {
                fmt::Display::fmt(&v, self.0)?;

                Ok(())
            }

            fn f64(&mut self, v: f64) -> Result<(), Error> {
                fmt::Display::fmt(&v, self.0)?;

                Ok(())
            }

            fn bool(&mut self, v: bool) -> Result<(), Error> {
                fmt::Display::fmt(&v, self.0)?;

                Ok(())
            }

            fn char(&mut self, v: char) -> Result<(), Error> {
                fmt::Display::fmt(&v, self.0)?;

                Ok(())
            }

            fn str(&mut self, v: &str) -> Result<(), Error> {
                fmt::Display::fmt(&v, self.0)?;

                Ok(())
            }

            fn none(&mut self) -> Result<(), Error> {
                self.debug(&format_args!("None"))
            }

            #[cfg(feature = "kv_unstable_sval")]
            fn sval(&mut self, v: &dyn super::sval::Value) -> Result<(), Error> {
                super::sval::fmt(self.0, v)
            }
        }

        self.visit(&mut DisplayVisitor(f)).map_err(|_| fmt::Error)?;

        Ok(())
    }
}


impl<'v> ToValue for dyn fmt::Debug + 'v {
    fn to_value(&self) -> kv::Value {
        kv::Value::from(self)
    }
}

impl<'v> ToValue for dyn fmt::Display + 'v {
    fn to_value(&self) -> kv::Value {
        kv::Value::from(self)
    }
}

impl<'v> From<&'v (dyn fmt::Debug)> for kv::Value<'v> {
    fn from(value: &'v (dyn fmt::Debug)) -> kv::Value<'v> {
        kv::Value {
            inner: Inner::Debug(value),
        }
    }
}

impl<'v> From<&'v (dyn fmt::Display)> for kv::Value<'v> {
    fn from(value: &'v (dyn fmt::Display)) -> kv::Value<'v> {
        kv::Value {
            inner: Inner::Display(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kv::value::test::Token;

    use crate::kv::value::ToValue;

    #[test]
    fn fmt_capture() {
        assert_eq!(kv::Value::capture_debug(&1u16).to_token(), Token::U64(1));
        assert_eq!(kv::Value::capture_display(&1u16).to_token(), Token::U64(1));

        assert_eq!(
            kv::Value::capture_debug(&Some(1u16)).to_token(),
            Token::U64(1)
        );
    }

    #[test]
    fn fmt_cast() {
        assert_eq!(
            42u32,
            kv::Value::capture_debug(&42u64)
                .to_u32()
                .expect("invalid value")
        );

        assert_eq!(
            "a string",
            kv::Value::capture_display(&"a string")
                .to_borrowed_str()
                .expect("invalid value")
        );
    }

    #[test]
    fn fmt_debug() {
        assert_eq!(
            format!("{:?}", "a string"),
            format!("{:?}", "a string".to_value()),
        );

        assert_eq!(
            format!("{:04?}", 42u64),
            format!("{:04?}", 42u64.to_value()),
        );
    }

    #[test]
    fn fmt_display() {
        assert_eq!(
            format!("{}", "a string"),
            format!("{}", "a string".to_value()),
        );

        assert_eq!(format!("{:04}", 42u64), format!("{:04}", 42u64.to_value()),);
    }
}
