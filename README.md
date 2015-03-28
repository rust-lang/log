log
===

A Rust library providing a lightweight logging *facade*.

[![Build Status](https://travis-ci.org/rust-lang/log.svg?branch=master)](https://travis-ci.org/rust-lang/log)

* [`log` documentation](http://doc.rust-lang.org/log)
* [`env_logger` documentation](http://doc.rust-lang.org/log/env_logger)

A logging facade provides a single logging API that abstracts over the actual
logging implementation. Libraries can use the logging API provided by this
crate, and the consumer of those libraries can choose the logging
implementation that is most suitable for its use case.

## Usage

## In libraries

Libraries should link only to the `log` crate, and use the provided macros to
log whatever information will be useful to downstream consumers:

```toml
[dependencies]
log = "0.3"
```

```rust
#[macro_use]
extern crate log;

pub fn shave_the_yak(yak: &Yak) {
    trace!("Commencing yak shaving");

    loop {
        match find_a_razor() {
            Ok(razor) => {
                info!("Razor located: {}", razor);
                yak.shave(razor);
                break;
            }
            Err(err) => {
                warn!("Unable to locate a razor: {}, retrying", err);
            }
        }
    }
}
```

## In executables

Executables should chose a logger implementation and initialize it early in the
runtime of the program. Logger implementations will typically include a
function to do this. Any log messages generated before the logger is
initialized will be ignored.

The executable itself may use the `log` crate to log as well.

The `env_logger` crate provides a logger implementation that mirrors the
functionality of the old revision of the `log` crate.

```toml
[dependencies]
log = "0.3"
env_logger = "0.3"
```

```rust
#[macro_use]
extern crate log;
extern crate env_logger;

fn main() {
    env_logger::init().unwrap();

    info!("starting up");

    // ...
}
```
