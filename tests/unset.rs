#[cfg(not(lib_build))]
#[macro_use]
extern crate log;

#[cfg(feature = "std")]
use log::{set_boxed_logger, unset_boxed_logger};

#[cfg(not(feature = "std"))]
use log::{set_logger, unset_logger};

use log::{LevelFilter, Log, Metadata, Record};
use std::sync::{Arc, Mutex};

struct State {
    log_count: Mutex<usize>,
    dropped: Mutex<bool>,
}

struct Logger(Arc<State>);

impl Log for Logger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, _: &Record) {
        *self.0.log_count.lock().unwrap() += 1;
    }

    fn flush(&self) {}
}

impl Drop for Logger {
    fn drop(&mut self) {
        *self.0.dropped.lock().unwrap() = true;
    }
}

#[test]
#[cfg(feature = "std")]
fn unset() {
    log::set_max_level(LevelFilter::Error);

    let state1 = Arc::new(State {
        log_count: Mutex::new(0),
        dropped: Mutex::new(false),
    });

    let logger1 = Box::new(Logger(state1.clone()));

    set_boxed_logger(logger1).unwrap();

    error!("");

    assert_eq!(*state1.log_count.lock().unwrap(), 1);

    unsafe { unset_boxed_logger() };

    assert_eq!(*state1.dropped.lock().unwrap(), true);

    error!("");

    assert_eq!(*state1.log_count.lock().unwrap(), 1);

    let state2 = Arc::new(State {
        log_count: Mutex::new(0),
        dropped: Mutex::new(false),
    });

    let logger2 = Box::new(Logger(state2.clone()));

    set_boxed_logger(logger2).unwrap();

    error!("");

    assert_eq!(*state2.log_count.lock().unwrap(), 1);
    assert_eq!(*state1.log_count.lock().unwrap(), 1);
}

#[test]
#[cfg(not(feature = "std"))]
fn unset() {
    log::set_max_level(LevelFilter::Error);

    let state = Arc::new(State {
        log_count: Mutex::new(0),
        dropped: Mutex::new(false),
    });

    let logger = Box::new(Logger(state.clone()));
    let logger = Box::leak(logger);

    set_logger(logger).unwrap();

    error!("");

    assert_eq!(*state.log_count.lock().unwrap(), 1);

    unset_logger();

    error!("");

    assert_eq!(*state.log_count.lock().unwrap(), 1);
}
