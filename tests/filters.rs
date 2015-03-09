#[macro_use] extern crate log;

use std::sync::{Arc, Mutex};
use log::{LogLevel, set_logger, LogLevelFilter, Log, LogRecord, LogMetadata};
use log::MaxLogLevelFilter;

struct State {
    last_log: Mutex<Option<LogLevel>>,
    filter: MaxLogLevelFilter,
}

impl Log for Arc<State> {
    fn enabled(&self, _: &LogMetadata) -> bool {
        true
    }

    fn log(&self, record: &LogRecord) {
        *self.last_log.lock().unwrap() = Some(record.level());
    }
}

fn main() {
    let mut a = None;
    set_logger(|max| {
        let me = Arc::new(State {
            last_log: Mutex::new(None),
            filter: max,
        });
        a = Some(me.clone());
        Box::new(me)
    }).unwrap();
    let a = a.unwrap();

    test(&a, LogLevelFilter::Off);
    test(&a, LogLevelFilter::Error);
    test(&a, LogLevelFilter::Warn);
    test(&a, LogLevelFilter::Info);
    test(&a, LogLevelFilter::Debug);
    test(&a, LogLevelFilter::Trace);
}

fn test(a: &State, filter: LogLevelFilter) {
    a.filter.set(filter);
    error!("");
    last(&a, t(LogLevel::Error, filter));
    warn!("");
    last(&a, t(LogLevel::Warn, filter));
    info!("");
    last(&a, t(LogLevel::Info, filter));
    debug!("");
    last(&a, t(LogLevel::Debug, filter));
    trace!("");
    last(&a, t(LogLevel::Trace, filter));

    fn t(lvl: LogLevel, filter: LogLevelFilter) -> Option<LogLevel> {
        if lvl <= filter {Some(lvl)} else {None}
    }
}

fn last(state: &State, expected: Option<LogLevel>) {
    let mut lvl = state.last_log.lock().unwrap();
    assert_eq!(*lvl, expected);
    *lvl = None;
}
