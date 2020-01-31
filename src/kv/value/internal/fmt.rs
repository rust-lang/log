//! Integration between `Value` and `std::fmt`.
//!
//! This module allows any `Value` to implement the `fmt::Debug` and `fmt::Display` traits,
//! and for any `fmt::Debug` or `fmt::Display` to be captured as a `Value`.

use std::fmt;

use super::{Inner, Visitor};
use crate::kv;
use crate::kv::value::{Error, Slot};

impl<'v> kv::Value<'v> {
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

pub(in kv::value) use self::fmt::{Debug, Display};

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
