#[macro_use]
extern crate log;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use log::{Level, Log, LevelFilter, Metadata, Record};
use log::kv::Key;

#[cfg(feature = "std")]
use log::set_boxed_logger;

#[cfg(not(feature = "std"))]
fn set_boxed_logger(logger: Box<dyn Log>) -> Result<(), log::SetLoggerError> {
    log::set_logger(unsafe { &*Box::into_raw(logger) })
}

fn main() {
    let ran = Arc::new(AtomicBool::new(false));
    set_boxed_logger(Box::new(Logger(Arc::clone(&ran)))).unwrap();
    log::set_max_level(LevelFilter::Info);

    log!(target: __log_module_path!(), Level::Info, "some {} message {}", "great", 1, {
        key1: "Some value",
        key2: 2,
    });

    assert!(ran.load(Ordering::Acquire), "logger didn't run");
}

struct Logger(Arc<AtomicBool>);

impl Log for Logger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        self.0.store(true, Ordering::Release);

        assert_eq!(record.level(), Level::Info);
        assert_eq!(record.target(), "kv_macro");
        assert_eq!(record.module_path(), Some("kv_macro"));
        assert_eq!(record.file(), Some("tests/kv_macro.rs"));
        assert_eq!(record.line(), Some(23));
        let kvs = record.key_values();
        assert_eq!(kvs.count(), 2);
        assert_eq!(kvs.get(Key::from_str("key1")).unwrap().to_string(), "Some value");
        assert_eq!(kvs.get(Key::from_str("key2")).unwrap().to_string(), "2");
    }

    fn flush(&self) {}
}
