//! WARNING: this is not part of the crate's public API and is subject to change at any time

use self::sealed::KVs;
use crate::{Level, Metadata, Record};
use std::fmt::Arguments;
pub use std::{format_args, module_path, stringify};

#[cfg(feature = "kv_unstable")]
pub type Value<'a> = dyn crate::kv::value::ToValue + 'a;

#[cfg(not(feature = "kv_unstable"))]
pub type Value<'a> = str;

mod sealed {
    /// Types for the `kv` argument.
    pub trait KVs<'a> {
        fn into_kvs(self) -> Option<&'a [(&'a str, &'a super::Value<'a>)]>;
    }
}

// Types for the `kv` argument.

impl<'a> KVs<'a> for &'a [(&'a str, &'a Value<'a>)] {
    #[inline]
    fn into_kvs(self) -> Option<&'a [(&'a str, &'a Value<'a>)]> {
        Some(self)
    }
}

impl<'a> KVs<'a> for () {
    #[inline]
    fn into_kvs(self) -> Option<&'a [(&'a str, &'a Value<'a>)]> {
        None
    }
}

#[track_caller]
pub fn file<'a>() -> &'a str {
    ::std::panic::Location::caller().file()
}

#[track_caller]
pub fn line() -> u32 {
    ::std::panic::Location::caller().line()
}

// Log implementation.

fn log_impl(
    args: Arguments,
    level: Level,
    &(target, module_path, file): &(&str, &'static str, &'static str),
    line: u32,
    kvs: Option<&[(&str, &Value)]>,
) {
    #[cfg(not(feature = "kv_unstable"))]
    if kvs.is_some() {
        panic!(
            "key-value support is experimental and must be enabled using the `kv_unstable` feature"
        )
    }

    let mut builder = Record::builder();

    builder
        .args(args)
        .level(level)
        .target(target)
        .module_path_static(Some(module_path))
        .file_static(Some(file))
        .line(Some(line));

    #[cfg(feature = "kv_unstable")]
    builder.key_values(&kvs);

    crate::logger().log(&builder.build());
}

pub fn log<'a, K>(
    args: Arguments,
    level: Level,
    target_module_path_and_file: &(&str, &'static str, &'static str),
    line: u32,
    kvs: K,
) where
    K: KVs<'a>,
{
    log_impl(
        args,
        level,
        target_module_path_and_file,
        line,
        kvs.into_kvs(),
    )
}

pub fn enabled(level: Level, target: &str) -> bool {
    crate::logger().enabled(&Metadata::builder().level(level).target(target).build())
}
