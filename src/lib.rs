// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A lightweight logging facade.
//!
//! A logging facade provides a single logging API that abstracts over the
//! actual logging implementation. Libraries can use the logging API provided
//! by this crate, and the consumer of those libraries can choose the logging
//! framework that is most suitable for its use case.
//!
//! If no logging implementation is selected, the facade falls back to a "noop"
//! implementation that ignores all log messages. The overhead in this case
//! is very small - just an integer load, comparison and jump.
//!
//! A log request consists of a target, a level, and a body. A target is a
//! string which defaults to the module path of the location of the log
//! request, though that default may be overridden. Logger implementations
//! typically use the target to filter requests based on some user
//! configuration.
//!
//! # Use
//!
//! ## In libraries
//!
//! Libraries should link only to the `log` crate, and use the provided
//! macros to log whatever information will be useful to downstream consumers.
//!
//! ### Examples
//!
//! ```rust
//! # #![allow(unstable)]
//! #[macro_use]
//! extern crate log;
//!
//! # #[derive(Debug)] pub struct Yak(String);
//! # impl Yak { fn shave(&self, _: u32) {} }
//! # fn find_a_razor() -> Result<u32, u32> { Ok(1) }
//! pub fn shave_the_yak(yak: &Yak) {
//!     info!(target: "yak_events", "Commencing yak shaving for {:?}", yak);
//!
//!     loop {
//!         match find_a_razor() {
//!             Ok(razor) => {
//!                 info!("Razor located: {}", razor);
//!                 yak.shave(razor);
//!                 break;
//!             }
//!             Err(err) => {
//!                 warn!("Unable to locate a razor: {}, retrying", err);
//!             }
//!         }
//!     }
//! }
//! # fn main() {}
//! ```
//!
//! ## In executables
//!
//! Executables should choose a logging framework and initialize it early in the
//! runtime of the program. Logging frameworks will typically include a
//! function to do this. Any log messages generated before the framework is
//! initialized will be ignored.
//!
//! The executable itself may use the `log` crate to log as well.
//!
//! ### Warning
//!
//! The logging system may only be initialized once.
//!
//! ### Examples
//!
//! ```rust,ignore
//! #[macro_use]
//! extern crate log;
//! extern crate my_logger;
//!
//! fn main() {
//!     my_logger::init();
//!
//!     info!("starting up");
//!
//!     // ...
//! }
//! ```
//!
//! # Logger implementations
//!
//! Loggers implement the `Log` trait. Here's a very basic example that simply
//! logs all messages at the `Error`, `Warn` or `Info` levels to stdout:
//!
//! ```rust
//! extern crate log;
//!
//! use log::{LogRecord, LogLevel, LogMetadata};
//!
//! struct SimpleLogger;
//!
//! impl log::Log for SimpleLogger {
//!     fn enabled(&self, metadata: &LogMetadata) -> bool {
//!         metadata.level() <= LogLevel::Info
//!     }
//!
//!     fn log(&self, record: &LogRecord) {
//!         if self.enabled(record.metadata()) {
//!             println!("{} - {}", record.level(), record.args());
//!         }
//!     }
//! }
//!
//! # fn main() {}
//! ```
//!
//! Loggers are installed by calling the `set_logger` function. It takes a
//! closure which is provided a `MaxLogLevel` token and returns a `Log` trait
//! object. The `MaxLogLevel` token controls the global maximum log level. The
//! logging facade uses this as an optimization to improve performance of log
//! messages at levels that are disabled. In the case of our example logger,
//! we'll want to set the maximum log level to `Info`, since we ignore any
//! `Debug` or `Trace` level log messages. A logging framework should provide a
//! function that wraps a call to `set_logger`, handling initialization of the
//! logger:
//!
//! ```rust
//! # extern crate log;
//! # use log::{LogLevel, LogLevelFilter, SetLoggerError, LogMetadata};
//! # struct SimpleLogger;
//! # impl log::Log for SimpleLogger {
//! #   fn enabled(&self, _: &LogMetadata) -> bool { false }
//! #   fn log(&self, _: &log::LogRecord) {}
//! # }
//! # fn main() {}
//! pub fn init() -> Result<(), SetLoggerError> {
//!     log::set_logger(|max_log_level| {
//!         max_log_level.set(LogLevelFilter::Info);
//!         Box::new(SimpleLogger)
//!     })
//! }
//! ```

#![doc(html_logo_url = "https://www.rust-lang.org/logos/rust-logo-128x128-blk-v2.png",
       html_favicon_url = "https://www.rust-lang.org/favicon.ico",
       html_root_url = "https://doc.rust-lang.org/log/")]
#![warn(missing_docs)]
#![cfg_attr(feature = "nightly", feature(panic_handler))]

extern crate libc;

extern crate log_core;

// Re-export most of log_core, but not set_logger_raw and shutdown_logger
pub use log_core::{LogLevel, LogLevelFilter, LogRecord, LogMetadata, Log,
                   LogLocation, MaxLogLevelFilter, max_log_level, __enabled,
                   __log, __static_max_level};

#[macro_use]
#[path = "../core/src/macros.rs"]
mod macros;

use std::error;
use std::fmt;

