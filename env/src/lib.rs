// Copyright 2014-2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A logger configured via an environment variable which writes to standard
//! error.
//!
//! ## Example
//!
//! ```
//! #[macro_use] extern crate log;
//! extern crate env_logger;
//!
//! use log::LogLevel;
//!
//! fn main() {
//!     env_logger::init().unwrap();
//!
//!     debug!("this is a debug {}", "message");
//!     error!("this is printed by default");
//!
//!     if log_enabled!(LogLevel::Info) {
//!         let x = 3 * 4; // expensive computation
//!         info!("the answer was: {}", x);
//!     }
//! }
//! ```
//!
//! Assumes the binary is `main`:
//!
//! ```{.bash}
//! $ RUST_LOG=error ./main
//! ERROR:main: this is printed by default
//! ```
//!
//! ```{.bash}
//! $ RUST_LOG=info ./main
//! ERROR:main: this is printed by default
//! INFO:main: the answer was: 12
//! ```
//!
//! ```{.bash}
//! $ RUST_LOG=debug ./main
//! DEBUG:main: this is a debug message
//! ERROR:main: this is printed by default
//! INFO:main: the answer was: 12
//! ```
//!
//! You can also set the log level on a per module basis:
//!
//! ```{.bash}
//! $ RUST_LOG=main=info ./main
//! ERROR:main: this is printed by default
//! INFO:main: the answer was: 12
//! ```
//!
//! And enable all logging:
//!
//! ```{.bash}
//! $ RUST_LOG=main ./main
//! DEBUG:main: this is a debug message
//! ERROR:main: this is printed by default
//! INFO:main: the answer was: 12
//! ```
//!
//! See the documentation for the log crate for more information about its API.
//!
//! ## Enabling logging
//!
//! Log levels are controlled on a per-module basis, and by default all logging
//! is disabled except for `error!`. Logging is controlled via the `RUST_LOG`
//! environment variable (this can be customized using `init_with_env_var` or
//! `init_with_spec`). The value of this environment variable is a
//! comma-separated list of logging directives. A logging directive is of the
//! form:
//!
//! ```text
//! path::to::module=log_level
//! ```
//!
//! The path to the module is rooted in the name of the crate it was compiled
//! for, so if your program is contained in a file `hello.rs`, for example, to
//! turn on logging for this file you would use a value of `RUST_LOG=hello`.
//! Furthermore, this path is a prefix-search, so all modules nested in the
//! specified module will also have logging enabled.
//!
//! The actual `log_level` is optional to specify. If omitted, all logging will
//! be enabled. If specified, it must be one of the strings `debug`, `error`,
//! `info`, `warn`, or `trace`.
//!
//! As the log level for a module is optional, the module to enable logging for
//! is also optional. If only a `log_level` is provided, then the global log
//! level for all modules is set to this value.
//!
//! Some examples of valid values of `RUST_LOG` are:
//!
//! * `hello` turns on all logging for the 'hello' module
//! * `info` turns on all info logging
//! * `hello=debug` turns on debug logging for 'hello'
//! * `hello,std::option` turns on hello, and std's option logging
//! * `error,hello=warn` turn on global error logging and also warn for hello
//!
//! ## Filtering results
//!
//! A RUST_LOG directive may include a regex filter. The syntax is to append `/`
//! followed by a regex. Each message is checked against the regex, and is only
//! logged if it matches. Note that the matching is done after formatting the
//! log string but before adding any logging meta-data. There is a single filter
//! for all modules.
//!
//! Some examples:
//!
//! * `hello/foo` turns on all logging for the 'hello' module where the log
//!   message includes 'foo'.
//! * `info/f.o` turns on all info logging where the log message includes 'foo',
//!   'f1o', 'fao', etc.
//! * `hello=debug/foo*foo` turns on debug logging for 'hello' where the log
//!   message includes 'foofoo' or 'fofoo' or 'fooooooofoo', etc.
//! * `error,hello=warn/[0-9] scopes` turn on global error logging and also
//!   warn for hello. In both cases the log message must include a single digit
//!   number followed by 'scopes'.

