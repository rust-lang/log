#![allow(dead_code, unused_imports)]

#[cfg(not(lib_build))]
#[macro_use]
extern crate log;

use log::{Level, LevelFilter, Log, Metadata, Record};
use std::sync::{Arc, Mutex};

#[cfg(feature = "std")]
use log::set_boxed_logger;

#[cfg(not(feature = "std"))]
fn set_boxed_logger(logger: Box<dyn Log>) -> Result<(), log::SetLoggerError> {
    log::set_logger(Box::leak(logger))
}

struct State {
    is_target_static: Mutex<Option<bool>>,
}

struct Logger(Arc<State>);

impl Log for Logger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        *self.0.is_target_static.lock().unwrap() = Some(record.target_static().is_some());
    }
    fn flush(&self) {}
}

#[cfg_attr(lib_build, test)]
fn main() {
    let me = Arc::new(State {
        is_target_static: Mutex::new(None),
    });
    let a = me.clone();
    set_boxed_logger(Box::new(Logger(me))).unwrap();
    log::set_max_level(log::LevelFilter::Error);

    let dynamic_target = "dynamic";
    error!("");
    last(&a, Some(true));
    error!(target: "","");
    last(&a, Some(true));
    error!(target: dynamic_target, "");
    last(&a, Some(false));
}

fn last(state: &State, expected: Option<bool>) {
    let is_static = state.is_target_static.lock().unwrap().take();
    assert_eq!(is_static, expected);
}
