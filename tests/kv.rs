#[macro_use]
extern crate log;

use std::io::Write;
use std::sync::{Arc, Mutex};
use log::{Level, LevelFilter, Log, Record, Metadata};

#[cfg(feature = "std")]
use log::set_boxed_logger;

#[cfg(not(feature = "std"))]
fn set_boxed_logger(logger: Box<Log>) -> Result<(), log::SetLoggerError> {
    log::set_logger(unsafe { &*Box::into_raw(logger) })
}

struct Logger {
    buf: Arc<Mutex<Vec<u8>>>,
}

impl Log for Logger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let mut buf = self.buf.lock().unwrap();
        buf.write_fmt(*record.args()).unwrap();
        for kv in record.kvs() {
            write!(buf, " {} = {},", kv.key(), kv.value()).unwrap();
        }
        buf.write(b"\n").unwrap();
    }

    fn flush(&self) {}
}

fn main() {
    let buf = Vec::new();
    let buf = Arc::new(Mutex::new(buf));

    log::set_max_level(LevelFilter::Trace);
    set_boxed_logger(Box::new(Logger { buf: Arc::clone(&buf) })).unwrap();

    let user = ("Bob", 123);

    // Single message, no key-value pairs.
    log!(Level::Error, "Simple message");
    log!(target: "target", Level::Error, "Targeted message");
    // Message with arguments, no key-value pairs.
    log!(Level::Error, "Args message: {}, {}", "arg1", 890);
    log!(target: "target", Level::Error, "Targeted args message: {}, {}", "arg1", 890);
    // Single message, no arguments.
    log!(Level::Error, "KV message"; id = user.1, name = user.0);
    log!(target: "target", Level::Error, "Targeted KV message"; id = 123, key2 = "value2");
    // Message with arguments, two key-value pairs.
    log!(Level::Error, "Args KV message: {}, {}", "arg1", 890; id = 123, key2 = "value2");
    log!(target: "target", Level::Error, "Targeted args KV message: {}, {}", "arg1", 890; id = 123, key2 = "value2");

    let buf = buf.lock().unwrap();
    let got = String::from_utf8_lossy(&buf);
    const WANT: &str = "Simple message
Targeted message
Args message: arg1, 890
Targeted args message: arg1, 890
KV message id = 123, name = Bob,
Targeted KV message id = 123, key2 = value2,
Args KV message: arg1, 890 id = 123, key2 = value2,
Targeted args KV message: arg1, 890 id = 123, key2 = value2,
";
    assert_eq!(got, WANT);
}
