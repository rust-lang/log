//! Converting standard types into `Value`s.
//!
//! This module provides `ToValue` implementations for commonly
//! logged types from the standard library.

use std::fmt;

use super::{Primitive, ToValue, Value};

impl<'v> ToValue for &'v str {
    fn to_value(&self) -> Value {
        Value::from(*self)
    }
}

impl<'v> From<&'v str> for Value<'v> {
    fn from(value: &'v str) -> Self {
        Value::from_primitive(value)
    }
}

impl<'v> ToValue for fmt::Arguments<'v> {
    fn to_value(&self) -> Value {
        Value::from(*self)
    }
}

impl<'v> From<fmt::Arguments<'v>> for Value<'v> {
    fn from(value: fmt::Arguments<'v>) -> Self {
        Value::from_primitive(value)
    }
}

impl ToValue for () {
    fn to_value(&self) -> Value {
        Value::from_primitive(Primitive::None)
    }
}

impl<T> ToValue for Option<T>
where
    T: ToValue,
{
    fn to_value(&self) -> Value {
        match *self {
            Some(ref value) => value.to_value(),
            None => Value::from_primitive(Primitive::None),
        }
    }
}

macro_rules! impl_to_value_primitive {
    ($($into_ty:ty,)*) => {
        $(
            impl ToValue for $into_ty {
                fn to_value(&self) -> Value {
                    Value::from(*self)
                }
            }

            impl<'v> From<$into_ty> for Value<'v> {
                fn from(value: $into_ty) -> Self {
                    Value::from_primitive(value)
                }
            }
        )*
    };
}

impl_to_value_primitive! [
    usize,
    u8,
    u16,
    u32,
    u64,

    isize,
    i8,
    i16,
    i32,
    i64,

    f32,
    f64,

    char,
    bool,
];

#[cfg(feature = "std")]
mod std_support {
    use super::*;

    use std::borrow::Cow;

    impl<T> ToValue for Box<T>
    where
        T: ToValue + ?Sized,
    {
        fn to_value(&self) -> Value {
            (**self).to_value()
        }
    }

    impl ToValue for String {
        fn to_value(&self) -> Value {
            Value::from_primitive(Primitive::Str(&*self))
        }
    }

    impl<'v> ToValue for Cow<'v, str> {
        fn to_value(&self) -> Value {
            Value::from_primitive(Primitive::Str(&*self))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kv::value::test::Token;

    #[test]
    fn test_to_value_display() {
        assert_eq!(42u64.to_value().to_string(), "42");
        assert_eq!(42i64.to_value().to_string(), "42");
        assert_eq!(42.01f64.to_value().to_string(), "42.01");
        assert_eq!(true.to_value().to_string(), "true");
        assert_eq!('a'.to_value().to_string(), "'a'");
        assert_eq!(
            format_args!("a {}", "value").to_value().to_string(),
            "a value"
        );
        assert_eq!(
            "a loong string".to_value().to_string(),
            "\"a loong string\""
        );
        assert_eq!(Some(true).to_value().to_string(), "true");
        assert_eq!(().to_value().to_string(), "None");
        assert_eq!(Option::None::<bool>.to_value().to_string(), "None");
    }

    #[test]
    fn test_to_value_structured() {
        assert_eq!(42u64.to_value().to_token(), Token::U64(42));
        assert_eq!(42i64.to_value().to_token(), Token::I64(42));
        assert_eq!(42.01f64.to_value().to_token(), Token::F64(42.01));
        assert_eq!(true.to_value().to_token(), Token::Bool(true));
        assert_eq!('a'.to_value().to_token(), Token::Char('a'));
        assert_eq!(
            format_args!("a {}", "value").to_value().to_token(),
            Token::Str("a value".into())
        );
        assert_eq!(
            "a loong string".to_value().to_token(),
            Token::Str("a loong string".into())
        );
        assert_eq!(Some(true).to_value().to_token(), Token::Bool(true));
        assert_eq!(().to_value().to_token(), Token::None);
        assert_eq!(Option::None::<bool>.to_value().to_token(), Token::None);
    }
}
