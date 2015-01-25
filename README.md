log
===

A Rust library providing a lightweight logging *facade*.

[![Build Status](https://travis-ci.org/rust-lang/log.svg?branch=master)](https://travis-ci.org/rust-lang/log)

[Documentation](http://doc.rust-lang.org/log)

A logging facade provides a single logging API that abstracts over the actual
logging implementation. Libraries can use the logging API provided by this
crate, and the consumer of those libraries can choose the logging
implementation that is most suitable for its use case.

Libraries should simply depend on the `log` crate, using the various logging
macros as they like. Applications should choose a logging implementation that
will process all logging messages.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
log = "0.2"
```

and this to your crate root:

```rust
#[macro_use]
extern crate log;
```
