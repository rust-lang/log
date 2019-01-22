//! Stateful tests of *v (inline expression value) log macros (debugv, etc.)

#[macro_use]
extern crate log;

use std::sync::{Arc, Mutex};
use log::{Level, LevelFilter, Log, Record, Metadata};

#[cfg(feature = "std")]
use log::set_boxed_logger;

#[cfg(not(feature = "std"))]
fn set_boxed_logger(logger: Box<Log>) -> Result<(), log::SetLoggerError> {
    log::set_logger(unsafe { &*Box::into_raw(logger) })
}

struct State {
    last_log: Mutex<Option<String>>,
}

fn last(state: &State) -> Option<String> {
    state.last_log.lock().unwrap().take()
}

struct Logger(Arc<State>);

impl Log for Logger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let msg = format!("{}", record.args());
        *self.0.last_log.lock().unwrap() = Some(msg);
        assert_eq!(record.file(), Some(file!()));
        assert!(record.line().is_some());
        let t = record.target().to_owned();
        assert!(t == "log_v" || t == "special", t);
    }
    fn flush(&self) {}
}

fn main() {
    let me = Arc::new(State { last_log: Mutex::new(None) });
    let a = me.clone();
    set_boxed_logger(Box::new(Logger(me))).unwrap();

    log::set_max_level(LevelFilter::Debug);

    // Simplest use
    let i = 32;
    assert_eq!(debugv!(i), 32);
    assert_eq!(last(&a), Some("i = 32".to_owned()));

    // More expression (note reformatting via `stringify!`)
    assert_eq!(debugv!(i+1), 33);
    assert_eq!(last(&a), Some("i + 1 = 33".to_owned()));

    // Use special target (note target assert in Logger::log)
    assert_eq!(errorv!(target: "special", i), 32);
    assert_eq!(last(&a), Some("i = 32".to_owned()));

    // Use custom format (note: Display format, hex output)
    assert_eq!(warnv!("custom: {} = {:#x}", i), 32);
    assert_eq!(last(&a), Some("custom: i = 0x20".to_owned()));

    // Use custom format with named specifiers (note: Display format)
    assert_eq!(infov!("index: {1:5?} ({0})", i), 32);
    assert_eq!(last(&a), Some("index:    32 (i)".to_owned()));

    // Use both special target and custom format
    assert_eq!(errorv!(target: "special", "custom: {} = {:05}", i), 32);
    assert_eq!(last(&a), Some("custom: i = 00032".to_owned()));

    // String and its default `Debug` formatting, by reference and move.
    let vt = "foo";
    assert_eq!(infov!(&vt), &"foo");
    assert_eq!(last(&a), Some("&vt = \"foo\"".to_owned()));
    assert_eq!(infov!(vt), "foo");
    assert_eq!(last(&a), Some("vt = \"foo\"".to_owned()));

    // Trace disabled, expression still returned, but no log
    let i = 2;
    assert_eq!(tracev!(i*4), 8);
    assert_eq!(last(&a), None);

    // v* macros expand and evaluate the expression exactly _once_.
    let mut o = Some(33);
    assert_eq!(debugv!(o.take()), Some(33));
    assert_eq!(last(&a), Some("o.take() = Some(33)".to_owned()));
    assert_eq!(debugv!(o.take()), None);
    assert_eq!(last(&a), Some("o.take() = None".to_owned()));

    // Use `logv!` and special target (asserted in Logger::log)
    let i = 3;
    assert_eq!(logv!(target: "special", Level::Info, i), 3);
    assert_eq!(last(&a), Some("i = 3".to_owned()));

    // logv, default target, tuple
    assert_eq!(logv!(Level::Warn, (i+1, i+2)).1, 5);
    assert_eq!(last(&a), Some("(i + 1, i + 2) = (4, 5)".to_owned()));
}
