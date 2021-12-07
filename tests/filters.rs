#![allow(dead_code, unused_imports)]

#[cfg(not(lib_build))]
#[macro_use]
extern crate log;

use log::{Level, LevelFilter, Log, Metadata, Record};
use std::sync::{Arc, Mutex};

#[cfg(feature = "std")]
use log::set_boxed_logger;

#[cfg(not(feature = "std"))]
fn set_boxed_logger(logger: Box<dyn Log>) -> Result<(), log::SetLoggerError> {
    log::set_logger(Box::leak(logger))
}

struct State {
    last_log: Mutex<Option<Level>>,
}

struct Logger(Arc<State>);

impl Log for Logger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        *self.0.last_log.lock().unwrap() = Some(record.level());
    }
    fn flush(&self) {}
}

#[cfg_attr(lib_build, test)]
fn main() {
    // These tests don't really make sense when static
    // max level filtering is applied
    #[cfg(not(any(
        feature = "max_level_off",
        feature = "max_level_error",
        feature = "max_level_warn",
        feature = "max_level_info",
        feature = "max_level_debug",
        feature = "max_level_trace",
        feature = "release_max_level_off",
        feature = "release_max_level_error",
        feature = "release_max_level_warn",
        feature = "release_max_level_info",
        feature = "release_max_level_debug",
        feature = "release_max_level_trace",
    )))]
    {
        let me = Arc::new(State {
            last_log: Mutex::new(None),
        });
        let a = me.clone();
        set_boxed_logger(Box::new(Logger(me))).unwrap();

        test(&a, LevelFilter::Off);
        test(&a, LevelFilter::Error);
        test(&a, LevelFilter::Warn);
        test(&a, LevelFilter::Info);
        test(&a, LevelFilter::Debug);
        test(&a, LevelFilter::Trace);
    }
}

fn test(a: &State, filter: LevelFilter) {
    log::set_max_level(filter);
    error!("");
    last(&a, t(Level::Error, filter));
    warn!("");
    last(&a, t(Level::Warn, filter));
    info!("");
    last(&a, t(Level::Info, filter));
    debug!("");
    last(&a, t(Level::Debug, filter));
    trace!("");
    last(&a, t(Level::Trace, filter));

    fn t(lvl: Level, filter: LevelFilter) -> Option<Level> {
        if lvl <= filter {
            Some(lvl)
        } else {
            None
        }
    }
}

fn last(state: &State, expected: Option<Level>) {
    let lvl = state.last_log.lock().unwrap().take();
    assert_eq!(lvl, expected);
}
