//! The internal `Value` serialization API.
//!
//! This implementation isn't intended to be public. It may need to change
//! for optimizations or to support new external serialization frameworks.

use super::{Error, Fill, Slot};

pub(super) mod coerce;
pub(super) mod fmt;
#[cfg(feature = "kv_unstable_sval")]
pub(super) mod sval;

/// A container for a structured value for a specific kind of visitor.
#[derive(Clone, Copy)]
pub(super) enum Inner<'v> {
    /// A simple primitive value that can be copied without allocating.
    Primitive(Primitive<'v>),
    /// A value that can be filled.
    Fill(&'v dyn Fill),
    /// A debuggable value.
    Debug(&'v dyn fmt::Debug),
    /// A displayable value.
    Display(&'v dyn fmt::Display),

    #[cfg(feature = "kv_unstable_sval")]
    /// A structured value from `sval`.
    Sval(&'v dyn sval::Value),
}

impl<'v> Inner<'v> {
    pub(super) fn visit(self, visitor: &mut dyn Visitor<'v>) -> Result<(), Error> {
        match self {
            Inner::Primitive(value) => value.visit(visitor),
            Inner::Fill(value) => value.fill(&mut Slot::new(visitor)),
            Inner::Debug(value) => visitor.debug(value),
            Inner::Display(value) => visitor.display(value),

            #[cfg(feature = "kv_unstable_sval")]
            Inner::Sval(value) => visitor.sval(value),
        }
    }
}

/// The internal serialization contract.
pub(super) trait Visitor<'v> {
    fn debug(&mut self, v: &dyn fmt::Debug) -> Result<(), Error>;
    fn display(&mut self, v: &dyn fmt::Display) -> Result<(), Error> {
        self.debug(&format_args!("{}", v))
    }

    fn u64(&mut self, v: u64) -> Result<(), Error>;
    fn i64(&mut self, v: i64) -> Result<(), Error>;
    fn f64(&mut self, v: f64) -> Result<(), Error>;
    fn bool(&mut self, v: bool) -> Result<(), Error>;
    fn char(&mut self, v: char) -> Result<(), Error>;

    fn str(&mut self, v: &str) -> Result<(), Error>;
    fn borrowed_str(&mut self, v: &'v str) -> Result<(), Error> {
        self.str(v)
    }

    fn none(&mut self) -> Result<(), Error>;

    #[cfg(feature = "kv_unstable_sval")]
    fn sval(&mut self, v: &dyn sval::Value) -> Result<(), Error>;
}

/// A captured primitive value.
///
/// These values are common and cheap to copy around.
#[derive(Clone, Copy)]
pub(super) enum Primitive<'v> {
    Signed(i64),
    Unsigned(u64),
    Float(f64),
    Bool(bool),
    Char(char),
    Str(&'v str),
    None,
}

impl<'v> Primitive<'v> {
    fn visit(self, visitor: &mut dyn Visitor<'v>) -> Result<(), Error> {
        match self {
            Primitive::Signed(value) => visitor.i64(value),
            Primitive::Unsigned(value) => visitor.u64(value),
            Primitive::Float(value) => visitor.f64(value),
            Primitive::Bool(value) => visitor.bool(value),
            Primitive::Char(value) => visitor.char(value),
            Primitive::Str(value) => visitor.borrowed_str(value),
            Primitive::None => visitor.none(),
        }
    }
}
