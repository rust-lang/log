use self::sealed::{LogArgs, LogKVs, LogLevel, LogTarget};
use crate::{Level, LevelFilter, Metadata, Record};
use std::cmp::Ordering;
pub use std::convert::identity;
use std::fmt::Arguments;
use std::option::Option;
pub use std::primitive::str;
pub use std::{file, format_args, line, module_path, stringify};

#[cfg(feature = "kv_unstable")]
pub type LogKvValue<'a> = dyn crate::kv::value::ToValue + 'a;

#[cfg(not(feature = "kv_unstable"))]
pub type LogKvValue<'a> = str;

mod sealed {
    use crate::Level;
    use std::fmt::Arguments;

    pub trait LogLevel {
        fn into_log_level(self) -> Level;
    }

    pub trait LogArgs {
        fn with(self, f: impl FnOnce(Arguments));
    }

    pub trait LogTarget {
        fn with(self, module_path: &'static str, f: impl FnOnce(&str));
    }

    pub trait LogKVs {
        fn with(self, f: impl FnOnce(&[(&str, &super::LogKvValue)]));
    }
}

// `LogLevel`.

impl LogLevel for Level {
    #[inline]
    fn into_log_level(self) -> Level {
        self
    }
}

macro_rules! define_static_levels {
    ($($ty:ident => $lvl:ident,)*) => {
        $(
            #[derive(Debug)]
            pub struct $ty;

            impl LogLevel for $ty {
                #[inline]
                fn into_log_level(self) -> Level {
                    Level::$lvl
                }
            }

            impl PartialEq<LevelFilter> for $ty {
                #[inline]
                fn eq(&self, other: &LevelFilter) -> bool {
                    Level::$lvl.eq(other)
                }
            }

            impl PartialOrd<LevelFilter> for $ty {
                #[inline]
                fn partial_cmp(&self, other: &LevelFilter) -> Option<Ordering> {
                    Level::$lvl.partial_cmp(other)
                }

                #[inline]
                fn lt(&self, other: &LevelFilter) -> bool {
                    Level::$lvl.lt(other)
                }

                #[inline]
                fn le(&self, other: &LevelFilter) -> bool {
                    Level::$lvl.le(other)
                }

                #[inline]
                fn gt(&self, other: &LevelFilter) -> bool {
                    Level::$lvl.gt(other)
                }

                #[inline]
                fn ge(&self, other: &LevelFilter) -> bool {
                    Level::$lvl.ge(other)
                }
            }
        )*
    };
}

define_static_levels![
    StaticLevelError => Error,
    StaticLevelWarn => Warn,
    StaticLevelInfo => Info,
    StaticLevelDebug => Debug,
    StaticLevelTrace => Trace,
];

// `LogArgs`.

impl LogArgs for &str {
    #[inline]
    fn with(self, f: impl FnOnce(Arguments)) {
        f(format_args!("{self}"))
    }
}

impl LogArgs for Arguments<'_> {
    fn with(self, f: impl FnOnce(Arguments)) {
        f(self)
    }
}

// `LogTarget`.

impl LogTarget for &str {
    #[inline]
    fn with(self, _module_path: &'static str, f: impl FnOnce(&str)) {
        f(self)
    }
}

#[derive(Debug)]
pub struct TargetIsModulePath;

impl LogTarget for TargetIsModulePath {
    #[inline]
    fn with(self, module_path: &'static str, f: impl FnOnce(&str)) {
        f(module_path)
    }
}

// `LogKVs`.

impl LogKVs for &[(&str, &LogKvValue<'_>)] {
    #[inline]
    fn with(self, f: impl FnOnce(&[(&str, &LogKvValue)])) {
        f(self)
    }
}

#[derive(Debug)]
pub struct EmptyKVs;

impl LogKVs for EmptyKVs {
    #[inline]
    fn with(self, f: impl FnOnce(&[(&str, &LogKvValue)])) {
        f(&[])
    }
}

// Log functions.

fn log_0(
    &(module_path, file): &'static (&'static str, &'static str),
    line: u32,
    level: Level,
    args: Arguments,
    target: &str,
    kvs: &[(&str, &LogKvValue)],
) {
    #[cfg(not(feature = "kv_unstable"))]
    if !kvs.is_empty() {
        panic!(
            "key-value support is experimental and must be enabled using the `kv_unstable` feature"
        );
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

fn log_1<K>(
    module_path_and_file: &'static (&'static str, &'static str),
    line: u32,
    level: Level,
    args: Arguments,
    target: &str,
    kvs: K,
) where
    K: LogKVs,
{
    kvs.with(|kvs| log_0(module_path_and_file, line, level, args, target, kvs));
}

fn log_2<T, K>(
    module_path_and_file: &'static (&'static str, &'static str),
    line: u32,
    level: Level,
    args: Arguments,
    target: T,
    kvs: K,
) where
    T: LogTarget,
    K: LogKVs,
{
    target.with(module_path_and_file.0, |target| {
        log_1(module_path_and_file, line, level, args, target, kvs)
    });
}

pub fn log_3<A, T, K>(
    module_path_and_file: &'static (&'static str, &'static str),
    line: u32,
    level: Level,
    args: A,
    target: T,
    kvs: K,
) where
    A: LogArgs,
    T: LogTarget,
    K: LogKVs,
{
    args.with(|args| log_2(module_path_and_file, line, level, args, target, kvs))
}

pub fn log<L, A, T, K>(
    module_path_and_file: &'static (&'static str, &'static str),
    line: u32,
    level: L,
    args: A,
    target: T,
    kvs: K,
) where
    L: LogLevel,
    A: LogArgs,
    T: LogTarget,
    K: LogKVs,
{
    log_3(
        module_path_and_file,
        line,
        level.into_log_level(),
        args,
        target,
        kvs,
    )
}

pub fn enabled(level: Level, target: &str) -> bool {
    crate::logger().enabled(&Metadata::builder().level(level).target(target).build())
}

pub const fn is_literal(s: &str) -> bool {
    let s = s.as_bytes();
    let n = s.len();
    let mut i = 0;

    while i < n {
        if matches!(s[i], b'{' | b'}') {
            return false;
        }

        i += 1;
    }

    true
}

pub fn unused() -> usize {
    fn for_all_kvs<L, A, T>() -> usize
    where
        L: LogLevel,
        A: LogArgs,
        T: LogTarget,
    {
        type K0 = &'static [(&'static str, &'static LogKvValue<'static>)];
        type K1 = EmptyKVs;

        log::<L, A, T, K0> as usize ^ log::<L, A, T, K1> as usize
    }

    fn for_all_targets<L, A>() -> usize
    where
        L: LogLevel,
        A: LogArgs,
    {
        type T0 = &'static str;
        type T1 = TargetIsModulePath;

        for_all_kvs::<L, A, T0>() ^ for_all_kvs::<L, A, T1>()
    }

    fn for_all_arguments<L>() -> usize
    where
        L: LogLevel,
    {
        type A0 = &'static str;
        type A1 = Arguments<'static>;

        for_all_targets::<L, A0>() ^ for_all_targets::<L, A1>()
    }

    [
        for_all_arguments::<Level>(),
        for_all_arguments::<StaticLevelError>(),
        for_all_arguments::<StaticLevelWarn>(),
        for_all_arguments::<StaticLevelInfo>(),
        for_all_arguments::<StaticLevelDebug>(),
        for_all_arguments::<StaticLevelTrace>(),
    ]
    .iter()
    .fold(0, |x, y| x ^ y)
}
