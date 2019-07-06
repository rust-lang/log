// Test support for inspecting Values

use std::fmt::{self, Write};
use std::str;

use super::{Value, Error};
use super::internal::Visitor;

#[derive(Debug, PartialEq)]
pub(in kv) enum Token {
    U64(u64),
    I64(i64),
    F64(f64),
    Char(char),
    Bool(bool),
    Str(StrBuf),
    None,
}

#[derive(Debug, PartialEq)]
pub(in kv) struct StrBuf {
    buf: [u8; 16],
    len: usize,
}

impl<'a> From<&'a str> for StrBuf {
    fn from(s: &'a str) -> Self {
        let mut buf = Buffer::new();
        write!(&mut buf, "{}", s).unwrap();

        buf.into_str_buf()
    }
}

impl PartialEq<str> for StrBuf {
    fn eq(&self, other: &str) -> bool {
        self.as_ref() == other
    }
}

impl<'a> PartialEq<&'a str> for StrBuf {
    fn eq(&self, other: &&'a str) -> bool {
        self.as_ref() == *other
    }
}

impl<'a> PartialEq<StrBuf> for &'a str {
    fn eq(&self, other: &StrBuf) -> bool {
        *self == other.as_ref()
    }
}

impl AsRef<str> for StrBuf {
    fn as_ref(&self) -> &str {
        str::from_utf8(&self.buf[0..self.len]).unwrap()
    }
}

#[cfg(test)]
impl<'v> Value<'v> {
    pub(in kv) fn to_token(&self) -> Token {
        struct TestVisitor(Option<Token>);

        impl Visitor for TestVisitor {
            fn debug(&mut self, v: &fmt::Debug) -> Result<(), Error> {
                let mut buf = Buffer::new();
                write!(&mut buf, "{:?}", v)?;

                self.0 = Some(Token::Str(buf.into_str_buf()));
                Ok(())
            }

            fn u64(&mut self, v: u64) -> Result<(), Error> {
                self.0 = Some(Token::U64(v));
                Ok(())
            }

            fn i64(&mut self, v: i64) -> Result<(), Error> {
                self.0 = Some(Token::I64(v));
                Ok(())
            }

            fn f64(&mut self, v: f64) -> Result<(), Error> {
                self.0 = Some(Token::F64(v));
                Ok(())
            }

            fn bool(&mut self, v: bool) -> Result<(), Error> {
                self.0 = Some(Token::Bool(v));
                Ok(())
            }

            fn char(&mut self, v: char) -> Result<(), Error> {
                self.0 = Some(Token::Char(v));
                Ok(())
            }

            fn str(&mut self, v: &str) -> Result<(), Error> {
                self.0 = Some(Token::Str(v.into()));
                Ok(())
            }

            fn none(&mut self) -> Result<(), Error> {
                self.0 = Some(Token::None);
                Ok(())
            }
        }

        let mut visitor = TestVisitor(None);
        self.visit(&mut visitor).unwrap();

        visitor.0.unwrap()
    }

    pub(in kv) fn to_str_buf(&self) -> StrBuf {
        let mut buf = Buffer::new();
        write!(&mut buf, "{}", self).unwrap();

        buf.into_str_buf()
    }
}

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

    fn into_str_buf(self) -> StrBuf {
        StrBuf {
            buf: self.buf,
            len: self.len,
        }
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
