#[cfg(not(lib_build))]
#[macro_use]
extern crate log;

#[cfg(feature = "std")]
use log::set_boxed_logger;

#[cfg(not(feature = "std"))]
fn set_boxed_logger(logger: Box<dyn Log>) -> Result<(), log::SetLoggerError> {
    log::set_logger(Box::leak(logger))
}

use log::{LevelFilter, Log, Metadata, Record};
use std::sync::{Arc, Mutex};

struct State {
    log_count: Mutex<usize>,
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

#[test]
fn unset() {
    let state = Arc::new(State {
        log_count: Mutex::new(0),
    });

    let logger = Box::new(Logger(state.clone()));

    set_boxed_logger(logger).unwrap();
    log::set_max_level(LevelFilter::Error);

    error!("");

    assert_eq!(*state.clone().log_count.lock().unwrap(), 1);

    log::unset_logger();

    error!("");

    assert_eq!(*state.clone().log_count.lock().unwrap(), 1);
}
