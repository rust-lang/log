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
//! use log::Level;
//!
//! fn main() {
//!     env_logger::init();
//!
//!     debug!("this is a debug {}", "message");
//!     error!("this is printed by default");
//!
//!     if log_enabled!(Level::Info) {
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
//! environment variable. The value of this environment variable is a
//! comma-separated list of logging directives. A logging directive is of the
//! form:
//!
//! ```text
//! path::to::module=level
//! ```
//!
//! The path to the module is rooted in the name of the crate it was compiled
//! for, so if your program is contained in a file `hello.rs`, for example, to
//! turn on logging for this file you would use a value of `RUST_LOG=hello`.
//! Furthermore, this path is a prefix-search, so all modules nested in the
//! specified module will also have logging enabled.
//!
//! The actual `level` is optional to specify. If omitted, all logging will
//! be enabled. If specified, it must be one of the strings `debug`, `error`,
//! `info`, `warn`, or `trace`.
//!
//! As the log level for a module is optional, the module to enable logging for
//! is also optional. If only a `level` is provided, then the global log
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
       html_root_url = "https://docs.rs/env_logger/0.4.3")]
#![cfg_attr(test, deny(warnings))]

// When compiled for the rustc compiler itself we want to make sure that this is
// an unstable crate
#![cfg_attr(rustbuild, feature(staged_api, rustc_private))]
#![cfg_attr(rustbuild, unstable(feature = "rustc_private", issue = "27812"))]

extern crate log;

use std::env;
use std::io::prelude::*;
use std::io;
use std::mem;

use log::{Log, Level, LevelFilter, Record, SetLoggerError, Metadata};

#[cfg(feature = "regex")]
#[path = "regex.rs"]
mod filter;

#[cfg(not(feature = "regex"))]
#[path = "string.rs"]
mod filter;

/// Log target, either stdout or stderr.
#[derive(Debug)]
pub enum Target {
    Stdout,
    Stderr,
}

/// The logger.
pub struct Logger {
    directives: Vec<Directive>,
    filter: Option<filter::Filter>,
    format: Box<Fn(&Record) -> String + Sync + Send>,
    target: Target,
}

/// Builder acts as builder for initializing the Logger.
/// It can be used to customize the log format, change the enviromental variable used
/// to provide the logging directives and also set the default log level filter.
///
/// ## Example
///
/// ```
/// #[macro_use]
/// extern crate log;
/// extern crate env_logger;
///
/// use std::env;
/// use log::{Record, LevelFilter};
/// use env_logger::Builder;
///
/// fn main() {
///     let format = |record: &Record| {
///         format!("{} - {}", record.level(), record.args())
///     };
///
///     let mut builder = Builder::new();
///     builder.format(format).filter(None, LevelFilter::Info);
///
///     if env::var("RUST_LOG").is_ok() {
///        builder.parse(&env::var("RUST_LOG").unwrap());
///     }
///
///     builder.init();
///
///     error!("error message");
///     info!("info message");
/// }
/// ```
pub struct Builder {
    directives: Vec<Directive>,
    filter: Option<filter::Filter>,
    format: Box<Fn(&Record) -> String + Sync + Send>,
    target: Target,
}

impl Builder {
    /// Initializes the log builder with defaults
    pub fn new() -> Builder {
        Builder {
            directives: Vec::new(),
            filter: None,
            format: Box::new(|record: &Record| {
                format!("{}:{}: {}", record.level(),
                        record.location().module_path(), record.args())
            }),
            target: Target::Stderr,
        }
    }

    /// Adds filters to the logger
    ///
    /// The given module (if any) will log at most the specified level provided.
    /// If no module is provided then the filter will apply to all log messages.
    pub fn filter(&mut self,
                  module: Option<&str>,
                  level: LevelFilter) -> &mut Self {
        self.directives.push(Directive {
            name: module.map(|s| s.to_string()),
            level: level,
        });
        self
    }

