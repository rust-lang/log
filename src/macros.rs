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
/// This macro will generically log with the specified `Level` and `format!`
/// based argument list.
///
/// The `max_level_*` build features can be used to statically disable logging at
/// various levels. For instance, `max_level_error` disables log messages below `Error`,
/// `max_level_info` allows for `Error`, `Warn` and `Info` while `max_level_off` disables logging all together.
///
/// When building in release mode (i.e., without the `debug_assertions` option),
/// the additional `release_max_level_*` build features with identical semantics take precedence.
/// For instance, `release_max_level_debug` allows for all messages except for `Trace`
/// while `release_max_level_trace` allows for all messages of logging levels.
///
/// # Examples
///
/// ```rust
/// # #[macro_use]
/// # extern crate log;
/// use log::Level;
///
/// # fn main() {
/// let data = (42, "Forty-two");
/// let private_data = "private";
///
/// log!(Level::Error, "Received errors: {}, {}", data.0, data.1);
/// log!(target: "app_events", Level::Warn, "App warning: {}, {}, {}",
///     data.0, data.1, private_data);
/// # }
/// ```
#[macro_export]
macro_rules! log {
    (target: $target:expr, $lvl:expr, $($arg:tt)+) => ({
        let lvl = $lvl;
        // Warning: This code is duplicated in `log!`/`error!`/`warn!`/... -
        // see discussion in #54. Once rust-lang/rust#25003 is fixed, this may
        // no longer be necessary.
        if lvl <= $crate::STATIC_MAX_LEVEL && lvl <= $crate::max_level() {
            $crate::Log::log(
                $crate::logger(),
                &$crate::RecordBuilder::new()
                    .args(format_args!($($arg)+))
                    .level(lvl)
                    .target($target)
                    .module_path(module_path!())
                    .file(file!())
                    .line(line!())
                    .build()
            )
        }
    });
    ($lvl:expr, $($arg:tt)+) => (log!(target: module_path!(), $lvl, $($arg)+))
}

/// Logs a message at the error level.
///
/// Logging at this level is disabled if the `max_level_off` feature is present.
///
/// # Examples
///
/// ```rust
/// # #[macro_use]
/// # extern crate log;
/// # fn main() {
/// let (err_info, port) = ("No connection", 22);
///
/// error!("Error: {} on port {}", err_info, port);
/// error!(target: "app_events", "App Error: {}, Port: {}", err_info, 22);
/// # }
/// ```
#[macro_export]
macro_rules! error {
    (target: $target:expr, $($arg:tt)*) => (
        let lvl = $crate::Level::Error;
        // Warning: This code is duplicated in `log!`/`error!`/`warn!`/...
        if lvl <= $crate::STATIC_MAX_LEVEL && lvl <= $crate::max_level() {
            $crate::Log::log(
                $crate::logger(),
                &$crate::RecordBuilder::new()
                    .args(format_args!($($arg)+))
                    .level(lvl)
                    .target($target)
                    .module_path(module_path!())
                    .file(file!())
                    .line(line!())
                    .build()
            )
        }
    );
    ($($arg:tt)*) => (
        error!(target: module_path!(), $($arg)*);
    )
}

/// Logs a message at the warn level.
///
/// Logging at this level is disabled if any of the following features are
/// present: `max_level_off` or `max_level_error`.
///
/// When building in release mode (i.e., without the `debug_assertions` option),
/// logging at this level is also disabled if any of the following features are
/// present: `release_max_level_off` or `max_level_error`.
///
/// # Examples
///
/// ```rust
/// # #[macro_use]
/// # extern crate log;
/// # fn main() {
/// let warn_description = "Invalid Input";
///
/// warn!("Warning! {}!", warn_description);
/// warn!(target: "input_events", "App received warning: {}", warn_description);
/// # }
/// ```
#[macro_export]
macro_rules! warn {
    (target: $target:expr, $($arg:tt)*) => (
        let lvl = $crate::Level::Warn;
        // Warning: This code is duplicated in `log!`/`error!`/`warn!`/...
        if lvl <= $crate::STATIC_MAX_LEVEL && lvl <= $crate::max_level() {
            $crate::Log::log(
                $crate::logger(),
                &$crate::RecordBuilder::new()
                    .args(format_args!($($arg)+))
                    .level(lvl)
                    .target($target)
                    .module_path(module_path!())
                    .file(file!())
                    .line(line!())
                    .build()
            )
        }
    );
    ($($arg:tt)*) => (
        warn!(target: module_path!(), $($arg)*);
    )
}

