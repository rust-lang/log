#![allow(dead_code, unused_imports)]

use lazy_static::lazy_static;
use log::{debug, error, info, trace, warn, Level, LevelFilter, Log, Metadata, Record};
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

#[cfg(feature = "std")]
use log::set_boxed_logger;

#[cfg(not(feature = "std"))]
fn set_boxed_logger(logger: Box<dyn Log>) -> Result<(), log::SetLoggerError> {
    log::set_logger(Box::leak(logger))
}

struct State {
    last_log_level: Mutex<Option<Level>>,
    last_log_location: Mutex<Option<u32>>,
}

struct Logger(Arc<State>);

impl Log for Logger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        *self.0.last_log_level.lock().unwrap() = Some(record.level());
        *self.0.last_log_location.lock().unwrap() = record.line();
    }
    fn flush(&self) {}
}

static mut setup_state: Mutex<bool> = Mutex::new(false);

lazy_static! {
    static ref a: Arc<State> = Arc::new(State {
        last_log_level: Mutex::new(None),
        last_log_location: Mutex::new(None),
    });
}

fn setup() {
    unsafe {
        let mut guard = setup_state.lock().unwrap();
        if *guard == false {
            set_boxed_logger(Box::new(Logger(a.clone()))).unwrap();
            *guard = true;
        }
    }
}

#[test]
fn filters() {
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
        setup();

        test_filter(&a, LevelFilter::Off);
        test_filter(&a, LevelFilter::Error);
        test_filter(&a, LevelFilter::Warn);
        test_filter(&a, LevelFilter::Info);
        test_filter(&a, LevelFilter::Debug);
        test_filter(&a, LevelFilter::Trace);
    }

    fn test_filter(b: &State, filter: LevelFilter) {
        log::set_max_level(filter);
        error!("");
        last(b, t(Level::Error, filter));
        warn!("");
        last(b, t(Level::Warn, filter));
        info!("");
        last(b, t(Level::Info, filter));
        debug!("");
        last(b, t(Level::Debug, filter));
        trace!("");
        last(b, t(Level::Trace, filter));

        fn last(state: &State, expected: Option<Level>) {
            let lvl = state.last_log_level.lock().unwrap().take();
            assert_eq!(lvl, expected);
        }

        fn t(lvl: Level, filter: LevelFilter) -> Option<Level> {
            if lvl <= filter {
                Some(lvl)
            } else {
                None
            }
        }
    }
}

#[test]
fn line_numbers() {
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
        setup();
        log::set_max_level(LevelFilter::Trace);

        info!(""); // ensure check_line function follows log macro
        check_log_location(&a);
    }
    #[track_caller]
    fn check_log_location(state: &State) {
        // gets check_line calling location -> compares w/ location preserved in most recent log
        // ensure check_line function follows log macro
        let location = std::panic::Location::caller().line();
        let line_number = state.last_log_location.lock().unwrap().take().unwrap();
        assert_eq!(line_number, location - 1);
    }
}