    /// Sets the format function for formatting the log output.
    ///
    /// This function is called on each record logged to produce a string which
    /// is actually printed out.
    pub fn format<F: 'static>(&mut self, format: F) -> &mut Self
        where F: Fn(&Record) -> String + Sync + Send
    {
        self.format = Box::new(format);
        self
    }

    /// Sets the target for the log output.
    ///
    /// Env logger can log to either stdout or stderr. The default is stderr.
    pub fn target(&mut self, target: Target) -> &mut Self {
        self.target = target;
        self
    }

    /// Parses the directives string in the same form as the RUST_LOG
    /// environment variable.
    ///
    /// See the module documentation for more details.
    pub fn parse(&mut self, filters: &str) -> &mut Self {
        let (directives, filter) = parse_spec(filters);

        self.filter = filter;

        for directive in directives {
            self.directives.push(directive);
        }
        self
    }

    /// Initializes the global logger with an env logger.
    ///
    /// This should be called early in the execution of a Rust program, and the
    /// global logger may only be initialized once. Future initialization
    /// attempts will return error.
    pub fn try_init(&mut self) -> Result<(), SetLoggerError> {
        log::try_set_logger(|max_level| {
            let logger = self.build();
            max_level.set(logger.filter());
            Box::new(logger)
        })
    }

    /// Initializes the global logger with an env logger.
    ///
    /// This should be called early in the execution of a Rust program, and the
    /// global logger may only be initialized once. Future initialization
    /// attempts will panic.
    pub fn init(&mut self) {
        let logger = self.build();
        let filter = logger.filter();
        let logger = Box::new(logger);
        log::set_logger(logger, filter);
    }

    /// Build an env logger.
    pub fn build(&mut self) -> Logger {
        if self.directives.is_empty() {
            // Adds the default filter if none exist
            self.directives.push(Directive {
                name: None,
                level: LevelFilter::Error,
            });
        } else {
            // Sort the directives by length of their name, this allows a
            // little more efficient lookup at runtime.
            self.directives.sort_by(|a, b| {
                let alen = a.name.as_ref().map(|a| a.len()).unwrap_or(0);
                let blen = b.name.as_ref().map(|b| b.len()).unwrap_or(0);
                alen.cmp(&blen)
            });
        }

        Logger {
            directives: mem::replace(&mut self.directives, Vec::new()),
            filter: mem::replace(&mut self.filter, None),
            format: mem::replace(&mut self.format, Box::new(|_| String::new())),
            target: mem::replace(&mut self.target, Target::Stderr),
        }
    }
}

impl Logger {
    pub fn new() -> Logger {
        let mut builder = Builder::new();

        if let Ok(s) = env::var("RUST_LOG") {
            builder.parse(&s);
        }

        builder.build()
    }

    pub fn filter(&self) -> LevelFilter {
        self.directives.iter()
            .map(|d| d.level).max()
            .unwrap_or(LevelFilter::Off)
    }

    fn enabled(&self, level: Level, target: &str) -> bool {
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
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.enabled(metadata.level(), metadata.target())
    }

    fn log(&self, record: &Record) {
        if !Log::enabled(self, record.metadata()) {
            return;
        }

        if let Some(filter) = self.filter.as_ref() {
            if !filter.is_match(&*record.args().to_string()) {
                return;
            }
        }

        match self.target {
            Target::Stdout => println!("{}", (self.format)(record)),
            Target::Stderr => {
                let _ = writeln!(&mut io::stderr(), "{}", (self.format)(record));
            },
        };
    }
}

struct Directive {
    name: Option<String>,
    level: LevelFilter,
}

/// Initializes the global logger with an env logger.
///
/// This should be called early in the execution of a Rust program, and the
/// global logger may only be initialized once. Future initialization attempts
/// will return an error.
pub fn try_init() -> Result<(), SetLoggerError> {
    let mut builder = Builder::new();

    if let Ok(s) = env::var("RUST_LOG") {
        builder.parse(&s);
    }

    builder.try_init()
}

