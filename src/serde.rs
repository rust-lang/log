#![cfg(feature = "serde")]

extern crate serde;
use self::serde::ser::{Serialize, Serializer};
use self::serde::de::{Deserialize, DeserializeSeed, Deserializer, Visitor, EnumAccess,
                      VariantAccess, Error};

use {Level, LevelFilter, LOG_LEVEL_NAMES};

use std::fmt;
use std::str::FromStr;

// The Deserialize impls are handwritten to be case insensitive using FromStr.

impl Serialize for Level {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        match *self {
            Level::Error => serializer.serialize_unit_variant("Level", 0, "ERROR"),
            Level::Warn => serializer.serialize_unit_variant("Level", 1, "WARN"),
            Level::Info => serializer.serialize_unit_variant("Level", 2, "INFO"),
            Level::Debug => serializer.serialize_unit_variant("Level", 3, "DEBUG"),
            Level::Trace => serializer.serialize_unit_variant("Level", 4, "TRACE"),
        }
    }
}

impl<'de> Deserialize<'de> for Level {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        struct LevelIdentifier;

        impl<'de> Visitor<'de> for LevelIdentifier {
            type Value = Level;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("log level")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
                where E: Error
            {
                // Case insensitive.
                FromStr::from_str(s).map_err(|_| Error::unknown_variant(s, &LOG_LEVEL_NAMES[1..]))
            }
        }

        impl<'de> DeserializeSeed<'de> for LevelIdentifier {
            type Value = Level;

            fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
                where D: Deserializer<'de>
            {
                deserializer.deserialize_identifier(LevelIdentifier)
            }
        }

        struct LevelEnum;

        impl<'de> Visitor<'de> for LevelEnum {
            type Value = Level;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("log level")
            }

            fn visit_enum<A>(self, value: A) -> Result<Self::Value, A::Error>
                where A: EnumAccess<'de>
            {
                let (level, variant) = value.variant_seed(LevelIdentifier)?;
                // Every variant is a unit variant.
                variant.unit_variant()?;
                Ok(level)
            }
        }

        deserializer.deserialize_enum("Level", &LOG_LEVEL_NAMES[1..], LevelEnum)
    }
}

impl Serialize for LevelFilter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        match *self {
            LevelFilter::Off => serializer.serialize_unit_variant("LevelFilter", 0, "OFF"),
            LevelFilter::Error => serializer.serialize_unit_variant("LevelFilter", 1, "ERROR"),
            LevelFilter::Warn => serializer.serialize_unit_variant("LevelFilter", 2, "WARN"),
            LevelFilter::Info => serializer.serialize_unit_variant("LevelFilter", 3, "INFO"),
            LevelFilter::Debug => serializer.serialize_unit_variant("LevelFilter", 4, "DEBUG"),
            LevelFilter::Trace => serializer.serialize_unit_variant("LevelFilter", 5, "TRACE"),
        }
    }
}

impl<'de> Deserialize<'de> for LevelFilter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        struct LevelFilterIdentifier;

        impl<'de> Visitor<'de> for LevelFilterIdentifier {
            type Value = LevelFilter;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("log level filter")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
                where E: Error
            {
                // Case insensitive.
                FromStr::from_str(s).map_err(|_| Error::unknown_variant(s, &LOG_LEVEL_NAMES))
            }
        }

        impl<'de> DeserializeSeed<'de> for LevelFilterIdentifier {
            type Value = LevelFilter;

            fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
                where D: Deserializer<'de>
            {
                deserializer.deserialize_identifier(LevelFilterIdentifier)
            }
        }

        struct LevelFilterEnum;

        impl<'de> Visitor<'de> for LevelFilterEnum {
            type Value = LevelFilter;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("log level filter")
            }

            fn visit_enum<A>(self, value: A) -> Result<Self::Value, A::Error>
                where A: EnumAccess<'de>
            {
                let (level_filter, variant) = value.variant_seed(LevelFilterIdentifier)?;
                // Every variant is a unit variant.
                variant.unit_variant()?;
                Ok(level_filter)
            }
        }

        deserializer.deserialize_enum("LevelFilter", &LOG_LEVEL_NAMES, LevelFilterEnum)
    }
}

#[cfg(test)]
mod tests {
    extern crate serde_test;
    use self::serde_test::{Token, assert_tokens, assert_de_tokens, assert_de_tokens_error};

    use {Level, LevelFilter};

