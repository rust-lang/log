//! Coerce a `Value` into some concrete types.
//!
//! These operations are cheap when the captured value is a simple primitive,
//! but may end up executing arbitrary caller code if the value is complex.

use std::fmt;

use super::{Inner, Primitive, Visitor};
use crate::kv;
use crate::kv::value::Error;

impl<'v> kv::Value<'v> {
    /// Try coerce the value into a borrowed string.
    pub fn get_str(&self) -> Option<&str> {
        self.inner.coerce().into_primitive().into_str()
    }

    /// Try coerce the value into a `u8`.
    pub fn get_u8(&self) -> Option<u8> {
        self.inner
            .coerce()
            .into_primitive()
            .into_u64()
            .map(|v| v as u8)
    }

    /// Try coerce the value into a `u16`.
    pub fn get_u16(&self) -> Option<u16> {
        self.inner
            .coerce()
            .into_primitive()
            .into_u64()
            .map(|v| v as u16)
    }

    /// Try coerce the value into a `u32`.
    pub fn get_u32(&self) -> Option<u32> {
        self.inner
            .coerce()
            .into_primitive()
            .into_u64()
            .map(|v| v as u32)
    }

    /// Try coerce the value into a `u64`.
    pub fn get_u64(&self) -> Option<u64> {
        self.inner.coerce().into_primitive().into_u64()
    }

    /// Try coerce the value into a `i8`.
    pub fn get_i8(&self) -> Option<i8> {
        self.inner
            .coerce()
            .into_primitive()
            .into_i64()
            .map(|v| v as i8)
    }

    /// Try coerce the value into a `i16`.
    pub fn get_i16(&self) -> Option<i16> {
        self.inner
            .coerce()
            .into_primitive()
            .into_i64()
            .map(|v| v as i16)
    }

    /// Try coerce the value into a `i32`.
    pub fn get_i32(&self) -> Option<i32> {
        self.inner
            .coerce()
            .into_primitive()
            .into_i64()
            .map(|v| v as i32)
    }

    /// Try coerce the value into a `i64`.
    pub fn get_i64(&self) -> Option<i64> {
        self.inner.coerce().into_primitive().into_i64()
    }

    /// Try coerce the value into a `f32`.
    pub fn get_f32(&self) -> Option<f32> {
        self.inner
            .coerce()
            .into_primitive()
            .into_f64()
            .map(|v| v as f32)
    }

    /// Try coerce the value into a `f64`.
    pub fn get_f64(&self) -> Option<f64> {
        self.inner.coerce().into_primitive().into_f64()
    }

    /// Try coerce the value into a `char`.
    pub fn get_char(&self) -> Option<char> {
        self.inner.coerce().into_primitive().into_char()
    }

    /// Try coerce the value into a `bool`.
    pub fn get_bool(&self) -> Option<bool> {
        self.inner.coerce().into_primitive().into_bool()
    }
}

impl<'v> Inner<'v> {
    fn coerce(&self) -> Coerced {
        struct Coerce<'v>(Coerced<'v>);

        impl<'v> Coerce<'v> {
            fn new() -> Self {
                Coerce(Coerced::Primitive(Primitive::None))
            }
        }

        impl<'v> Visitor<'v> for Coerce<'v> {
            fn debug(&mut self, _: &dyn fmt::Debug) -> Result<(), Error> {
                Ok(())
            }

            fn u64(&mut self, v: u64) -> Result<(), Error> {
                self.0 = Coerced::Primitive(Primitive::Unsigned(v));
                Ok(())
            }

            fn i64(&mut self, v: i64) -> Result<(), Error> {
                self.0 = Coerced::Primitive(Primitive::Signed(v));
                Ok(())
            }

            fn f64(&mut self, v: f64) -> Result<(), Error> {
                self.0 = Coerced::Primitive(Primitive::Float(v));
                Ok(())
            }

            fn bool(&mut self, v: bool) -> Result<(), Error> {
                self.0 = Coerced::Primitive(Primitive::Bool(v));
                Ok(())
            }

            fn char(&mut self, v: char) -> Result<(), Error> {
                self.0 = Coerced::Primitive(Primitive::Char(v));
                Ok(())
            }

            fn borrowed_str(&mut self, v: &'v str) -> Result<(), Error> {
                self.0 = Coerced::Primitive(Primitive::Str(v));
                Ok(())
            }

            #[cfg(not(feature = "std"))]
            fn str(&mut self, _: &str) -> Result<(), Error> {
                Ok(())
            }