#![doc(html_logo_url = "http://www.rust-lang.org/logos/rust-logo-128x128-blk-v2.png",
       html_favicon_url = "http://www.rust-lang.org/favicon.ico",
       html_root_url = "http://doc.rust-lang.org/env_logger/")]
#![cfg_attr(test, deny(warnings))]

extern crate regex;
extern crate log;

use regex::Regex;
use std::io::prelude::*;
use std::io;
use std::env;

use log::{Log, LogLevel, LogLevelFilter, LogRecord, SetLoggerError, LogMetadata};

struct Logger {
    directives: Vec<LogDirective>,
    filter: Option<Regex>,
}

impl Logger {
    fn enabled(&self, level: LogLevel, target: &str) -> bool {
        // Search for the longest match, the vector is assumed to be pre-sorted.
        for directive in self.directives.iter().rev() {
            match directive.name {
                Some(ref name) if !target.starts_with(&**name) => {},
                Some(..) | None => {
                    return level <= directive.level
                }
            }
        }
        false
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        self.enabled(metadata.level(), metadata.target())
    }

    fn log(&self, record: &LogRecord) {
        if !Log::enabled(self, record.metadata()) {
            return;
        }

        if let Some(filter) = self.filter.as_ref() {
            if filter.is_match(&*record.args().to_string()) {
                return;
            }
        }

        let _ = writeln!(&mut io::stderr(),
                         "{}:{}: {}",
                         record.level(),
                         record.location().module_path(),
                         record.args());
    }
}

struct LogDirective {
    name: Option<String>,
    level: LogLevelFilter,
}

/// Initializes the global logger, with a log spec from the RUST_LOG environment variable.
///
/// This should be called early in the execution of a Rust program, and the
/// global logger may only be initialized once. Future initialization attempts
/// will return an error.
pub fn init() -> Result<(), SetLoggerError> {
    init_with_env_var("RUST_LOG")
}

/// Initializes the global logger, with a log spec from an environment variable.
///
/// This should be called early in the execution of a Rust program, and the
/// global logger may only be initialized once. Future initialization attempts
/// will return an error.
pub fn init_with_env_var(env_var: &str) -> Result<(), SetLoggerError> {
    init_internal(env::var(env_var).ok().as_ref().map(|s| &**s))
}

/// Initializes the global logger with a log spec.
///
/// This should be called early in the execution of a Rust program, and the
/// global logger may only be initialized once. Future initialization attempts
/// will return an error.
pub fn init_with_spec(spec: &str) -> Result<(), SetLoggerError> {
    init_internal(Some(spec))
}

/// Initializes the global logger with a log spec or a default.
fn init_internal(spec: Option<&str>) -> Result<(), SetLoggerError> {
    log::set_logger(|max_level| {
        let (mut directives, filter) = match spec {
            Some(spec) => parse_logging_spec(&spec),
            None => (vec![LogDirective { name: None, level: LogLevelFilter::Error }], None),
        };

        // Sort the provided directives by length of their name, this allows a
        // little more efficient lookup at runtime.
        directives.sort_by(|a, b| {
            let alen = a.name.as_ref().map(|a| a.len()).unwrap_or(0);
            let blen = b.name.as_ref().map(|b| b.len()).unwrap_or(0);
            alen.cmp(&blen)
        });

        let level = {
            let max = directives.iter().map(|d| d.level).max();
            max.unwrap_or(LogLevelFilter::Off)
        };
        max_level.set(level);

        Box::new(Logger {
            directives: directives,
            filter: filter,
        })
    })
}

