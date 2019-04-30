use std::fmt;

use super::Error;

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
    /// A debuggable value.
    Debug(&'v fmt::Debug),
    /// A displayable value.
    Display(&'v fmt::Display),
}

impl<'v> Inner<'v> {
    pub(super) fn visit(&self, visitor: &mut Visitor) -> Result<(), Error> {
        match *self {
            Inner::Internal(ref value) => value.visit(visitor),
            Inner::Debug(ref value) => visitor.debug(value),
            Inner::Display(ref value) => visitor.display(value),
        }
    }
}

/// An internal structure-preserving value.
pub(super) trait Visit {
    fn visit(&self, backend: &mut Visitor) -> Result<(), Error>;
}

/// The internal serialization contract.
pub(super) trait Visitor {
    fn debug(&mut self, v: &fmt::Debug) -> Result<(), Error>;
    fn display(&mut self, v: &fmt::Display) -> Result<(), Error> {
        self.debug(&format_args!("{}",  v))
    }

    fn u64(&mut self, v: u64) -> Result<(), Error>;
    fn i64(&mut self, v: i64) -> Result<(), Error>;
    fn f64(&mut self, v: f64) -> Result<(), Error>;
    fn bool(&mut self, v: bool) -> Result<(), Error>;
    fn char(&mut self, v: char) -> Result<(), Error>;
    fn str(&mut self, v: &str) -> Result<(), Error>;
    fn none(&mut self) -> Result<(), Error>;
}

/// A visitor for `std::fmt`.
pub(super) struct FmtVisitor<'a, 'b: 'a>(pub(super) &'a mut fmt::Formatter<'b>);

impl<'a, 'b: 'a> Visitor for FmtVisitor<'a, 'b> {
    fn debug(&mut self, v: &fmt::Debug) -> Result<(), Error> {
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
}