            #[cfg(feature = "std")]
            fn str(&mut self, v: &str) -> Result<(), Error> {
                self.0 = Coerced::String(v.into());
                Ok(())
            }

            fn none(&mut self) -> Result<(), Error> {
                self.0 = Coerced::Primitive(Primitive::None);
                Ok(())
            }

            #[cfg(feature = "kv_unstable_sval")]
            fn sval(&mut self, v: &dyn super::sval::Value) -> Result<(), Error> {
                self.0 = super::sval::coerce(v);
                Ok(())
            }
        }

        let mut coerce = Coerce::new();
        let _ = self.visit(&mut coerce);
        coerce.0
    }
}

pub(super) enum Coerced<'v> {
    Primitive(Primitive<'v>),
    #[cfg(feature = "std")]
    String(String),
}

impl<'v> Coerced<'v> {
    fn into_primitive(self) -> Primitive<'v> {
        match self {
            Coerced::Primitive(value) => value,
            #[cfg(feature = "std")]
            _ => Primitive::None,
        }
    }
}

impl<'v> Primitive<'v> {
    fn into_str(self) -> Option<&'v str> {
        if let Primitive::Str(value) = self {
            Some(value)
        } else {
            None
        }
    }

    fn into_u64(self) -> Option<u64> {
        if let Primitive::Unsigned(value) = self {
            Some(value)
        } else {
            None
        }
    }

    fn into_i64(self) -> Option<i64> {
        if let Primitive::Signed(value) = self {
            Some(value)
        } else {
            None
        }
    }

    fn into_f64(self) -> Option<f64> {
        if let Primitive::Float(value) = self {
            Some(value)
        } else {
            None
        }
    }

    fn into_char(self) -> Option<char> {
        if let Primitive::Char(value) = self {
            Some(value)
        } else {
            None
        }
    }

    fn into_bool(self) -> Option<bool> {
        if let Primitive::Bool(value) = self {
            Some(value)
        } else {
            None
        }
    }
}

#[cfg(feature = "std")]
mod std_support {
    use super::*;

    use std::borrow::Cow;

    impl<'v> kv::Value<'v> {
        /// Try coerce the value into an owned or borrowed string.
        pub fn get_string(&self) -> Option<Cow<str>> {
            self.inner.coerce().into_string()
        }
    }

    impl<'v> Coerced<'v> {
        pub(super) fn into_string(self) -> Option<Cow<'v, str>> {
            match self {
                Coerced::Primitive(Primitive::Str(value)) => Some(value.into()),
                Coerced::String(value) => Some(value.into()),
                _ => None,
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::kv::ToValue;

        #[test]
        fn primtive_coercion() {
            assert_eq!(
                "a string",
                "a string"
                    .to_owned()
                    .to_value()
                    .get_str()
                    .expect("invalid value")
            );
            assert_eq!(
                "a string",
                &*"a string".to_value().get_string().expect("invalid value")
            );
            assert_eq!(
                "a string",
                &*"a string"
                    .to_owned()
                    .to_value()
                    .get_string()
                    .expect("invalid value")
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::kv::ToValue;

    #[test]
    fn primitive_coercion() {
        assert_eq!(
            "a string",
            "a string".to_value().get_str().expect("invalid value")
        );
        assert_eq!(
            "a string",
            Some("a string")
                .to_value()
                .get_str()
                .expect("invalid value")
        );

        assert_eq!(1u8, 1u64.to_value().get_u8().expect("invalid value"));
        assert_eq!(1u16, 1u64.to_value().get_u16().expect("invalid value"));
        assert_eq!(1u32, 1u64.to_value().get_u32().expect("invalid value"));
        assert_eq!(1u64, 1u64.to_value().get_u64().expect("invalid value"));

        assert_eq!(-1i8, -1i64.to_value().get_i8().expect("invalid value"));
        assert_eq!(-1i16, -1i64.to_value().get_i16().expect("invalid value"));
        assert_eq!(-1i32, -1i64.to_value().get_i32().expect("invalid value"));
        assert_eq!(-1i64, -1i64.to_value().get_i64().expect("invalid value"));

        assert!(1f32.to_value().get_f32().is_some(), "invalid value");
        assert!(1f64.to_value().get_f64().is_some(), "invalid value");

        assert_eq!('a', 'a'.to_value().get_char().expect("invalid value"));
        assert_eq!(true, true.to_value().get_bool().expect("invalid value"));
    }
}
