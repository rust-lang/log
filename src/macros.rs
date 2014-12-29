// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Logging macros

#![macro_escape]

/// The standard logging macro
///
/// This macro will generically log over a provided level (of type u32) with a
/// format!-based argument list. See documentation in `std::fmt` for details on
/// how to use the syntax.
///
/// # Example
///
/// ```
/// #![feature(phase)]
/// #[phase(plugin, link)] extern crate log;
///
/// fn main() {
///     log!(log::WARN, "this is a warning {}", "message");
///     log!(log::DEBUG, "this is a debug message");
///     log!(6, "this is a custom logging level: {level}", level=6u);
/// }
/// ```
///
/// Assumes the binary is `main`:
///
/// ```{.bash}
/// $ RUST_LOG=warn ./main
/// WARN:main: this is a warning message
/// ```
///
/// ```{.bash}
/// $ RUST_LOG=debug ./main
/// DEBUG:main: this is a debug message
/// WARN:main: this is a warning message
/// ```
///
/// ```{.bash}
/// $ RUST_LOG=6 ./main
/// DEBUG:main: this is a debug message
/// WARN:main: this is a warning message
/// 6:main: this is a custom logging level: 6
/// ```
#[macro_export]
macro_rules! log {
    ($lvl:expr, $($arg:tt)+) => ({
        static LOC: ::log::LogLocation = ::log::LogLocation {
            line: line!(),
            file: file!(),
            module_path: module_path!(),
        };
        let lvl = $lvl;
        if log_enabled!(lvl) {
            ::log::log(lvl, &LOC, &format_args!($($arg)+))
        }
    })
}

/// A convenience macro for logging at the error log level.
///
/// # Example
///
/// ```
/// #![feature(phase)]
/// #[phase(plugin, link)] extern crate log;
///
/// fn main() {
///     let error = 3u;
///     error!("the build has failed with error code: {}", error);
/// }
/// ```
///
/// Assumes the binary is `main`:
///
/// ```{.bash}
/// $ RUST_LOG=error ./main
/// ERROR:main: the build has failed with error code: 3
/// ```
///
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => (log!(::log::ERROR, $($arg)*))
}

/// A convenience macro for logging at the warning log level.
///
/// # Example
///
/// ```
/// #![feature(phase)]
/// #[phase(plugin, link)] extern crate log;
///
/// fn main() {
///     let code = 3u;
///     warn!("you may like to know that a process exited with: {}", code);
/// }
/// ```
///
/// Assumes the binary is `main`:
///
/// ```{.bash}
/// $ RUST_LOG=warn ./main
/// WARN:main: you may like to know that a process exited with: 3
/// ```
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => (log!(::log::WARN, $($arg)*))
}

/// A convenience macro for logging at the info log level.
///
/// # Example
///
/// ```
/// #![feature(phase)]
/// #[phase(plugin, link)] extern crate log;
///
/// fn main() {
///     let ret = 3i;
///     info!("this function is about to return: {}", ret);
/// }
/// ```
///
/// Assumes the binary is `main`:
///
/// ```{.bash}
/// $ RUST_LOG=info ./main
/// INFO:main: this function is about to return: 3
/// ```
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => (log!(::log::INFO, $($arg)*))
}

/// A convenience macro for logging at the debug log level. This macro can also
/// be omitted at compile time by passing `--cfg ndebug` to the compiler. If
/// this option is not passed, then debug statements will be compiled.
///
/// # Example
///
/// ```
/// #![feature(phase)]
/// #[phase(plugin, link)] extern crate log;
///
/// fn main() {
///     debug!("x = {x}, y = {y}", x=10i, y=20i);
/// }
/// ```
///
/// Assumes the binary is `main`:
///
/// ```{.bash}
/// $ RUST_LOG=debug ./main
/// DEBUG:main: x = 10, y = 20
/// ```
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => (if cfg!(not(ndebug)) { log!(::log::DEBUG, $($arg)*) })
}

/// A macro to test whether a log level is enabled for the current module.
///
/// # Example
///
/// ```
/// #![feature(phase)]
/// #[phase(plugin, link)] extern crate log;
///
/// struct Point { x: int, y: int }
/// fn some_expensive_computation() -> Point { Point { x: 1, y: 2 } }
///
/// fn main() {
///     if log_enabled!(log::DEBUG) {
///         let x = some_expensive_computation();
///         debug!("x.x = {}, x.y = {}", x.x, x.y);
///     }
/// }
/// ```
///
/// Assumes the binary is `main`:
///
/// ```{.bash}
/// $ RUST_LOG=error ./main
/// ```
///
/// ```{.bash}
/// $ RUST_LOG=debug ./main
/// DEBUG:main: x.x = 1, x.y = 2
/// ```
#[macro_export]
macro_rules! log_enabled {
    ($lvl:expr) => ({
        let lvl = $lvl;
        (lvl != ::log::DEBUG || cfg!(not(ndebug))) &&
        lvl <= ::log::log_level() &&
        ::log::mod_enabled(lvl, module_path!())
    })
}
