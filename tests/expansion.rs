#[macro_use]
extern crate log;

use log::{Level, LevelFilter, Log, Metadata, Record};

#[cfg(feature = "std")]
use log::set_boxed_logger;

#[cfg(not(feature = "std"))]
fn set_boxed_logger(logger: Box<dyn Log>) -> Result<(), log::SetLoggerError> {
    log::set_logger(Box::leak(logger))
}

struct SimpleLogger;

impl Log for SimpleLogger {
    fn enabled(&self, m: &Metadata) -> bool {
        m.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let _ = format!("{}", record.args());
        }
    }
    fn flush(&self) {}
}

static mut COUNT: usize = 0;
fn expand(msg: &str) -> &str {
    unsafe { COUNT += 1 };
    msg
}

fn main() {
    set_boxed_logger(Box::new(SimpleLogger)).unwrap();
    log::set_max_level(LevelFilter::Trace);

    unsafe { COUNT = 0 };
    trace!("expand: {}", expand("expanded"));
    assert_eq!(unsafe { COUNT }, 0);

    unsafe { COUNT = 0 };
    warn!("expand: {}", expand("expanded"));
    assert_eq!(unsafe { COUNT }, 1);
}
