//! External tests of debugv and tracev macros.

#[macro_use]
extern crate log;

use std::sync::{Arc, Mutex};
use log::{LevelFilter, Log, Record, Metadata};

#[cfg(feature = "std")]
use log::set_boxed_logger;

#[cfg(not(feature = "std"))]
fn set_boxed_logger(logger: Box<Log>) -> Result<(), log::SetLoggerError> {
    log::set_logger(unsafe { &*Box::into_raw(logger) })
}

struct State {
    last_log: Mutex<Option<String>>,
}

struct Logger(Arc<State>);

impl Log for Logger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let args = format!("{}", record.args());
        *self.0.last_log.lock().unwrap() = Some(args);
    }
    fn flush(&self) {}
}

fn main() {
    let me = Arc::new(State { last_log: Mutex::new(None) });
    let a = me.clone();
    set_boxed_logger(Box::new(Logger(me))).unwrap();

    log::set_max_level(LevelFilter::Trace);

    let i = 32;
    assert_eq!(debugv!(i), 32);
    assert_eq!(last(&a), Some("i = 32".to_owned()));

    let s = "foo";
    assert_eq!(tracev!(s), "foo");
    assert_eq!(last(&a), Some("s = \"foo\"".to_owned()));
}

fn last(state: &State) -> Option<String> {
    state.last_log.lock().unwrap().take()
}
