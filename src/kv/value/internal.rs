use std::fmt;

use super::{Fill, Slot, Error};
use kv;

// `Visit` and `Visitor` is an internal API for visiting the structure of a value.
// It's not intended to be public (at this stage).
//
// Right now we only have an implementation for `std::fmt`, but
// this trait makes it possible to add more structured backends like
// `serde` that can retain that original structure.

/// A container for a structured value for a specific kind of visitor.
#[derive(Clone, Copy)]
pub(super) enum Inner<'v> {
    /// A simple primitive value that can be copied without allocating.
    Primitive(Primitive<'v>),
    /// A value that can be filled.
    Fill(&'v Fill),
    /// A debuggable value.
    Debug(&'v fmt::Debug),
    /// A displayable value.
    Display(&'v fmt::Display),
}

impl<'v> Inner<'v> {
    pub(super) fn visit(&self, visitor: &mut Visitor) -> Result<(), Error> {
        match *self {
            Inner::Primitive(value) => match value {
                Primitive::Signed(value) => visitor.i64(value),
                Primitive::Unsigned(value) => visitor.u64(value),
                Primitive::Float(value) => visitor.f64(value),
                Primitive::Bool(value) => visitor.bool(value),
                Primitive::Char(value) => visitor.char(value),
                Primitive::Str(value) => visitor.str(value),
                Primitive::None => visitor.none(),
            },
            Inner::Fill(value) => value.fill(&mut Slot::new(visitor)),
            Inner::Debug(value) => visitor.debug(value),
            Inner::Display(value) => visitor.display(value),
        }
    }
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

mod fmt_support {
    use super::*;

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
}
