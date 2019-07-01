use std::fmt;

use super::{Error, ToValue, Value, Visit, Visitor};

impl ToValue for usize {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for usize {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), Error> {
        visitor.u64(*self as u64)
    }
}

impl ToValue for isize {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for isize {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), Error> {
        visitor.i64(*self as i64)
    }
}

impl ToValue for u8 {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for u8 {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), Error> {
        visitor.u64(*self as u64)
    }
}

impl ToValue for u16 {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for u16 {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), Error> {
        visitor.u64(*self as u64)
    }
}

impl ToValue for u32 {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for u32 {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), Error> {
        visitor.u64(*self as u64)
    }
}

impl ToValue for u64 {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for u64 {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), Error> {
        visitor.u64(*self)
    }
}

impl ToValue for i8 {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for i8 {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), Error> {
        visitor.i64(*self as i64)
    }
}

impl ToValue for i16 {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for i16 {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), Error> {
        visitor.i64(*self as i64)
    }
}

impl ToValue for i32 {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for i32 {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), Error> {
        visitor.i64(*self as i64)
    }
}

impl ToValue for i64 {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for i64 {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), Error> {
        visitor.i64(*self)
    }
}

impl ToValue for f32 {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for f32 {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), Error> {
        visitor.f64(*self as f64)
    }
}

impl ToValue for f64 {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for f64 {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), Error> {
        visitor.f64(*self)
    }
}

impl ToValue for bool {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for bool {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), Error> {
        visitor.bool(*self)
    }
}

impl ToValue for char {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for char {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), Error> {
        visitor.char(*self)
    }
}

impl<'v> ToValue for &'v str {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl<'v> Visit for &'v str {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), Error> {
        visitor.str(*self)
    }
}

impl ToValue for () {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for () {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), Error> {
        visitor.none()
    }
}

impl<T> ToValue for Option<T>
where
    T: ToValue,
{
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl<T> Visit for Option<T>
where
    T: ToValue,
{
    fn visit(&self, visitor: &mut Visitor) -> Result<(), Error> {
        match *self {
            Some(ref value) => value.to_value().visit(visitor),
            None => visitor.none(),
        }
    }
}

impl<'v> ToValue for fmt::Arguments<'v> {
    fn to_value(&self) -> Value {
        Value::from_debug(self)
    }
}

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
            Value::from_internal(self)
        }
    }

    impl Visit for String {
        fn visit(&self, visitor: &mut Visitor) -> Result<(), Error> {
            visitor.str(&*self)
        }
    }

    impl<'a> ToValue for Cow<'a, str> {
        fn to_value(&self) -> Value {
            Value::from_internal(self)
        }
    }

    impl<'a> Visit for Cow<'a, str> {
        fn visit(&self, visitor: &mut Visitor) -> Result<(), Error> {
            visitor.str(&*self)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kv::value::test::Token;

    #[test]
    fn test_to_value_display() {
        assert_eq!(42u64.to_value().to_str_buf(), "42");
        assert_eq!(42i64.to_value().to_str_buf(), "42");
        assert_eq!(42.01f64.to_value().to_str_buf(), "42.01");
        assert_eq!(true.to_value().to_str_buf(), "true");
        assert_eq!('a'.to_value().to_str_buf(), "'a'");
        assert_eq!(format_args!("a {}", "value").to_value().to_str_buf(), "a value");
        assert_eq!("a loong string".to_value().to_str_buf(), "\"a loong string\"");
        assert_eq!(Some(true).to_value().to_str_buf(), "true");
        assert_eq!(().to_value().to_str_buf(), "None");
        assert_eq!(Option::None::<bool>.to_value().to_str_buf(), "None");
    }

    #[test]
    fn test_to_value_structured() {
        assert_eq!(42u64.to_value().to_token(), Token::U64(42));
        assert_eq!(42i64.to_value().to_token(), Token::I64(42));
        assert_eq!(42.01f64.to_value().to_token(), Token::F64(42.01));
        assert_eq!(true.to_value().to_token(), Token::Bool(true));
        assert_eq!('a'.to_value().to_token(), Token::Char('a'));
        assert_eq!(format_args!("a {}", "value").to_value().to_token(), Token::Str("a value".into()));
        assert_eq!("a loong string".to_value().to_token(), Token::Str("a loong string".into()));
        assert_eq!(Some(true).to_value().to_token(), Token::Bool(true));
        assert_eq!(().to_value().to_token(), Token::None);
        assert_eq!(Option::None::<bool>.to_value().to_token(), Token::None);
    }
}