    #[test]
    fn test_level_ser_de() {
        let cases = [(Level::Error,
                      [Token::UnitVariant {
                           name: "Level",
                           variant: "ERROR",
                       }]),
                     (Level::Warn,
                      [Token::UnitVariant {
                           name: "Level",
                           variant: "WARN",
                       }]),
                     (Level::Info,
                      [Token::UnitVariant {
                           name: "Level",
                           variant: "INFO",
                       }]),
                     (Level::Debug,
                      [Token::UnitVariant {
                           name: "Level",
                           variant: "DEBUG",
                       }]),
                     (Level::Trace,
                      [Token::UnitVariant {
                           name: "Level",
                           variant: "TRACE",
                       }])];

        for &(s, expected) in &cases {
            assert_tokens(&s, &expected);
        }
    }

    #[test]
    fn test_level_case_insensitive() {
        let cases = [(Level::Error,
                      [Token::UnitVariant {
                           name: "Level",
                           variant: "error",
                       }]),
                     (Level::Warn,
                      [Token::UnitVariant {
                           name: "Level",
                           variant: "warn",
                       }]),
                     (Level::Info,
                      [Token::UnitVariant {
                           name: "Level",
                           variant: "info",
                       }]),
                     (Level::Debug,
                      [Token::UnitVariant {
                           name: "Level",
                           variant: "debug",
                       }]),
                     (Level::Trace,
                      [Token::UnitVariant {
                           name: "Level",
                           variant: "trace",
                       }])];

        for &(s, expected) in &cases {
            assert_de_tokens(&s, &expected);
        }
    }

    #[test]
    fn test_level_de_error() {
        assert_de_tokens_error::<Level>(&[Token::UnitVariant {
                                             name: "Level",
                                             variant: "errorx",
                                         }],
                                        "unknown variant `errorx`, expected one of `ERROR`, `WARN`, `INFO`, `DEBUG`, `TRACE`");
    }

    #[test]
    fn test_level_filter_ser_de() {
        let cases = [(LevelFilter::Off,
                      [Token::UnitVariant {
                           name: "LevelFilter",
                           variant: "OFF",
                       }]),
                     (LevelFilter::Error,
                      [Token::UnitVariant {
                           name: "LevelFilter",
                           variant: "ERROR",
                       }]),
                     (LevelFilter::Warn,
                      [Token::UnitVariant {
                           name: "LevelFilter",
                           variant: "WARN",
                       }]),
                     (LevelFilter::Info,
                      [Token::UnitVariant {
                           name: "LevelFilter",
                           variant: "INFO",
                       }]),
                     (LevelFilter::Debug,
                      [Token::UnitVariant {
                           name: "LevelFilter",
                           variant: "DEBUG",
                       }]),
                     (LevelFilter::Trace,
                      [Token::UnitVariant {
                           name: "LevelFilter",
                           variant: "TRACE",
                       }])];

        for &(s, expected) in &cases {
            assert_tokens(&s, &expected);
        }
    }

    #[test]
    fn test_level_filter_case_insensitive() {
        let cases = [(LevelFilter::Off,
                      [Token::UnitVariant {
                           name: "LevelFilter",
                           variant: "off",
                       }]),
                     (LevelFilter::Error,
                      [Token::UnitVariant {
                           name: "LevelFilter",
                           variant: "error",
                       }]),
                     (LevelFilter::Warn,
                      [Token::UnitVariant {
                           name: "LevelFilter",
                           variant: "warn",
                       }]),
                     (LevelFilter::Info,
                      [Token::UnitVariant {
                           name: "LevelFilter",
                           variant: "info",
                       }]),
                     (LevelFilter::Debug,
                      [Token::UnitVariant {
                           name: "LevelFilter",
                           variant: "debug",
                       }]),
                     (LevelFilter::Trace,
                      [Token::UnitVariant {
                           name: "LevelFilter",
                           variant: "trace",
                       }])];

        for &(s, expected) in &cases {
            assert_de_tokens(&s, &expected);
        }
    }

    #[test]
    fn test_level_filter_de_error() {
        assert_de_tokens_error::<LevelFilter>(&[Token::UnitVariant {
                                                   name: "LevelFilter",
                                                   variant: "errorx",
                                               }],
                                              "unknown variant `errorx`, expected one of `OFF`, `ERROR`, `WARN`, `INFO`, `DEBUG`, `TRACE`");
    }
}
