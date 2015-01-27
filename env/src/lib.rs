// Copyright 2014-2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
#![allow(unstable)]
extern crate regex;
extern crate log;

use regex::Regex;
use std::io::{self, LineBufferedWriter};
use std::io::stdio::StdWriter;
use std::sync::Mutex;
use std::os;

use log::{Log, LogLevel, LogLevelFilter, LogRecord, SetLoggerError};

struct Logger {
    directives: Vec<LogDirective>,
    filter: Option<Regex>,
    out: Mutex<LineBufferedWriter<StdWriter>>,
}

impl Log for Logger {
    fn enabled(&self, level: LogLevel, module: &str) -> bool {
        // Search for the longest match, the vector is assumed to be pre-sorted.
        for directive in self.directives.iter().rev() {
            match directive.name {
                Some(ref name) if !module.starts_with(&**name) => {},
                Some(..) | None => {
                    return level <= directive.level
                }
            }
        }
        false
    }

    fn log(&self, record: &LogRecord) {
        if !self.enabled(record.level(), record.location().module_path) {
            return;
        }

        if let Some(filter) = self.filter.as_ref() {
            if filter.is_match(&*record.args().to_string()) {
                return;
            }
        }

        let _ = writeln!(&mut *self.out.lock().unwrap(),
                         "{}:{}: {}",
                         record.level(),
                         record.location().module_path,
                         record.args());
    }
}

struct LogDirective {
    name: Option<String>,
    level: LogLevelFilter,
}

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(|max_level| {
        let (mut directives, filter) = match os::getenv("RUST_LOG") {
            Some(spec) => parse_logging_spec(spec.as_slice()),
            None => (Vec::new(), None),
        };

        // Sort the provided directives by length of their name, this allows a
        // little more efficient lookup at runtime.
        directives.sort_by(|a, b| {
            let alen = a.name.as_ref().map(|a| a.len()).unwrap_or(0);
            let blen = b.name.as_ref().map(|b| b.len()).unwrap_or(0);
            alen.cmp(&blen)
        });

        let level = {
            let max = directives.iter().max_by(|d| d.level);
            max.map(|d| d.level).unwrap_or(LogLevelFilter::max())
        };
        max_level.set(level);

        Box::new(Logger {
            directives: directives,
            filter: filter,
            out: Mutex::new(io::stderr()),
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
                    Some(num) => (num, None),
                    None => (LogLevelFilter::max(), Some(part0)),
                }
            }
            (Some(part0), Some(""), None) => (LogLevelFilter::max(), Some(part0)),
            (Some(part0), Some(part1), None) => {
                match part1.parse() {
                    Some(num) => (num, Some(part0)),
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
    use std::io;
    use std::sync::Mutex;
    use log::{Log, LogLevel, LogLevelFilter};

    use super::{Logger, LogDirective, parse_logging_spec};

    fn make_logger(dirs: Vec<LogDirective>) -> Logger {
        Logger {
            directives: dirs,
            filter: None,
            out: Mutex::new(io::stderr())
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
