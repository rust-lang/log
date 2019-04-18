use std::fmt;

use super::KeyValueError;

// `Backend` is an internal visitor for the structure of a value.
// Right now we only have an implementation for `std::fmt`, but
// this trait makes it possible to add more structured backends like
// `serde` that can retain some of that original structure.
// 
// `Backend` isn't expected to be public, so that `log` can hide its
// internal serialization contract. This keeps its public API small
// and gives us room to move while the API is still evolving.
// For a public API, see the `FromAny` type.

pub(super) trait Backend {
    fn fmt(&mut self, v: fmt::Arguments) -> Result<(), KeyValueError>;
    fn u64(&mut self, v: u64) -> Result<(), KeyValueError>;
    fn i64(&mut self, v: i64) -> Result<(), KeyValueError>;
    fn f64(&mut self, v: f64) -> Result<(), KeyValueError>;
    fn bool(&mut self, v: bool) -> Result<(), KeyValueError>;
    fn char(&mut self, v: char) -> Result<(), KeyValueError>;
    fn str(&mut self, v: &str) -> Result<(), KeyValueError>;
    fn none(&mut self) -> Result<(), KeyValueError>;
}

pub(super) struct FmtBackend<'a, 'b: 'a>(pub(super) &'a mut fmt::Formatter<'b>);

impl<'a, 'b: 'a> Backend for FmtBackend<'a, 'b> {
    fn fmt(&mut self, v: fmt::Arguments) -> Result<(), KeyValueError> {
        fmt::Debug::fmt(&v, self.0)?;

        Ok(())
    }

    fn u64(&mut self, v: u64) -> Result<(), KeyValueError> {
        self.fmt(format_args!("{:?}", v))
    }

    fn i64(&mut self, v: i64) -> Result<(), KeyValueError> {
        self.fmt(format_args!("{:?}", v))
    }

    fn f64(&mut self, v: f64) -> Result<(), KeyValueError> {
        self.fmt(format_args!("{:?}", v))
    }

    fn bool(&mut self, v: bool) -> Result<(), KeyValueError> {
        self.fmt(format_args!("{:?}", v))
    }

    fn char(&mut self, v: char) -> Result<(), KeyValueError> {
        self.fmt(format_args!("{:?}", v))
    }

    fn str(&mut self, v: &str) -> Result<(), KeyValueError> {
        self.fmt(format_args!("{:?}", v))
    }

    fn none(&mut self) -> Result<(), KeyValueError> {
        self.fmt(format_args!("{:?}", Option::None::<()>))
    }
}
