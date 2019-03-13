use std::{fmt, mem};
use std::marker::PhantomData;

use super::{ToValue, KeyValueError};
use super::backend::Backend;

/// A function for converting some type `T` into a [`Value`](struct.Value.html).
pub type FromAnyFn<T> = fn(FromAny, &T) -> Result<(), KeyValueError>;

/// A helper for converting any type into a [`Value`](struct.Value.html).
pub struct FromAny<'a>(&'a mut Backend);

impl<'a> FromAny<'a> {
    pub(super) fn value<T>(self, v: T) -> Result<(), KeyValueError>
    where
        T: ToValue,
    {
        v.to_value().inner.visit(self.0)
    }

    /// Convert a formattable type into a value.
    pub fn debug<T>(self, v: T) -> Result<(), KeyValueError>
    where
        T: fmt::Debug
    {
        self.0.fmt(format_args!("{:?}", v))
    }

    /// Convert a `u64` into a value.
    pub fn u64(self, v: u64) -> Result<(), KeyValueError> {
        self.0.u64(v)
    }

    /// Convert a `i64` into a value.
    pub fn i64(self, v: i64) -> Result<(), KeyValueError> {
        self.0.i64(v)
    }
    
    /// Convert a `f64` into a value.
    pub fn f64(self, v: f64) -> Result<(), KeyValueError> {
        self.0.f64(v)
    }

    /// Convert a `bool` into a value.
    pub fn bool(self, v: bool) -> Result<(), KeyValueError> {
        self.0.bool(v)
    }

    /// Convert a `char` into a value.
    pub fn char(self, v: char) -> Result<(), KeyValueError> {
        self.0.char(v)
    }

    /// Convert an empty type into a value.
    pub fn none(self) -> Result<(), KeyValueError> {
        self.0.none()
    }

    /// Convert a string into a value.
    pub fn str(self, v: &str) -> Result<(), KeyValueError> {
        self.0.str(v)
    }
}

// `Any<'a>` is very similar to `std::fmt::Arguments<'a>`
// It takes a &T and fn pointer and stores them in an erased structure.
// It's a bit like an ad-hoc trait object that can accept any arbitrary
// value without those values needing to implement any traits.

#[derive(Clone, Copy)]
pub(super) struct Any<'a> {
    data: &'a Void,
    from: FromAnyFn<Void>,
}

// FIXME: This would be more correct as an extern type
// Replace once the `extern_types` feature is stable
// and available
struct Void {
    _priv: (),
    _oibit_remover: PhantomData<*mut Fn()>,
}

impl<'a> Any<'a> {
    pub(super) fn new<T>(data: &'a T, from: FromAnyFn<T>) -> Self {
        unsafe {
            Any {
                data: mem::transmute::<&'a T, &'a Void>(data),
                from: mem::transmute::<FromAnyFn<T>, FromAnyFn<Void>>(from),
            }
        }
    }

    pub(super) fn visit(&self, backend: &mut Backend) -> Result<(), KeyValueError> {
        (self.from)(FromAny(backend), self.data)
    }
}
