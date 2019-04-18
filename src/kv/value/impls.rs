use std::fmt;

use super::{KeyValueError, ToValue, Value, Visit, Visitor};

impl ToValue for u8 {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for u8 {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), KeyValueError> {
        visitor.u64(*self as u64)
    }
}

impl ToValue for u16 {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for u16 {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), KeyValueError> {
        visitor.u64(*self as u64)
    }
}

impl ToValue for u32 {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for u32 {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), KeyValueError> {
        visitor.u64(*self as u64)
    }
}

impl ToValue for u64 {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for u64 {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), KeyValueError> {
        visitor.u64(*self)
    }
}

impl ToValue for i8 {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for i8 {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), KeyValueError> {
        visitor.i64(*self as i64)
    }
}

impl ToValue for i16 {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for i16 {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), KeyValueError> {
        visitor.i64(*self as i64)
    }
}

impl ToValue for i32 {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for i32 {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), KeyValueError> {
        visitor.i64(*self as i64)
    }
}

impl ToValue for i64 {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for i64 {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), KeyValueError> {
        visitor.i64(*self)
    }
}

impl ToValue for f32 {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for f32 {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), KeyValueError> {
        visitor.f64(*self as f64)
    }
}

impl ToValue for f64 {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for f64 {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), KeyValueError> {
        visitor.f64(*self)
    }
}

impl ToValue for bool {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for bool {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), KeyValueError> {
        visitor.bool(*self)
    }
}

impl ToValue for char {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for char {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), KeyValueError> {
        visitor.char(*self)
    }
}

impl<'v> ToValue for &'v str {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl<'v> Visit for &'v str {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), KeyValueError> {
        visitor.str(*self)
    }
}

impl ToValue for () {
    fn to_value(&self) -> Value {
        Value::from_internal(self)
    }
}

impl Visit for () {
    fn visit(&self, visitor: &mut Visitor) -> Result<(), KeyValueError> {
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
    fn visit(&self, visitor: &mut Visitor) -> Result<(), KeyValueError> {
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
        fn visit(&self, visitor: &mut Visitor) -> Result<(), KeyValueError> {
            visitor.str(&*self)
        }
    }

    impl<'a> ToValue for Cow<'a, str> {
        fn to_value(&self) -> Value {
            Value::from_internal(self)
        }
    }

    impl<'a> Visit for Cow<'a, str> {
        fn visit(&self, visitor: &mut Visitor) -> Result<(), KeyValueError> {
            visitor.str(&*self)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kv::value::KeyValueError;
    use kv::value::internal::Visitor;

    use std::fmt::Write;
    use std::str::{self, Utf8Error};

    // A quick-and-dirty no-std buffer
    // to write strings into
    struct Buffer {
        buf: [u8; 16],
        len: usize,
    }

    impl Buffer {
        fn new() -> Self {
            Buffer {
                buf: [0; 16],
                len: 0,
            }
        }

        fn as_str(&self) -> Result<&str, Utf8Error> {
            str::from_utf8(&self.buf[0..self.len])
        }
    }

    impl Write for Buffer {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            let bytes = s.as_bytes();

            let end = self.len + bytes.len();

            if end > 16 {
                panic!("`{}` would overflow", s);
            }

            let buf = &mut self.buf[self.len..end];
            buf.copy_from_slice(bytes);
            self.len = end;

            Ok(())
        }
    }

    #[test]
    fn test_to_value_display() {
        // Write a value into our buffer using `<Value as Display>::fmt`
        fn check(value: Value, expected: &str) {
            let mut buf = Buffer::new();
            write!(&mut buf, "{}", value).unwrap();

            assert_eq!(expected, buf.as_str().unwrap());
        }

        check(42u64.to_value(), "42");
        check(42i64.to_value(), "42");
        check(42.01f64.to_value(), "42.01");
        check(true.to_value(), "true");
        check('a'.to_value(), "'a'");
        check(format_args!("a {}", "value").to_value(), "a value");
        check("a loong string".to_value(), "\"a loong string\"");
        check(Some(true).to_value(), "true");
        check(().to_value(), "None");
        check(Option::None::<bool>.to_value(), "None");
    }

    #[test]
    fn test_to_value_structured() {
        #[derive(Debug, PartialEq)]
        enum Token<'a> {
            U64(u64),
            I64(i64),
            F64(f64),
            Char(char),
            Bool(bool),
            Str(&'a str),
            None,
        }

        struct TestVisitor<F>(F);

        impl<F> Visitor for TestVisitor<F>
        where
            F: Fn(Token),
        {
            fn debug(&mut self, v: &fmt::Debug) -> Result<(), KeyValueError> {
                let mut buf = Buffer::new();
                write!(&mut buf, "{:?}", v)?;

                let s = buf.as_str().map_err(|_| KeyValueError::msg("invalid UTF8"))?;
                (self.0)(Token::Str(s));
                Ok(())
            }

            fn u64(&mut self, v: u64) -> Result<(), KeyValueError> {
                (self.0)(Token::U64(v));
                Ok(())
            }

            fn i64(&mut self, v: i64) -> Result<(), KeyValueError> {
                (self.0)(Token::I64(v));
                Ok(())
            }

            fn f64(&mut self, v: f64) -> Result<(), KeyValueError> {
                (self.0)(Token::F64(v));
                Ok(())
            }

            fn bool(&mut self, v: bool) -> Result<(), KeyValueError> {
                (self.0)(Token::Bool(v));
                Ok(())
            }

            fn char(&mut self, v: char) -> Result<(), KeyValueError> {
                (self.0)(Token::Char(v));
                Ok(())
            }

            fn str(&mut self, v: &str) -> Result<(), KeyValueError> {
                (self.0)(Token::Str(v));
                Ok(())
            }

            fn none(&mut self) -> Result<(), KeyValueError> {
                (self.0)(Token::None);
                Ok(())
            }
        }

        // Check that a value retains the right structure
        fn check(value: Value, expected: Token) {
            let mut visitor = TestVisitor(|token: Token| assert_eq!(expected, token));
            value.visit(&mut visitor).unwrap();
        }

        check(42u64.to_value(), Token::U64(42));
        check(42i64.to_value(), Token::I64(42));
        check(42.01f64.to_value(), Token::F64(42.01));
        check(true.to_value(), Token::Bool(true));
        check('a'.to_value(), Token::Char('a'));
        check(format_args!("a {}", "value").to_value(), Token::Str("a value"));
        check("a loong string".to_value(), Token::Str("a loong string"));
        check(Some(true).to_value(), Token::Bool(true));
        check(().to_value(), Token::None);
        check(Option::None::<bool>.to_value(), Token::None);
    }
}
