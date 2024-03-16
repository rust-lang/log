#![allow(dead_code, unused_imports)]

use log::{debug, error, info, trace, warn, Level, LevelFilter, Log, Metadata, Record};
use std::sync::{Arc, Mutex};

#[cfg(feature = "std")]
use log::set_boxed_logger;

#[cfg(not(feature = "std"))]
fn set_boxed_logger(logger: Box<dyn Log>) -> Result<(), log::SetLoggerError> {
    log::set_logger(Box::leak(logger))
}

struct State { last_log: Mutex<Option<u32>> }

struct Logger(Arc<State>);

impl Log for Logger {
    fn enabled(&self, _: &Metadata) -> bool { true }

    fn log(&self, record: &Record) { *self.0.last_log.lock().unwrap() = record.line(); }

    fn flush(&self) {}
}

#[test]
fn line_number() {
    let default_state = Arc::new(State { last_log: Mutex::new(None) });
    let state = default_state.clone();
    set_boxed_logger(Box::new(Logger(default_state))).unwrap();

    // let record = RecordBuilder::new().args(format_args!("")).metadata(Metadata::builder().build()).module_path(None).file(None).line(Some(5)).build();
    // let logger = Logger(a.clone());
    // logger.log(&record);

    info!("");
    last(&state, 36);

}

fn last(state: &State, expected: u32) {
    let line_number = state.last_log.lock().unwrap().take().unwrap();

    assert_eq!(line_number, expected);
}