/// Initializes the global logger with an env logger.
///
/// This should be called early in the execution of a Rust program, and the
/// global logger may only be initialized once. Future initialization attempts
/// will panic.
pub fn init() {
    let mut builder = Builder::new();

    if let Ok(s) = env::var("RUST_LOG") {
        builder.parse(&s);
    }

    builder.init()
}

/// Parse a logging specification string (e.g: "crate1,crate2::mod3,crate3::x=error/foo")
/// and return a vector with log directives.
fn parse_spec(spec: &str) -> (Vec<Directive>, Option<filter::Filter>) {
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
                    Err(_) => (LevelFilter::max(), Some(part0)),
                }
            }
            (Some(part0), Some(""), None) => (LevelFilter::max(), Some(part0)),
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
        dirs.push(Directive {
            name: name.map(|s| s.to_string()),
            level: log_level,
        });
    }});

    let filter = filter.map_or(None, |filter| {
        match filter::Filter::new(filter) {
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
    use log::{Level, LevelFilter};

    use super::{Builder, Logger, Directive, parse_spec};

    fn make_logger(dirs: Vec<Directive>) -> Logger {
        let mut logger = Builder::new().build();
        logger.directives = dirs;
        logger
    }

    #[test]
    fn filter_info() {
        let logger = Builder::new().filter(None, LevelFilter::Info).build();
        assert!(logger.enabled(Level::Info, "crate1"));
        assert!(!logger.enabled(Level::Debug, "crate1"));
    }

    #[test]
    fn filter_beginning_longest_match() {
        let logger = Builder::new()
                        .filter(Some("crate2"), LevelFilter::Info)
                        .filter(Some("crate2::mod"), LevelFilter::Debug)
                        .filter(Some("crate1::mod1"), LevelFilter::Warn)
                        .build();
        assert!(logger.enabled(Level::Debug, "crate2::mod1"));
        assert!(!logger.enabled(Level::Debug, "crate2"));
    }

    #[test]
    fn parse_default() {
        let logger = Builder::new().parse("info,crate1::mod1=warn").build();
        assert!(logger.enabled(Level::Warn, "crate1::mod1"));
        assert!(logger.enabled(Level::Info, "crate2::mod2"));
    }

    #[test]
    fn match_full_path() {
        let logger = make_logger(vec![
            Directive {
                name: Some("crate2".to_string()),
                level: LevelFilter::Info
            },
            Directive {
                name: Some("crate1::mod1".to_string()),
                level: LevelFilter::Warn
            }
        ]);
        assert!(logger.enabled(Level::Warn, "crate1::mod1"));
        assert!(!logger.enabled(Level::Info, "crate1::mod1"));
        assert!(logger.enabled(Level::Info, "crate2"));
        assert!(!logger.enabled(Level::Debug, "crate2"));
    }

    #[test]
    fn no_match() {
        let logger = make_logger(vec![
            Directive { name: Some("crate2".to_string()), level: LevelFilter::Info },
            Directive { name: Some("crate1::mod1".to_string()), level: LevelFilter::Warn }
        ]);
        assert!(!logger.enabled(Level::Warn, "crate3"));
    }

    #[test]
    fn match_beginning() {
        let logger = make_logger(vec![
            Directive { name: Some("crate2".to_string()), level: LevelFilter::Info },
            Directive { name: Some("crate1::mod1".to_string()), level: LevelFilter::Warn }
        ]);
        assert!(logger.enabled(Level::Info, "crate2::mod1"));
    }

    #[test]
    fn match_beginning_longest_match() {
        let logger = make_logger(vec![
            Directive { name: Some("crate2".to_string()), level: LevelFilter::Info },
            Directive { name: Some("crate2::mod".to_string()), level: LevelFilter::Debug },
            Directive { name: Some("crate1::mod1".to_string()), level: LevelFilter::Warn }
        ]);
        assert!(logger.enabled(Level::Debug, "crate2::mod1"));
        assert!(!logger.enabled(Level::Debug, "crate2"));
    }

    #[test]
    fn match_default() {
        let logger = make_logger(vec![
            Directive { name: None, level: LevelFilter::Info },
            Directive { name: Some("crate1::mod1".to_string()), level: LevelFilter::Warn }
        ]);
        assert!(logger.enabled(Level::Warn, "crate1::mod1"));
        assert!(logger.enabled(Level::Info, "crate2::mod2"));
    }

    #[test]
    fn zero_level() {
        let logger = make_logger(vec![
            Directive { name: None, level: LevelFilter::Info },
            Directive { name: Some("crate1::mod1".to_string()), level: LevelFilter::Off }
        ]);
        assert!(!logger.enabled(Level::Error, "crate1::mod1"));
        assert!(logger.enabled(Level::Info, "crate2::mod2"));
    }

    #[test]
    fn parse_spec_valid() {
        let (dirs, filter) = parse_spec("crate1::mod1=error,crate1::mod2,crate2=debug");
        assert_eq!(dirs.len(), 3);
        assert_eq!(dirs[0].name, Some("crate1::mod1".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::Error);

        assert_eq!(dirs[1].name, Some("crate1::mod2".to_string()));
        assert_eq!(dirs[1].level, LevelFilter::max());

        assert_eq!(dirs[2].name, Some("crate2".to_string()));
        assert_eq!(dirs[2].level, LevelFilter::Debug);
        assert!(filter.is_none());
    }

    #[test]
    fn parse_spec_invalid_crate() {
        // test parse_spec with multiple = in specification
        let (dirs, filter) = parse_spec("crate1::mod1=warn=info,crate2=debug");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::Debug);
        assert!(filter.is_none());
    }

    #[test]
    fn parse_spec_invalid_level() {
        // test parse_spec with 'noNumber' as log level
        let (dirs, filter) = parse_spec("crate1::mod1=noNumber,crate2=debug");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::Debug);
        assert!(filter.is_none());
    }

    #[test]
    fn parse_spec_string_level() {
        // test parse_spec with 'warn' as log level
        let (dirs, filter) = parse_spec("crate1::mod1=wrong,crate2=warn");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::Warn);
        assert!(filter.is_none());
    }

    #[test]
    fn parse_spec_empty_level() {
        // test parse_spec with '' as log level
        let (dirs, filter) = parse_spec("crate1::mod1=wrong,crate2=");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::max());
        assert!(filter.is_none());
    }

    #[test]
    fn parse_spec_global() {
        // test parse_spec with no crate
        let (dirs, filter) = parse_spec("warn,crate2=debug");
        assert_eq!(dirs.len(), 2);
        assert_eq!(dirs[0].name, None);
        assert_eq!(dirs[0].level, LevelFilter::Warn);
        assert_eq!(dirs[1].name, Some("crate2".to_string()));
        assert_eq!(dirs[1].level, LevelFilter::Debug);
        assert!(filter.is_none());
    }

    #[test]
    fn parse_spec_valid_filter() {
        let (dirs, filter) = parse_spec("crate1::mod1=error,crate1::mod2,crate2=debug/abc");
        assert_eq!(dirs.len(), 3);
        assert_eq!(dirs[0].name, Some("crate1::mod1".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::Error);

        assert_eq!(dirs[1].name, Some("crate1::mod2".to_string()));
        assert_eq!(dirs[1].level, LevelFilter::max());

        assert_eq!(dirs[2].name, Some("crate2".to_string()));
        assert_eq!(dirs[2].level, LevelFilter::Debug);
        assert!(filter.is_some() && filter.unwrap().to_string() == "abc");
    }

    #[test]
    fn parse_spec_invalid_crate_filter() {
        let (dirs, filter) = parse_spec("crate1::mod1=error=warn,crate2=debug/a.c");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::Debug);
        assert!(filter.is_some() && filter.unwrap().to_string() == "a.c");
    }

    #[test]
    fn parse_spec_empty_with_filter() {
        let (dirs, filter) = parse_spec("crate1/a*c");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate1".to_string()));
        assert_eq!(dirs[0].level, LevelFilter::max());
        assert!(filter.is_some() && filter.unwrap().to_string() == "a*c");
    }
}
