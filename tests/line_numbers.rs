#![allow(dead_code, unused_imports)]

use log::{debug, error, info, trace, warn, Level, LevelFilter, Log, Metadata, Record};
use std::sync::{Arc, Mutex};

#[cfg(feature = "std")]
use log::set_boxed_logger;

#[cfg(not(feature = "std"))]
fn set_boxed_logger(logger: Box<dyn Log>) -> Result<(), log::SetLoggerError> {
    log::set_logger(Box::leak(logger))
}

struct State<'a> {
    last_log: Mutex<Option<Record<'a>>>,
}

struct Logger<'a>(Arc<State<'a>>);

impl Log for Logger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: Record) {
        *self.0.last_log.lock().unwrap() = Some(record);
    }
    fn flush(&self) {}
}

#[cfg_attr(lib_build, test)]
fn main() {
    let me = Arc::new(State {
        last_log: Mutex::new(None),
    });
    let a = me.clone();
    set_boxed_logger(Box::new(Logger(me))).unwrap();


    error!("");
    last(&a, 40);
    warn!("");
    last(&a, 42);
    info!("");
    last(&a, 44);
    debug!("");
    last(&a, 46);
    trace!("");
    last(&a, 48);

}

fn last(state: &State, expected: u32) {
    let last_log= state.last_log.lock().unwrap().take().unwrap();
    let line_number = last_log.line().unwrap();

    assert_eq!(line_number, expected);
}
