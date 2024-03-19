// ensure line number (from log!() calling position) is correctly within log record

#![allow(dead_code, unused_imports)]

use std::sync::{Arc, Mutex};

#[cfg(feature = "std")]
use log::set_boxed_logger;
use log::{info, LevelFilter, Log, Metadata, Record};

#[cfg(not(feature = "std"))]
fn set_boxed_logger(logger: Box<dyn Log>) -> Result<(), log::SetLoggerError> {
    log::set_logger(Box::leak(logger))
}

struct State {
    last_log: Mutex<Option<u32>>,
}

struct Logger(Arc<State>);

impl Log for Logger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        *self.0.last_log.lock().unwrap() = record.line();
    }

    fn flush(&self) {}
}

#[test]
fn line_number() {
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
        let default_state = Arc::new(State {
            last_log: Mutex::new(None),
        });
        let state = default_state.clone();
        set_boxed_logger(Box::new(Logger(default_state))).unwrap();
        log::set_max_level(LevelFilter::Trace);

        info!("");
        check_line(&state, 60);
    }
    fn check_line(state: &State, expected: u32) {
        let line_number = state.last_log.lock().unwrap().take().unwrap();
        assert_eq!(line_number, expected);
    }
}
