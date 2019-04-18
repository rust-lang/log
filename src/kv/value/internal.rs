use std::fmt;

use super::KeyValueError;

// `Visit` and `Visitor` is an internal API for visiting the structure of a value.
// It's not intended to be public (at this stage).
// 
// Right now we only have an implementation for `std::fmt`, but
// this trait makes it possible to add more structured backends like
// `serde` that can retain that original structure.

/// A container for a structured value for a specific kind of visitor.
#[derive(Clone, Copy)]
pub(super) enum Inner<'v> {
    /// An internal `Visit`. It'll be an internal structure-preserving
    /// type from the standard library that's implemented in this crate.
    Internal(&'v Visit),
    /// A formattable value.
    Debug(&'v fmt::Debug),
}

impl<'v> Inner<'v> {
    pub(super) fn visit(&self, visitor: &mut Visitor) -> Result<(), KeyValueError> {
        match *self {
            Inner::Internal(ref value) => value.visit(visitor),
            Inner::Debug(ref value) => visitor.debug(value),
        }
    }
}

/// An internal structure-preserving value.
pub(super) trait Visit {
    fn visit(&self, backend: &mut Visitor) -> Result<(), KeyValueError>;
}

/// The internal serialization contract.
pub(super) trait Visitor {
    fn debug(&mut self, v: &fmt::Debug) -> Result<(), KeyValueError>;

    fn u64(&mut self, v: u64) -> Result<(), KeyValueError>;
    fn i64(&mut self, v: i64) -> Result<(), KeyValueError>;
    fn f64(&mut self, v: f64) -> Result<(), KeyValueError>;
    fn bool(&mut self, v: bool) -> Result<(), KeyValueError>;
    fn char(&mut self, v: char) -> Result<(), KeyValueError>;
    fn str(&mut self, v: &str) -> Result<(), KeyValueError>;
    fn none(&mut self) -> Result<(), KeyValueError>;
}

/// A visitor for `std::fmt`.
pub(super) struct FmtVisitor<'a, 'b: 'a>(pub(super) &'a mut fmt::Formatter<'b>);

impl<'a, 'b: 'a> Visitor for FmtVisitor<'a, 'b> {
    fn debug(&mut self, v: &fmt::Debug) -> Result<(), KeyValueError> {
        v.fmt(self.0)?;

        Ok(())
    }

    fn u64(&mut self, v: u64) -> Result<(), KeyValueError> {
        self.debug(&format_args!("{:?}", v))
    }

    fn i64(&mut self, v: i64) -> Result<(), KeyValueError> {
        self.debug(&format_args!("{:?}", v))
    }

    fn f64(&mut self, v: f64) -> Result<(), KeyValueError> {
        self.debug(&format_args!("{:?}", v))
    }

    fn bool(&mut self, v: bool) -> Result<(), KeyValueError> {
        self.debug(&format_args!("{:?}", v))
    }

    fn char(&mut self, v: char) -> Result<(), KeyValueError> {
        self.debug(&format_args!("{:?}", v))
    }

    fn str(&mut self, v: &str) -> Result<(), KeyValueError> {
        self.debug(&format_args!("{:?}", v))
    }

    fn none(&mut self) -> Result<(), KeyValueError> {
        self.debug(&format_args!("None"))
    }
}