/// Parse a logging specification string (e.g: "crate1,crate2::mod3,crate3::x=error/foo")
/// and return a vector with log directives.
fn parse_logging_spec(spec: &str) -> (Vec<LogDirective>, Option<Regex>) {
    let mut dirs = Vec::new();

    let mut parts = spec.split('/');
    let mods = parts.next();
    let filter = parts.next();
    if parts.next().is_some() {
        println!("warning: invalid logging spec '{}', \
                 ignoring it (too many '/'s)", spec);
        return (dirs, None);
    }
    mods.map(|m| { for s in m.split(',') {
        if s.len() == 0 { continue }
        let mut parts = s.split('=');
        let (log_level, name) = match (parts.next(), parts.next().map(|s| s.trim()), parts.next()) {
            (Some(part0), None, None) => {
                // if the single argument is a log-level string or number,
                // treat that as a global fallback
                match part0.parse() {
                    Ok(num) => (num, None),
                    Err(_) => (LogLevelFilter::max(), Some(part0)),
                }
            }
            (Some(part0), Some(""), None) => (LogLevelFilter::max(), Some(part0)),
            (Some(part0), Some(part1), None) => {
                match part1.parse() {
                    Ok(num) => (num, Some(part0)),
                    _ => {
                        println!("warning: invalid logging spec '{}', \
                                 ignoring it", part1);
                        continue
                    }
                }
            },
            _ => {
                println!("warning: invalid logging spec '{}', \
                         ignoring it", s);
                continue
            }
        };
        dirs.push(LogDirective {
            name: name.map(|s| s.to_string()),
            level: log_level,
        });
    }});

    let filter = filter.map_or(None, |filter| {
        match Regex::new(filter) {
            Ok(re) => Some(re),
            Err(e) => {
                println!("warning: invalid regex filter - {}", e);
                None
            }
        }
    });

    return (dirs, filter);
}

#[cfg(test)]
mod tests {
    use log::{Log, LogLevel, LogLevelFilter};

    use super::{Logger, LogDirective, parse_logging_spec};

    fn make_logger(dirs: Vec<LogDirective>) -> Logger {
        Logger {
            directives: dirs,
            filter: None,
        }
    }

    #[test]
    fn match_full_path() {
        let logger = make_logger(vec![
            LogDirective {
                name: Some("crate2".to_string()),
                level: LogLevelFilter::Info
            },
            LogDirective {
                name: Some("crate1::mod1".to_string()),
                level: LogLevelFilter::Warn
            }
        ]);
        assert!(logger.enabled(LogLevel::Warn, "crate1::mod1"));
        assert!(!logger.enabled(LogLevel::Info, "crate1::mod1"));
        assert!(logger.enabled(LogLevel::Info, "crate2"));
        assert!(!logger.enabled(LogLevel::Debug, "crate2"));
    }

    #[test]
    fn no_match() {
        let logger = make_logger(vec![
            LogDirective { name: Some("crate2".to_string()), level: LogLevelFilter::Info },
            LogDirective { name: Some("crate1::mod1".to_string()), level: LogLevelFilter::Warn }
        ]);
        assert!(!logger.enabled(LogLevel::Warn, "crate3"));
    }

    #[test]
    fn match_beginning() {
        let logger = make_logger(vec![
            LogDirective { name: Some("crate2".to_string()), level: LogLevelFilter::Info },
            LogDirective { name: Some("crate1::mod1".to_string()), level: LogLevelFilter::Warn }
        ]);
        assert!(logger.enabled(LogLevel::Info, "crate2::mod1"));
    }

    #[test]
    fn match_beginning_longest_match() {
        let logger = make_logger(vec![
            LogDirective { name: Some("crate2".to_string()), level: LogLevelFilter::Info },
            LogDirective { name: Some("crate2::mod".to_string()), level: LogLevelFilter::Debug },
            LogDirective { name: Some("crate1::mod1".to_string()), level: LogLevelFilter::Warn }
        ]);
        assert!(logger.enabled(LogLevel::Debug, "crate2::mod1"));
        assert!(!logger.enabled(LogLevel::Debug, "crate2"));
    }

    #[test]
    fn match_default() {
        let logger = make_logger(vec![
            LogDirective { name: None, level: LogLevelFilter::Info },
            LogDirective { name: Some("crate1::mod1".to_string()), level: LogLevelFilter::Warn }
        ]);
        assert!(logger.enabled(LogLevel::Warn, "crate1::mod1"));
        assert!(logger.enabled(LogLevel::Info, "crate2::mod2"));
    }