/// Sets the global logger.
///
/// The `make_logger` closure is passed a `MaxLogLevel` object, which the
/// logger should use to keep the global maximum log level in sync with the
/// highest log level that the logger will not ignore.
///
/// This function may only be called once in the lifetime of a program. Any log
/// events that occur before the call to `set_logger` completes will be
/// ignored.
///
/// This function does not typically need to be called manually. Logger
/// implementations should provide an initialization method that calls
/// `set_logger` internally.
pub fn set_logger<M>(make_logger: M) -> Result<(), SetLoggerError>
        where M: FnOnce(MaxLogLevelFilter) -> Box<Log> {
    let result = unsafe {
        log_core::set_logger_raw(|max_level| Box::into_raw(make_logger(max_level)))
    };

    return match result {
        Ok(()) => {
            assert_eq!(unsafe { libc::atexit(shutdown) }, 0);
            Ok(())
        }
        Err(_) => Err(SetLoggerError(())),
    };

    extern fn shutdown() {
        log_core::shutdown_logger().map(|logger| unsafe {
            Box::from_raw(logger as *mut Log);
        }).ok();
    }
}

/// The type returned by `set_logger` if `set_logger` has already been called.
#[allow(missing_copy_implementations)]
#[derive(Debug)]
pub struct SetLoggerError(());

impl fmt::Display for SetLoggerError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "attempted to set a logger after the logging system \
                     was already initialized")
    }
}

impl error::Error for SetLoggerError {
    fn description(&self) -> &str { "set_logger() called multiple times" }
}

/// Registers a panic handler which logs at the error level.
///
/// The format is the same as the default panic handler. The reporting module is
/// `log::panic`.
///
/// Requires the `nightly` feature.
#[cfg(feature = "nightly")]
pub fn log_panics() {
    std::panic::set_handler(panic::log);
}

// inner module so that the reporting module is log::panic instead of log
#[cfg(feature = "nightly")]
mod panic {
    use std::panic::PanicInfo;
    use std::thread;

    pub fn log(info: &PanicInfo) {
        let thread = thread::current();
        let thread = thread.name().unwrap_or("<unnamed>");

        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<Any>",
            }
        };

        match info.location() {
            Some(location) => {
                error!("thread '{}' panicked at '{}': {}:{}",
                       thread,
                       msg,
                       location.file(),
                       location.line())
            }
            None => error!("thread '{}' panicked at '{}'", thread, msg),
        }
    }
}

#[cfg(test)]
mod tests {
     use std::error::Error;
     use super::{LogLevel, LogLevelFilter, SetLoggerError};

     #[test]
     fn test_loglevelfilter_from_str() {
         let tests = [
             ("off",   Ok(LogLevelFilter::Off)),
             ("error", Ok(LogLevelFilter::Error)),
             ("warn",  Ok(LogLevelFilter::Warn)),
             ("info",  Ok(LogLevelFilter::Info)),
             ("debug", Ok(LogLevelFilter::Debug)),
             ("trace", Ok(LogLevelFilter::Trace)),
             ("OFF",   Ok(LogLevelFilter::Off)),
             ("ERROR", Ok(LogLevelFilter::Error)),
             ("WARN",  Ok(LogLevelFilter::Warn)),
             ("INFO",  Ok(LogLevelFilter::Info)),
             ("DEBUG", Ok(LogLevelFilter::Debug)),
             ("TRACE", Ok(LogLevelFilter::Trace)),
             ("asdf",  Err(())),
         ];
         for &(s, ref expected) in &tests {
             assert_eq!(expected, &s.parse());
         }
     }

     #[test]
     fn test_loglevel_from_str() {
         let tests = [
             ("OFF",   Err(())),
             ("error", Ok(LogLevel::Error)),
             ("warn",  Ok(LogLevel::Warn)),
             ("info",  Ok(LogLevel::Info)),
             ("debug", Ok(LogLevel::Debug)),
             ("trace", Ok(LogLevel::Trace)),
             ("ERROR", Ok(LogLevel::Error)),
             ("WARN",  Ok(LogLevel::Warn)),
             ("INFO",  Ok(LogLevel::Info)),
             ("DEBUG", Ok(LogLevel::Debug)),
             ("TRACE", Ok(LogLevel::Trace)),
             ("asdf",  Err(())),
         ];
         for &(s, ref expected) in &tests {
             assert_eq!(expected, &s.parse());
         }
     }

     #[test]
     fn test_loglevel_show() {
         assert_eq!("INFO", LogLevel::Info.to_string());
         assert_eq!("ERROR", LogLevel::Error.to_string());
     }

     #[test]
     fn test_loglevelfilter_show() {
         assert_eq!("OFF", LogLevelFilter::Off.to_string());
         assert_eq!("ERROR", LogLevelFilter::Error.to_string());
     }

     #[test]
     fn test_cross_cmp() {
         assert!(LogLevel::Debug > LogLevelFilter::Error);
         assert!(LogLevelFilter::Warn < LogLevel::Trace);
         assert!(LogLevelFilter::Off < LogLevel::Error);
     }

     #[test]
     fn test_cross_eq() {
         assert!(LogLevel::Error == LogLevelFilter::Error);
         assert!(LogLevelFilter::Off != LogLevel::Error);
         assert!(LogLevel::Trace == LogLevelFilter::Trace);
     }

     #[test]
     fn test_to_log_level() {
         assert_eq!(Some(LogLevel::Error), LogLevelFilter::Error.to_log_level());
         assert_eq!(None, LogLevelFilter::Off.to_log_level());
         assert_eq!(Some(LogLevel::Debug), LogLevelFilter::Debug.to_log_level());
     }

     #[test]
     fn test_to_log_level_filter() {
         assert_eq!(LogLevelFilter::Error, LogLevel::Error.to_log_level_filter());
         assert_eq!(LogLevelFilter::Trace, LogLevel::Trace.to_log_level_filter());
     }

     #[test]
     fn test_error_trait() {
         let e = SetLoggerError(());
         assert_eq!(e.description(), "set_logger() called multiple times");
     }
}
