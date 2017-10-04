#[macro_use]
extern crate log;

use std::sync::{Arc, Mutex};
use log::{Level, LevelFilter, Log, Record, Metadata};
use log::MaxLevelFilter;

#[cfg(feature = "use_std")]
use log::set_boxed_logger;
#[cfg(not(feature = "use_std"))]
fn set_boxed_logger<M>(make_logger: M) -> Result<(), log::SetLoggerError>
    where M: FnOnce(MaxLevelFilter) -> Box<Log> {
    unsafe {
        log::set_logger(|x| &*Box::into_raw(make_logger(x)))
    }
}

struct State {
    last_log: Mutex<Option<Level>>,
    filter: MaxLevelFilter,
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

fn main() {
    let mut a = None;
    set_boxed_logger(|max| {
        let me = Arc::new(State {
            last_log: Mutex::new(None),
            filter: max,
        });
        a = Some(me.clone());
        Box::new(Logger(me))
    }).unwrap();
    let a = a.unwrap();

    test(&a, LevelFilter::Off);
    test(&a, LevelFilter::Error);
    test(&a, LevelFilter::Warn);
    test(&a, LevelFilter::Info);
    test(&a, LevelFilter::Debug);
    test(&a, LevelFilter::Trace);
}

fn test(a: &State, filter: LevelFilter) {
    a.filter.set(filter);
    error!("");
    last(&a, t(Level::Error, filter));
    warn!("");
    last(&a, t(Level::Warn, filter));
    info!("");
    last(&a, t(Level::Info, filter));

    debug!("");
    if cfg!(debug_assertions) {
        last(&a, t(Level::Debug, filter));
    } else {
        last(&a, None);
    }

    trace!("");
    last(&a, None);

    fn t(lvl: Level, filter: LevelFilter) -> Option<Level> {
        if lvl <= filter {Some(lvl)} else {None}
    }
}

fn last(state: &State, expected: Option<Level>) {
    let mut lvl = state.last_log.lock().unwrap();
    assert_eq!(*lvl, expected);
    *lvl = None;
}
