// Copyright 2014-2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
/// The standard logging macro.
///
/// This macro will generically log with the specified `LogLevel` and `format!`
/// based argument list.
///
/// The `log_level` cfg can be used to statically disable logging at various
/// levels.
#[macro_export]
macro_rules! log {
    (target: $target:expr, $lvl:expr, $($arg:tt)+) => ({
        static LOC: $crate::LogLocation = $crate::LogLocation {
            __line: line!(),
            __file: file!(),
            __module_path: module_path!(),
        };
        let lvl = $lvl;
        if !cfg!(log_level = "off") &&
                (lvl <= $crate::LogLevel::Error || !cfg!(log_level = "error")) &&
                (lvl <= $crate::LogLevel::Warn || !cfg!(log_level = "warn")) &&
                (lvl <= $crate::LogLevel::Debug || !cfg!(log_level = "debug")) &&
                (lvl <= $crate::LogLevel::Info || !cfg!(log_level = "info")) &&
                lvl <= $crate::max_log_level() {
            $crate::__log(lvl, $target, &LOC, format_args!($($arg)+))
        }
    });
    ($lvl:expr, $($arg:tt)+) => (log!(target: module_path!(), $lvl, $($arg)+))
}

/// Logs a message at the error level.
///
/// Logging at this level is disabled if the `log_level = "off"` cfg is
/// present.
#[macro_export]
macro_rules! error {
    (target: $target:expr, $($arg:tt)*) => (
        log!(target: $target, $crate::LogLevel::Error, $($arg)*);
    );
    ($($arg:tt)*) => (
        log!($crate::LogLevel::Error, $($arg)*);
    )
}

/// Logs a message at the warn level.
///
/// Logging at this level is disabled if any of the following cfgs are present:
/// `log_level = "off"` or `log_level = "error"`.
#[macro_export]
macro_rules! warn {
    (target: $target:expr, $($arg:tt)*) => (
        log!(target: $target, $crate::LogLevel::Warn, $($arg)*);
    );
    ($($arg:tt)*) => (
        log!($crate::LogLevel::Warn, $($arg)*);
    )
}

/// Logs a message at the info level.
///
/// Logging at this level is disabled if any of the following cfgs are present:
/// `log_level = "off"`, `log_level = "error"`, or
/// `log_level = "warn"`.
#[macro_export]
macro_rules! info {
    (target: $target:expr, $($arg:tt)*) => (
        log!(target: $target, $crate::LogLevel::Info, $($arg)*);
    );
    ($($arg:tt)*) => (
        log!($crate::LogLevel::Info, $($arg)*);
    )
}

/// Logs a message at the debug level.
///
/// Logging at this level is disabled if any of the following cfgs are present:
/// `log_level = "off"`, `log_level = "error"`, `log_level = "warn"`,
/// or `log_level = "info"`.
#[macro_export]
macro_rules! debug {
    (target: $target:expr, $($arg:tt)*) => (
        log!(target: $target, $crate::LogLevel::Debug, $($arg)*);
    );
    ($($arg:tt)*) => (
        log!($crate::LogLevel::Debug, $($arg)*);
    )
}

/// Logs a message at the trace level.
///
/// Logging at this level is disabled if any of the following cfgs are present:
/// `log_level = "off"`, `log_level = "error"`, `log_level = "warn"`,
/// `log_level = "info"`, or `log_level = "debug"`.
#[macro_export]
macro_rules! trace {
    (target: $target:expr, $($arg:tt)*) => (
        log!(target: $target, $crate::LogLevel::Trace, $($arg)*);
    );
    ($($arg:tt)*) => (
        log!($crate::LogLevel::Trace, $($arg)*);
    )
}

/// Determines if a message logged at the specified level in that module will
/// be logged.
///
/// This can be used to avoid expensive computation of log message arguments if
/// the message would be ignored anyway.
///
/// # Examples
///
/// ```rust
/// # #[macro_use]
/// # extern crate log;
/// use log::LogLevel::Debug;
///
/// # fn foo() {
/// if log_enabled!(Debug) {
///     debug!("expensive debug data: {}", expensive_call());
/// }
/// # }
/// # fn expensive_call() -> u32 { 0 }
/// # fn main() {}
/// ```
#[macro_export]
macro_rules! log_enabled {
    (target: $target:expr, $lvl:expr) => ({
        let lvl = $lvl;
        !cfg!(log_level = "off") &&
            (lvl <= $crate::LogLevel::Error || !cfg!(log_level = "error")) &&
            (lvl <= $crate::LogLevel::Warn || !cfg!(log_level = "warn")) &&
            (lvl <= $crate::LogLevel::Debug || !cfg!(log_level = "debug")) &&
            (lvl <= $crate::LogLevel::Info || !cfg!(log_level = "info")) &&
            lvl <= $crate::max_log_level() &&
            $crate::__enabled(lvl, $target)
    });
    ($lvl:expr) => (log_enabled!(target: module_path!(), $lvl))
}