/// Logs a message at the info level.
///
/// Logging at this level is disabled if any of the following features are
/// present: `max_level_off`, `max_level_error`, or `max_level_warn`.
///
/// When building in release mode (i.e., without the `debug_assertions` option),
/// logging at this level is also disabled if any of the following features are
/// present: `release_max_level_off`, `release_max_level_error`, or
/// `release_max_level_warn`.
///
/// # Examples
///
/// ```rust
/// # #[macro_use]
/// # extern crate log;
/// # fn main() {
/// # struct Connection { port: u32, speed: f32 }
/// let conn_info = Connection { port: 40, speed: 3.20 };
///
/// info!("Connected to port {} at {} Mb/s", conn_info.port, conn_info.speed);
/// info!(target: "connection_events", "Successfull connection, port: {}, speed: {}",
///       conn_info.port, conn_info.speed);
/// # }
/// ```
#[macro_export]
macro_rules! info {
    (target: $target:expr, $($arg:tt)*) => (
        let lvl = $crate::Level::Info;
        // Warning: This code is duplicated in `log!`/`error!`/`warn!`/...
        if lvl <= $crate::STATIC_MAX_LEVEL && lvl <= $crate::max_level() {
            $crate::Log::log(
                $crate::logger(),
                &$crate::RecordBuilder::new()
                    .args(format_args!($($arg)+))
                    .level(lvl)
                    .target($target)
                    .module_path(module_path!())
                    .file(file!())
                    .line(line!())
                    .build()
            )
        }
    );
    ($($arg:tt)*) => (
        info!(target: module_path!(), $($arg)*);
    )
}

/// Logs a message at the debug level.
///
/// Logging at this level is disabled if any of the following features are
/// present: `max_level_off`, `max_level_error`, `max_level_warn`, or
/// `max_level_info`.
///
/// When building in release mode (i.e., without the `debug_assertions` option),
/// logging at this level is also disabled if any of the following features are
/// present: `release_max_level_off`, `release_max_level_error`,
/// `release_max_level_warn`, or `release_max_level_info`.
///
/// # Examples
///
/// ```rust
/// # #[macro_use]
/// # extern crate log;
/// # fn main() {
/// # struct Position { x: f32, y: f32 }
/// let pos = Position { x: 3.234, y: -1.223 };
///
/// debug!("New position: x: {}, y: {}", pos.x, pos.y);
/// debug!(target: "app_events", "New position: x: {}, y: {}", pos.x, pos.y);
/// # }
/// ```
#[macro_export]
macro_rules! debug {
    (target: $target:expr, $($arg:tt)*) => (
        let lvl = $crate::Level::Debug;
        // Warning: This code is duplicated in `log!`/`error!`/`warn!`/...
        if lvl <= $crate::STATIC_MAX_LEVEL && lvl <= $crate::max_level() {
            $crate::Log::log(
                $crate::logger(),
                &$crate::RecordBuilder::new()
                    .args(format_args!($($arg)+))
                    .level(lvl)
                    .target($target)
                    .module_path(module_path!())
                    .file(file!())
                    .line(line!())
                    .build()
            )
        }
    );
    ($($arg:tt)*) => (
        debug!(target: module_path!(), $($arg)*);
    )
}

/// Logs a message at the trace level.
///
/// Logging at this level is disabled if any of the following features are
/// present: `max_level_off`, `max_level_error`, `max_level_warn`,
/// `max_level_info`, or `max_level_debug`.
///
/// When building in release mode (i.e., without the `debug_assertions` option),
/// logging at this level is also disabled if any of the following features are
/// present: `release_max_level_off`, `release_max_level_error`,
/// `release_max_level_warn`, `release_max_level_info`, or
/// `release_max_level_debug`.
///
/// # Examples
///
/// ```rust
/// # #[macro_use]
/// # extern crate log;
/// # fn main() {
/// # struct Position { x: f32, y: f32 }
/// let pos = Position { x: 3.234, y: -1.223 };
///
/// trace!("Position is: x: {}, y: {}", pos.x, pos.y);
/// trace!(target: "app_events", "x is {} and y is {}",
///        if pos.x >= 0.0 { "positive" } else { "negative" },
///        if pos.y >= 0.0 { "positive" } else { "negative" });
/// # }
/// ```
#[macro_export]
macro_rules! trace {
    (target: $target:expr, $($arg:tt)*) => (
        let lvl = $crate::Level::Trace;
        // Warning: This code is duplicated in `log!`/`error!`/`warn!`/...
        if lvl <= $crate::STATIC_MAX_LEVEL && lvl <= $crate::max_level() {
            $crate::Log::log(
                $crate::logger(),
                &$crate::RecordBuilder::new()
                    .args(format_args!($($arg)+))
                    .level(lvl)
                    .target($target)
                    .module_path(module_path!())
                    .file(file!())
                    .line(line!())
                    .build()
            )
        }
    );
    ($($arg:tt)*) => (
        trace!(target: module_path!(), $($arg)*);
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
/// use log::Level::Debug;
///
/// # fn foo() {
/// if log_enabled!(Debug) {
///     let data = expensive_call();
///     debug!("expensive debug data: {} {}", data.x, data.y);
/// }
/// if log_enabled!(target: "Global", Debug) {
///    let data = expensive_call();
///    debug!(target: "Global", "expensive debug data: {} {}", data.x, data.y);
/// }
/// # }
/// # struct Data { x: u32, y: u32 }
/// # fn expensive_call() -> Data { Data { x: 0, y: 0 } }
/// # fn main() {}
/// ```
#[macro_export]
macro_rules! log_enabled {
    (target: $target:expr, $lvl:expr) => ({
        let lvl = $lvl;
        lvl <= $crate::STATIC_MAX_LEVEL && lvl <= $crate::max_level() &&
            $crate::Log::enabled(
                $crate::logger(),
                &$crate::MetadataBuilder::new()
                    .level(lvl)
                    .target($target)
                    .build(),
            )
    });
    ($lvl:expr) => (log_enabled!(target: module_path!(), $lvl))
}