    #[test]
    fn zero_level() {
        let logger = make_logger(vec![
            LogDirective { name: None, level: LogLevelFilter::Info },
            LogDirective { name: Some("crate1::mod1".to_string()), level: LogLevelFilter::Off }
        ]);
        assert!(!logger.enabled(LogLevel::Error, "crate1::mod1"));
        assert!(logger.enabled(LogLevel::Info, "crate2::mod2"));
    }

    #[test]
    fn parse_logging_spec_valid() {
        let (dirs, filter) = parse_logging_spec("crate1::mod1=error,crate1::mod2,crate2=debug");
        assert_eq!(dirs.len(), 3);
        assert_eq!(dirs[0].name, Some("crate1::mod1".to_string()));
        assert_eq!(dirs[0].level, LogLevelFilter::Error);

        assert_eq!(dirs[1].name, Some("crate1::mod2".to_string()));
        assert_eq!(dirs[1].level, LogLevelFilter::max());

        assert_eq!(dirs[2].name, Some("crate2".to_string()));
        assert_eq!(dirs[2].level, LogLevelFilter::Debug);
        assert!(filter.is_none());
    }

    #[test]
    fn parse_logging_spec_invalid_crate() {
        // test parse_logging_spec with multiple = in specification
        let (dirs, filter) = parse_logging_spec("crate1::mod1=warn=info,crate2=debug");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LogLevelFilter::Debug);
        assert!(filter.is_none());
    }

    #[test]
    fn parse_logging_spec_invalid_log_level() {
        // test parse_logging_spec with 'noNumber' as log level
        let (dirs, filter) = parse_logging_spec("crate1::mod1=noNumber,crate2=debug");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LogLevelFilter::Debug);
        assert!(filter.is_none());
    }

    #[test]
    fn parse_logging_spec_string_log_level() {
        // test parse_logging_spec with 'warn' as log level
        let (dirs, filter) = parse_logging_spec("crate1::mod1=wrong,crate2=warn");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LogLevelFilter::Warn);
        assert!(filter.is_none());
    }

    #[test]
    fn parse_logging_spec_empty_log_level() {
        // test parse_logging_spec with '' as log level
        let (dirs, filter) = parse_logging_spec("crate1::mod1=wrong,crate2=");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LogLevelFilter::max());
        assert!(filter.is_none());
    }

    #[test]
    fn parse_logging_spec_global() {
        // test parse_logging_spec with no crate
        let (dirs, filter) = parse_logging_spec("warn,crate2=debug");
        assert_eq!(dirs.len(), 2);
        assert_eq!(dirs[0].name, None);
        assert_eq!(dirs[0].level, LogLevelFilter::Warn);
        assert_eq!(dirs[1].name, Some("crate2".to_string()));
        assert_eq!(dirs[1].level, LogLevelFilter::Debug);
        assert!(filter.is_none());
    }

    #[test]
    fn parse_logging_spec_valid_filter() {
        let (dirs, filter) = parse_logging_spec("crate1::mod1=error,crate1::mod2,crate2=debug/abc");
        assert_eq!(dirs.len(), 3);
        assert_eq!(dirs[0].name, Some("crate1::mod1".to_string()));
        assert_eq!(dirs[0].level, LogLevelFilter::Error);

        assert_eq!(dirs[1].name, Some("crate1::mod2".to_string()));
        assert_eq!(dirs[1].level, LogLevelFilter::max());

        assert_eq!(dirs[2].name, Some("crate2".to_string()));
        assert_eq!(dirs[2].level, LogLevelFilter::Debug);
        assert!(filter.is_some() && filter.unwrap().to_string() == "abc");
    }

    #[test]
    fn parse_logging_spec_invalid_crate_filter() {
        let (dirs, filter) = parse_logging_spec("crate1::mod1=error=warn,crate2=debug/a.c");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LogLevelFilter::Debug);
        assert!(filter.is_some() && filter.unwrap().to_string() == "a.c");
    }

    #[test]
    fn parse_logging_spec_empty_with_filter() {
        let (dirs, filter) = parse_logging_spec("crate1/a*c");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate1".to_string()));
        assert_eq!(dirs[0].level, LogLevelFilter::max());
        assert!(filter.is_some() && filter.unwrap().to_string() == "a*c");
    }
}
