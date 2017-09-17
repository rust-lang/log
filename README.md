log
===

A Rust library providing a lightweight logging *facade*.

[![Build Status](https://travis-ci.org/rust-lang-nursery/log.svg?branch=master)](https://travis-ci.org/rust-lang-nursery/log)
[![Build status](https://ci.appveyor.com/api/projects/status/nopdjmmjt45xcrki?svg=true)](https://ci.appveyor.com/project/alexcrichton/log)

* [`log` documentation](https://docs.rs/log)

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

In order to produce log output executables have to use a logger implementation compatible with the facade.
There are many available implementations to chose from, here are some of the most popular ones:

* Simple minimal loggers:
    * [`env_logger`](https://docs.rs/env_logger/*/env_logger/)
    * [`simple_logger`](https://github.com/borntyping/rust-simple_logger)
    * [`simplelog`](https://github.com/drakulix/simplelog.rs)
    * [`pretty_env_logger`](https://docs.rs/pretty_env_logger/*/pretty_env_logger/)
    * [`stderrlog`](https://docs.rs/stderrlog/*/stderrlog/)
    * [`flexi_logger`](https://docs.rs/flexi_logger/*/flexi_logger/)
* Complex configurable frameworks:
    * [`log4rs`](https://docs.rs/log4rs/*/log4rs/)
    * [`fern`](https://docs.rs/fern/*/fern/)
* Adaptors for other facilities:
    * [`syslog`](https://docs.rs/syslog/*/syslog/)
    * [`slog-stdlog`](https://docs.rs/slog-stdlog/*/slog_stdlog/)

Executables should choose a logger implementation and initialize it early in the
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
    env_logger::init();

    info!("starting up");

    // ...
}
```

Then when running the executable, specify a value for the `RUST_LOG`
environment variable that corresponds with the log messages you want to show.

```bash
$ RUST_LOG=info ./main
starting up
```

See the [`env_logger` documentation](https://docs.rs/env_logger)
for the `RUST_LOG` values that can be used to get log messages with different
levels or filtered to different modules.

## In tests

Tests can use the `env_logger` crate to see log messages generated during that test:

```toml
[dependencies]
log = "0.3"

[dev-dependencies]
env_logger = "0.3"
```

```rust
#[macro_use]
extern crate log;

fn add_one(num: i32) -> i32 {
    info!("add_one called with {}", num);
    num + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate env_logger;

    #[test]
    fn it_adds_one() {
        let _ = env_logger::init();
        info!("can log from the test too");
        assert_eq!(3, add_one(2));
    }

    #[test]
    fn it_handles_negative_numbers() {
        let _ = env_logger::init();
        info!("logging from another test");
        assert_eq!(-7, add_one(-8));
    }
}
```

Assuming the module under test is called `my_lib`, running the tests with the
`RUST_LOG` filtering to info messages from this module looks like:

```bash
$ RUST_LOG=my_lib=info cargo test
     Running target/debug/my_lib-...

running 2 tests
INFO:my_lib::tests: logging from another test
INFO:my_lib: add_one called with -8
test tests::it_handles_negative_numbers ... ok
INFO:my_lib::tests: can log from the test too
INFO:my_lib: add_one called with 2
test tests::it_adds_one ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured
```

Note that `env_logger::init()` needs to be called in each test in which you
want to enable logging. Additionally, the default behavior of tests to
run in parallel means that logging output may be interleaved with test output.
Either run tests in a single thread by specifying `RUST_TEST_THREADS=1` or by
running one test by specifying its name as an argument to the test binaries as
directed by the `cargo test` help docs:

```bash
$ RUST_LOG=my_lib=info cargo test it_adds_one
     Running target/debug/my_lib-...

running 1 test
INFO:my_lib::tests: can log from the test too
INFO:my_lib: add_one called with 2
test tests::it_adds_one ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured
```

## Configuring log target

By default, `env_logger` logs to stderr. If you want to log to stdout instead,
you can use the `Builder` to change the log target:

```rust
use std::env;
use env_logger::{Builder, Target};

let mut builder = Builder::new();
builder.target(Target::Stdout);
if env::var("RUST_LOG").is_ok() {
    builder.parse(&env::var("RUST_LOG").unwrap());
}
builder.init();
```
