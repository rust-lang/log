# Summary
[summary]: #summary

Add support for structured logging to the `log` crate in both `std` and `no_std` environments, allowing log records to carry typed data beyond a textual message. This document serves as an introduction to what structured logging is all about, and as an RFC for an implementation in the `log` crate.

`log` will provide a lightweight fundamental serialization API out-of-the-box that allows a fixed set of common types from the standard library to be logged as structured values. Using optional Cargo features, that set can be expanded to support anything that implements `serde::Serialize + std::fmt::Debug`. It doesn't turn `log` into a pervasive public dependency to support structured logging for types outside the standard library.

The API is heavily inspired by the `slog` logging framework.

> NOTE: Code in this RFC uses recent language features like `impl Trait`, but can be implemented without them.

# Contents

- [Motivation](#motivation)
  - [What is structured logging?](#what-is-structured-logging)
  - [Why do we need structured logging in `log`?](#why-do-we-need-structured-logging-in-log)
- [Guide-level explanation](#guide-level-explanation)
  - [Logging structured key-value pairs](#logging-structured-key-value-pairs)
  - [Supporting key-value pairs in `Log` implementations](#supporting-key-value-pairs-in-log-implementations)
  - [Integrating log frameworks with `log`](#integrating-log-frameworks-with-log)
  - [Writing your own `value::Visitor`](#writing-your-own-valuevisitor)
- [Reference-level explanation](#reference-level-explanation)
  - [Design considerations](#design-considerations)
  - [Implications for dependents](#implications-for-dependents)
  - [Cargo features](#cargo-features)
  - [Key-values API](#key-values-api)
    - [`Error`](#error)
    - [`value::Visit`](#valueVisit)
    - [`value::Visitor`](#valuevisitor)
    - [`Value`](#valuevalue)
    - [`Key`](#key)
    - [`Source`](#source)
    - [`source::Visitor`](#sourcevisitor)
    - [`Record` and `RecordBuilder`](#record-and-recordbuilder)
  - [The `log!` macros](#the-log-macros)
- [Drawbacks, rationale, and alternatives](#drawbacks-rationale-and-alternatives)
- [Prior art](#prior-art)
  - [Rust](#rust)
  - [Go](#go)
  - [.NET](#net)
- [Unresolved questions](#unresolved-questions)
- [Appendix](#appendix)
  - [Public API](#public-api)

# Motivation
[motivation]: #motivation

## What is structured logging?

Information in log records can be traditionally captured as a blob of text, including a level, a message, and maybe a few other pieces of metadata. There's a lot of potentially valuable information we throw away when we format data as text. Arbitrary textual representations often result in log records that are neither easy for humans to read, nor for machines to parse.

Structured logs can retain their original structure in a machine-readable format. They can be changed programmatically within a logging pipeline before reaching their destination. Once there, they can be analyzed using common database tools.

As an example of structured logging, a textual log like this:

```
[INF 2018-09-27T09:32:03Z basic] [service: database, correlation: 123] Operation completed successfully in 18ms
```

could be represented as a structured log like this:

```json
{
    "ts": 1538040723000,
    "lvl": "INFO",
    "msg": "Operation completed successfully in 18ms",
    "module": "basic",
    "service": "database",
    "correlation": 123,
    "took": 18
}
```

When log records are kept in a format like this, potentially interesting queries like _what are all records where the correlation is 123?_, or _how many errors were there in the last hour?_ can be computed efficiently.

Even when logging to a console for immediate consumption, the human-readable message can be presented better when it's not trying to include ambient metadata inline:

```
[INF 2018-09-27T09:32:03Z] Operation completed successfully in 18ms
module: "basic"
service: "database"
correlation: 123
took: 18
```

Having a way to capture additional metadata is good for human-centric formats. Having a way to retain the structure of that metadata is good for machine-centric formats.

## Why do we need structured logging in `log`?

Why add structured logging support to the `log` crate when libraries like `slog` already exist and support it? `log` needs to support structured logging to make the experience of using `slog` and other logging tools in the Rust ecosystem more compatible.

On the surface there doesn't seem to be a lot of difference between `log` and `slog`, so why not just deprecate one in favour of the other? Conceptually, `log` and `slog` are different libraries that fill different roles, even if there's some overlap.

`slog` is a logging _framework_. It offers all the fundamental tools needed out-of-the-box to capture log records, define and implement the composable pieces of a logging pipeline, and pass them through that pipeline to an eventual destination. It has conventions and trade-offs baked into the design of its API. Loggers are treated explicitly as values in data structures and as arguments, and callers can control whether to pass owned or borrowed data.

`log` is a logging _facade_. It's only concerned with a standard, minimal API for capturing log records, and surfacing those records to some consumer. The tools provided by `log` are only those that are fundamental to the operation of the `log!` macro. From `log`'s point of view, a logging framework like `slog` is a black-box implementation of the `Log` trait. In this role, the `Log` trait can act as a common entrypoint for capturing log records. That means the `Record` type can act as a common container for describing a log record. `log` has its own set of trade-offs baked into the design of its API. The `log!` macro assumes a single, global entrypoint, and all data in a log record is borrowed from the callsite.

A healthy logging ecosystem needs both `log` and frameworks like `slog`. As a standard API, `log` can support a diverse but cohesive ecosystem of logging tools in Rust by acting as the glue between libraries, frameworks, and applications. A lot of libraries already depend on it. In order to really fulfil this role though, `log` needs to support structured logging so that libraries and their consumers can take advantage of it in a framework-agnostic way.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Logging structured key-value pairs

Structured logging is supported in `log` by allowing typed key-value pairs to be associated with a log record. A `;` separates structured key-value pairs from values that are replaced into the message:

```rust
info!(
    "This is the rendered {message}. It is not structured",
    message = "message";
    correlation = correlation_id,
    user
);
```

Any `value` or `key = value` expressions before the `;` in the macro will be interpolated into the message as unstructured text using `std::fmt`. This is the `log!` macro we have today. Any `value` or `key = value` expressions after the `;` will be captured as structured key-value pairs. These structured key-value pairs can be inspected or serialized, retaining some notion of their original type. That means in the above example, the `message` key is unstructured, and the `correlation` and `user` keys are structured:

```
info!(
    "This is the rendered {message}. It is not structured",
    message = "message";
    ^^^^^^^^^^^^^^^^^^^
    unstructured

    correlation = correlation_id,
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    structured

    user
    ^^^^
    structured
);
```

### What can be logged?

Using default Cargo features, a closed set of types from the standard library are supported as structured values:

- Standard formats: `Arguments`
- Primitives: `bool`, `char`
- Unsigned integers: `u8`, `u16`, `u32`, `u64`, `u128`
- Signed integers: `i8`, `i16`, `i32`, `i64`, `i128`
- Strings: `&str`, `String`
- Bytes: `&[u8]`, `Vec<u8>`
- Paths: `Path`, `PathBuf`
- Special types: `Option<T>` and `()`.

In the example from before, `correlation_id` and `user` can be used as structured values if they're in that set of concrete types:

```rust
let user = "a user id";
let correlation_id = "some correlation id";

info!(
    "This is the rendered {message}. It is not structured",
    message = "message";
    correlation = correlation_id,
    user
);
```

What if the `correlation_id` is a `uuid::Uuid` instead of a string? What if the `user` is some other datastructure containing an id along with some other metadata? Only being able to log a few types from the standard library is a bit limiting. To make logging other values possible, the `kv_serde` Cargo feature expands the set of loggable values above to also include any other type that implements both `std::fmt::Debug` and `serde::Serialize`:

```toml
[dependencies.log]
features = ["kv_serde"]

[dependencies.uuid]
features = ["serde"]
```

```rust
#[derive(Debug, Serialize)]
struct User {
    id: Uuid,
    ..
}

let user = User { id, .. };
let correlation_id = Uuid::new_v4();

info!(
    "This is the rendered {message}. It is not structured",
    message = "message";
    correlation = correlation_id,
    user
);
```

So the effective trait bounds for structured values are `Debug + Serialize`:

```
info!(
    "This is the rendered {message}. It is not structured",
    message = "message";
              ^^^^^^^^^
              Display

    correlation = correlation_id,
                  ^^^^^^^^^^^^^^
                  Debug + Serialize

    user
    ^^^^
    Debug + Serialize
);
```

If you come across a data type in the Rust ecosystem that you can't log, then add the `kv_serde` feature to `log` and try looking for a `serde` feature on the crate that defines it. If there isn't one already then adding it will be useful not just for you, but for anyone that might want to serialize those types for other reasons.

## Supporting key-value pairs in `Log` implementations

Capturing structured logs is only half the story. Implementors of the `Log` trait also need to be able to work with any key-value pairs associated with a log record. Key-value pairs are accessible on a log record through the `key_values` method:

```rust
impl Record {
    pub fn key_values(&self) -> impl Source;
}
```

where `Source` is a trait for iterating over the individual key-value pairs.

To demonstrate how to work with a `Source`, let's take the terminal log format from before:

```
[INF 2018-09-27T09:32:03Z] Operation completed successfully in 18ms
module: "basic"
service: "database"
correlation: 123
took: 18
```

Each key-value pair, shown as `$key: $value`, can be formatted from the `Source` using the `std::fmt` machinery:

```rust
use log::kv::Source;

fn write_pretty(w: impl Write, r: &Record) -> io::Result<()> {
    // Write the first line of the log record
    ...

    // Write each key-value pair on a new line
    record
        .key_values()
        .try_for_each(|k, v| writeln!("{}: {}", k, v))?;
    
    Ok(())
}
```

In the above example, the `try_for_each` method iterates over each key-value pair and writes them to the terminal. Now take the following json format:

```json
{
    "ts": 1538040723000,
    "lvl": "INFO",
    "msg": "Operation completed successfully in 18ms",
    "module": "basic",
    "service": "database",
    "correlation": 123,
    "took": 18
}
```

Defining a serializable structure based on a log record for this format could be done using `serde_derive`, and then written using `serde_json`. This requires the `kv_serde` feature:

```toml
[dependencies.log]
features = ["kv_serde"]
```

The structured key-value pairs can then be naturally serialized as a map:

```rust
use log::kv::Source;

fn write_json(w: impl Write, r: &Record) -> io::Result<()> {
    let r = SerializeRecord {
        lvl: r.level(),
        ts: epoch_millis(),
        msg: r.args().to_string(),
        props: r.key_values().serialize_as_map(),
    };

    serde_json::to_writer(w, &r)?;

    Ok(())
}

#[derive(Serialize)]
struct SerializeRecord<KVS> {
    lvl: Level,
    ts: u64,
    msg: String,
    #[serde(flatten)]
    props: KVS,
}
```

This time, instead of using the `try_for_each` method, we use `serialize_as_map` to get an adapter that will serialize each key-value pair as an entry in a map.

The crate that produces log records might not be the same crate that consumes them. A producer can depend on the `kv_serde` feature to log more types, and a consumer will always be able to handle them, even if they don't depend on the `kv_serde` feature.

## Integrating log frameworks with `log`

The `Source` trait describes some container for structured key-value pairs. Other log frameworks that want to integrate with the `log` crate should build `Record`s that contain some implementation of `Source` based on their own structured logging.

The previous section demonstrated some of the methods available on `Source` like `Source::try_for_each` and `Source::serialize_as_map`. Both of those methods are provided on top of a single required `Source::visit` method. The `Source` trait itself looks something like this:

```rust
trait Source {
    fn visit<'kvs>(&'kvs self, visitor: &mut dyn Visitor<'kvs>) -> Result<(), Error>;

    // Provided methods
}
```

where `source::Visitor` is another trait that accepts individual key-value pairs:

```rust
trait Visitor<'kvs> {
    fn visit_pair(&mut self, k: Key<'kvs>, v: Value<'kvs>) -> Result<(), Error>;
}
```

The following example wraps up a `BTreeMap<String, serde_json::Value>` and implements the `Source` trait for it:

```rust
use log::kv::source::{self, Source};

struct MySource {
    data: BTreeMap<String, serde_json::Value>,
}

impl Source for MySource {
    fn visit<'kvs>(&'kvs self, visitor: &mut dyn source::Visitor<'kvs>) -> Result<(), source::Error> {
        self.data.visit(visitor)
    }
}
```

The implementation is pretty trivial because `BTreeMap<String, serde_json::Value>` happens to already implement the `Source` trait. Now let's assume `BTreeMap<String, serde_json::Value>` didn't implement `Source`. A manual implementation iterating through the map and converting the `(String, serde_json::Value)` pairs into types that can be visited could look like this:

```rust
use log::kv::{
    source::{self, Source},
    value,
};

struct MySource {
    data: BTreeMap<String, serde_json::Value>,
}

impl Source for MySource {
    fn visit<'kvs>(&'kvs self, visitor: &mut dyn source::Visitor<'kvs>) -> Result<(), source::Error> {
        for (k, v) in self.data {
            visitor.visit_pair(source::Key::new(k), source::Value::new(v))
        }
    }
}
```

The `Key::new` method accepts any `T: Borrow<str>`. The `Value::new` accepts any `T: std::fmt::Debug + serde::Serialize`. Values that can't implement `Debug + Serialize` can still be visited using the `source::Value::any` method. This method lets us provide an inline function that will visit the value:

```rust
use log::kv::{
    source::{self, Source},
    value,
};

struct MySource {
    data: BTreeMap<String, MyValue>,
}

impl Source for MySource {
    fn visit<'kvs>(&'kvs self, visitor: &mut dyn source::Visitor<'kvs>) -> Result<(), source::Error> {
        for (k, v) in self.data {
            let key = source::Key::new(k);

            // Let's assume `MyValue` doesn't implement `Serialize`
            // Instead it implements `Display`.
            let value = source::Value::any(v: &MyValue, |v: &MyValue, visitor: &mut dyn value::Visitor| {
                // Let's assume `MyValue` implements `Display`
                visitor.visit_fmt(format_args!("{}", v))
            });

            visitor.visit_pair(key, value)
    }
}
```

The `value::Visitor` trait is similar to `serde::Serializer`, but only supports a few common types:

```rust
trait Visitor {
    fn visit_i64(&mut self, v: i64) -> Result<(), Error>;
    fn visit_u64(&mut self, v: u64) -> Result<(), Error>;
    fn visit_i128(&mut self, v: i128) -> Result<(), Error>;
    fn visit_u128(&mut self, v: u128) -> Result<(), Error>;
    fn visit_f64(&mut self, v: f64) -> Result<(), Error>;
    fn visit_bool(&mut self, v: bool) -> Result<(), Error>;
    fn visit_char(&mut self, v: char) -> Result<(), Error>;
    fn visit_str(&mut self, v: &str) -> Result<(), Error>;
    fn visit_bytes(&mut self, v: &[u8]) -> Result<(), Error>;
    fn visit_none(&mut self) -> Result<(), Error>;
    fn visit_fmt(&mut self, v: &fmt::Arguments) -> Result<(), Error>'
}
```

A `Source` doesn't have to just contain key-value pairs directly like `BTreeMap<String, Value>` though. It could also act like an adapter, like we have for iterators in the standard library. As another example, the following `Source` doesn't store any key-value pairs of its own, it will sort and de-duplicate pairs read from another source by first reading them into a map before forwarding them on:

```rust
use log::kv::source::{self, Source, Visitor};

pub struct SortRetainLast<KVS>(KVS);

impl<KVS> Source for SortRetainLast<KVS>
where
    KVS: Source,
{
    fn visit<'kvs>(&'kvs self, visitor: &mut dyn source::Visitor<'kvs>) -> Result<(), source::Error> {
        // `Seen` is a visitor that will capture key-value pairs
        // in a `BTreeMap`. We use it internally to sort and de-duplicate
        // the key-value pairs that `SortRetainLast` is wrapping.
        struct Seen<'kvs>(BTreeMap<source::Key<'kvs>, source::Value<'kvs>>);

        impl<'kvs> Visitor<'kvs> for Seen<'kvs> {
            fn visit_pair<'vis>(&'vis mut self, k: source::Key<'kvs>, v: source::Value<'kvs>) -> Result<(), source::Error> {
                self.0.insert(k, v);

                Ok(())
            }
        }

        // Visit the inner source and collect its key-value pairs into `seen`
        let mut seen = Seen(BTreeMap::new());
        self.0.visit(&mut seen)?;

        // Iterate through the seen key-value pairs in order
        // and pass them to the `visitor`.
        for (k, v) in seen.0 {
            visitor.visit_pair(k, v)?;
        }

        Ok(())
    }
}
```

## Writing your own `value::Visitor`

Consumers of key-value pairs can visit structured values without needing a `serde::Serializer`. Instead they can implement a `value::Visitor`. A `Visitor` can always visit any structured value by formatting it using its `Debug` implementation:

```rust
use log::kv::value::{self, Value};

struct WriteVisitor<W>(W);

impl<W> value::Visitor for WriteVisitor<W>
where
    W: Write,
{
    fn visit_any(&mut self, v: Value) -> Result<(), value::Error> {
        write!(&mut self.0, "{:?}", v)?;

        Ok(())
    }
}
```

There are other methods besides `visit_any` that can be implemented. By default they all forward to `visit_any`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Design considerations

### Don't break anything

Allow structured logging to be added in the current `0.4.x` series of `log`. This gives us the option of including structured logging in an `0.4.x` release, or bumping the minor crate version without introducing any actual breaking changes.

### Don't create a public dependency

Don't create a new serialization framework that causes `log` to become a public dependency of any library that wants their data types to be loggable. Logging is a truly cross-cutting concern, so if `log` was a public dependency it would probably be at least as pervasive as `serde` is now.

### Support arbitrary producers and arbitrary consumers

Provide an API that's suitable for two independent logging frameworks to integrate through if they want.

### Prioritize end-users of `log!`

There are far more consumers of the `log!` macros that don't need to worry about the internals of the `log` crate than there are log frameworks and sinks that do so it makes sense to prioritize `log!` ergonomics.

### Object safety

`log` is already designed to be object-safe so this new structured logging API needs to be object-safe too.

## Cargo features

Structured logging will be supported in either `std` or `no_std` contexts by default.

```toml
[features]
kv_serde = ["std", "serde", "erased-serde"]
i128 = []
```

### `kv_serde`

Using default features, structured logging will be supported by `log` in `no_std` environments for a fixed set of types from the standard library. Using the `kv_serde` feature, any type that implements `Debug + Serialize` can be logged, and its potentially complex structure will be retained.

### `i128`

Add support for 128bit numbers without bumping `log`'s current minimally supported version of `rustc`.

## Implications for dependents

Dependents of `log` will notice the following:

### Default crate features

The API that's available with default features doesn't add any extra dependencies to the `log` crate, and shouldn't impact compile times or artifact size much.

### After opting in to `kv_serde`

In `no_std` environments (which is the default for `log`):

- `serde` will enter the `Cargo.lock` if it wasn't there already. This will impact compile-times.
- Artifact size of `log` will increase.

In `std` environments (which is common when using `env_logger` and other crates that implement `Log`):

- `serde` and `erased-serde` will enter the `Cargo.lock` if it wasn't there already. This will impact compile-times.
- Artifact size of `log` will increase.

In either case, `serde` will become a public dependency of the `log` crate, so any breaking changes to `serde` will result in breaking changes to `log`.

## Key-values API

### `Error`

Just about the only things you can do with a structured value are format it or serialize it. Serialization and writing might fail, so to allow errors to get carried back to callers there needs to be a general error type that they can early return with:

```rust
pub struct Error(Inner);

enum Inner {
    Static(&'static str),
    #[cfg(feature = "std")]
    Owned(String),
}

impl Error {
    pub fn msg(msg: &'static str) -> Self {
        Error(Inner::Static(msg))
    }

    #[cfg(feature = "std")]
    pub fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        &self.0
    }

    #[cfg(feature = "std")]
    pub fn into_error(self) -> Box<dyn std::error::Error + Send + Sync> {
        Box::new(self.0)
    }

    #[cfg(feature = "kv_serde")]
    pub fn into_serde<E>(self) -> E
    where
        E: serde::ser::Error,
    {
        E::custom(self)
    }
}

#[cfg(feature = "std")]
impl<E> From<E> for Error
where
    E: std::error::Error,
{
    fn from(err: E) -> Self {
        Error(Inner::Owned(err.to_string()))
    }
}

#[cfg(feature = "std")]
impl From<Error> for Box<dyn std::error::Error + Send + Sync> {
    fn from(err: Error) -> Self {
        err.into_error()
    }
}

#[cfg(feature = "std")]
impl From<Error> for io::Error {
    fn from(err: Error) -> Self {

    }
}

impl AsRef<dyn std::error::Error + Send + Sync + 'static> for Error {
    fn as_ref(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        self.as_error()
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Inner {
    fn description(&self) -> &str {
        match self {
            Inner::Static(msg) => msg,
            Inner::Owned(msg) => msg,
        }
    }
}
```

There's no really universal way to handle errors in a logging pipeline. Knowing that some error occurred, and knowing where, should be enough for implementations of `Log` to decide how to handle it. The `Error` type doesn't try to be a general-purpose error management tool, it tries to make it easy to early-return with other errors.

To make it possible to carry any arbitrary `S::Error` type, where we don't know how long the value can live for and whether it's `Send` or `Sync`, without extra work, the `Error` type does not attempt to store the error value itself. It just converts it into a `String`.

### `value::Visit`

The `Visit` trait can be treated like a lightweight subset of `serde::Serialize` that can interoperate with `serde`, without necessarily depending on it:

```rust
/// A type that can be converted into a borrowed value.
pub trait Visit: private::Sealed {
    /// Visit this value.
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error>;

    /// Convert a reference to this value into an erased `Value`.
    fn to_value(&self) -> Value
    where
        Self: Sized,
    {
        Value::new(self)
    }
}

mod private {
    #[cfg(not(feature = "kv_serde"))]
    pub trait Sealed: Debug {}

    #[cfg(feature = "kv_serde")]
    pub trait Sealed: Debug + Serialize {}
}
```

We'll look at the `Visitor` trait shortly. It's like `serde::Serializer`.

`Visit` is the trait bound that structured values need to satisfy before they can be logged. The trait can't be implemented outside of the `log` crate, because it uses blanket implementations depending on Cargo features. If a crate defines a datastructure that users might want to log, instead of trying to implement `Visit`, it should implement the `serde::Serialize` and `std::fmt::Debug` traits. This means that `Visit` can piggyback off `serde::Serialize` as the pervasive public dependency, so that `Visit` itself doesn't need to be one.

The trait bounds on `private::Sealed` ensure that any generic `T: Visit` carries some additional traits that are needed for the blanket implementation of `Serialize`. As an example, any `Option<T: Visit>` can also be treated as `Option<T: Serialize>` and therefore implement `Serialize` itself. The `Visit` trait is responsible for a lot of type system mischief.

With default features, the types that implement `Visit` are a subset of `T: Debug + Serialize`:

```
-------- feature = "kv_serde" --------
|                                    |
|        T: Debug + Serialize        |
|                                    |
|                                    |
|   - not(feature = "kv_serde") -    |
|   |                           |    |
|   | u8, i8, &str, &[u8], bool |    |
|   | etc...                    |    |
|   |                           |    |
|   -----------------------------    |
|                                    |
|                                    |
--------------------------------------
```

The full set of standard types that implement `Visit` are:

- Standard formats: `Arguments`
- Primitives: `bool`, `char`
- Unsigned integers: `u8`, `u16`, `u32`, `u64`, `u128`
- Signed integers: `i8`, `i16`, `i32`, `i64`, `i128`
- Strings: `&str`, `String`
- Bytes: `&[u8]`, `Vec<u8>`
- Paths: `Path`, `PathBuf`
- Special types: `Option<T>` and `()`.

Enabling the `kv_serde` feature expands the set of types that implement `Visit` from this subset to all `T: Debug + Serialize`.

#### Object safety

The `Visit` trait is not object-safe, but has a simple object-safe wrapper used by `Value`.

#### Without `serde`

Without the `kv_serde` feature, the `Visit` trait is implemented for a fixed set of fundamental types from the standard library:

```rust
impl Visit for u8 {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        visitor.visit_u64(*self as u64)
    }
}

impl Visit for u16 {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        visitor.visit_u64(*self as u64)
    }
}

impl Visit for u32 {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        visitor.visit_u64(*self as u64)
    }
}

impl Visit for u64 {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        visitor.visit_u64(*self)
    }
}

impl Visit for i8 {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        visitor.visit_i64(*self as i64)
    }
}

impl Visit for i16 {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        visitor.visit_i64(*self as i64)
    }
}

impl Visit for i32 {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        visitor.visit_i64(*self as i64)
    }
}

impl Visit for i64 {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        visitor.visit_i64(*self)
    }
}

impl Visit for f32 {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        visitor.visit_f64(*self as f64)
    }
}

impl Visit for f64 {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        visitor.visit_f64(*self)
    }
}

impl Visit for char {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        visitor.visit_char(*self)
    }
}

impl Visit for bool {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        visitor.visit_bool(*self)
    }
}

impl Visit for () {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        visitor.visit_none()
    }
}

#[cfg(feature = "i128")]
impl Visit for u128 {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        visitor.visit_u128(*self)
    }
}

#[cfg(feature = "i128")]
impl Visit for i128 {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        visitor.visit_i128(*self)
    }
}

impl<T> Visit for Option<T>
where
    T: Visit,
{
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        match self {
            Some(v) => v.visit(visitor),
            None => visitor.visit_none(),
        }
    }
}

impl<'a> Visit for fmt::Arguments<'a> {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        visitor.visit_fmt(self)
    }
}

impl<'a> Visit for &'a str {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        visitor.visit_str(self)
    }
}

impl<'a> Visit for &'a [u8] {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        visitor.visit_bytes(self)
    }
}

impl<'v> Visit for Value<'v> {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        self.visit(visitor)
    }
}

#[cfg(feature = "std")]
impl<T: ?Sized> Visit for Box<T>
where
    T: Visit
{
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        (**self).visit(visitor)
    }
}

#[cfg(feature = "std")]
impl<'a> Visit for &'a Path {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        match self.to_str() {
            Some(s) => visitor.visit_str(s),
            None => visitor.visit_fmt(&format_args!("{:?}", self)),
        }
    }
}

#[cfg(feature = "std")]
impl Visit for String {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        visitor.visit_str(&*self)
    }
}

#[cfg(feature = "std")]
impl Visit for Vec<u8> {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        visitor.visit_bytes(&*self)
    }
}

#[cfg(feature = "std")]
impl Visit for PathBuf {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        self.as_path().visit(visitor)
    }
}
```

#### With `serde`

With the `kv_serde` feature, the `Visit` trait is implemented for any type that is `Debug + Serialize`:

```rust
#[cfg(feature = "kv_serde")]
impl<T: ?Sized> Visit for T
where
    T: Debug + Serialize {}
```

#### Ensuring the fixed set is a subset of the blanket implementation

Changing trait implementations based on Cargo features is a dangerous game. Cargo features are additive, so any observable changes to trait implementations must also be purely additive, otherwise you can end up with libraries that can't compile if a feature is active. This can be very subtle when references and generics are involved.

When the `kv_serde` feature is active, the implementaiton of `Visit` changes from a fixed set to an open one. We have to guarantee that the open set is a superset of the fixed one. That means any valid `T: Visit` without the `kv_serde` feature remains a valid `T: Visit` with the `kv_serde` feature.

There are a few ways we could achieve this, depending on the quality of the docs we want to produce.

For more readable documentation at the risk of incorrectly implementing `Visit`, we can use a private trait like `EnsureVisit: Visit` that is implemented alongside the concrete `Visit` trait regardless of any blanket implementations of `Visit`:

```rust
// The blanket implemention of `Visit` when `kv_serde` is enabled
#[cfg(feature = "kv_serde")]
impl<T: ?Sized> Visit for T where T: Debug + Serialize {}

/// This trait is a private implementation detail for testing.
/// 
/// All it does is make sure that our set of concrete types
/// that implement `Visit` always implement the `Visit` trait,
/// regardless of crate features and blanket implementations.
trait EnsureVisit: Visit {}

// Ensure any reference to a `Visit` implements `Visit`
impl<'a, T> EnsureVisit for &'a T where T: Visit {}

// These impl blocks always exists
impl<T> EnsureVisit for Option<T> where T: Visit {}
// This impl block only exists if the `kv_serde` isn't active
#[cfg(not(feature = "kv_serde"))]
impl<T> private::Sealed for Option<T> where T: Visit {}
#[cfg(not(feature = "kv_serde"))]
impl<T> Visit for Option<T> where T: Visit {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {

    }
}
```

In the above example, we can ensure that `Option<T: Visit>` always implements the `Visit` trait, whether it's done manually or as part of a blanket implementation. All types that implement `Visit` manually with any `#[cfg]` _must_ also always implement `EnsureVisit` manually (with no `#[cfg]`) with the exact same type bounds. It's pretty subtle, but the subtlety can be localized to a single module within the `log` crate so it can be managed.

Using a trait for this type checking means the `impl Visit for Option<T>` and `impl EnsureVisit for Option<T>` can be wrapped up in a macro so that we never miss adding them. The below macro is an example of a (not very pretty) one that can add the needed implementations of `EnsureVisit` along with the regular `Visit`:

```rust
macro_rules! impl_to_value {
    () => {};
    (
        impl: { $($params:tt)* }
        where: { $($where:tt)* }
        $ty:ty: { $($serialize:tt)* }
        $($rest:tt)*
    ) => {
        impl<$($params)*> EnsureVisit for $ty
        where
            $($where)* {}
        
        #[cfg(not(feature = "kv_serde"))]
        impl<$($params)*> private::Sealed for $ty
        where
            $($where)* {}

        #[cfg(not(feature = "kv_serde"))]
        impl<$($params)*> Visit for $ty
        where
            $($where)*
        {
            $($serialize)*
        }

        impl_to_value!($($rest)*);
    };
    (
        impl: { $($params:tt)* }
        $ty:ty: { $($serialize:tt)* } 
        $($rest:tt)*
    ) => {
        impl_to_value! {
            impl: {$($params)*} where: {} $ty: { $($serialize)* } $($rest)*
        }
    };
    (
        $ty:ty: { $($serialize:tt)* } 
        $($rest:tt)*
    ) => {
        impl_to_value! {
            impl: {} where: {} $ty: { $($serialize)* } $($rest)*
        }
    }
}

// Ensure any reference to a `Visit` is also `Visit`
impl<'a, T> EnsureVisit for &'a T where T: Visit {}

impl_to_value! {
    u8: {
        fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
            visitor.visit_u64(*self as u64)
        }
    }

    impl: { T: Visit } Option<T>: {
        fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
            match self {
                Some(v) => v.to_value().visit(visitor),
                None => visitor.visit_none(),
            }
        }
    }

    ...
}
```

We don't necessarily need a macro to make new implementations accessible for new contributors safely though.

##### What about specialization?

In a future Rust with specialization we might be able to avoid all the machinery needed to keep the manual impls consistent with the blanket one, and allow consumers to implement `Visit` without needing `serde`. The specifics of specialization are still up in the air though. Under the proposed _always applicable_ rule, manual implementations like `impl<T> Visit for Option<T> where T: Visit` wouldn't be allowed. The ` where specialize(T: Visit)` scheme might make it possible though, although this would probably be a breaking change in any case.

### `value::Visitor`

A visitor for a `Visit` that can interogate its structure:

```rust
/// A serializer for primitive values.
pub trait Visitor {
    /// Visit an arbitrary value.
    /// 
    /// Depending on crate features there are a few things
    /// you can do with a value. You can:
    /// 
    /// - format it using `Debug`.
    /// - serialize it using `serde`.
    fn visit_any(&mut self, v: Value) -> Result<(), Error>;

    /// Visit a signed integer.
    fn visit_i64(&mut self, v: i64) -> Result<(), Error> {
        self.visit_any(v.to_value())
    }

    /// Visit an unsigned integer.
    fn visit_u64(&mut self, v: u64) -> Result<(), Error> {
        self.visit_any(v.to_value())
    }

    /// Visit a 128bit signed integer.
    #[cfg(feature = "i128")]
    fn visit_i128(&mut self, v: i128) -> Result<(), Error> {
        self.visit_any(v.to_value())
    }

    /// Visit a 128bit unsigned integer.
    #[cfg(feature = "i128")]
    fn visit_u128(&mut self, v: u128) -> Result<(), Error> {
        self.visit_any(v.to_value())
    }

    /// Visit a floating point number.
    fn visit_f64(&mut self, v: f64) -> Result<(), Error> {
        self.visit_any(v.to_value())
    }

    /// Visit a boolean.
    fn visit_bool(&mut self, v: bool) -> Result<(), Error> {
        self.visit_any(v.to_value())
    }

    /// Visit a single character.
    fn visit_char(&mut self, v: char) -> Result<(), Error> {
        let mut b = [0; 4];
        self.visit_str(&*v.encode_utf8(&mut b))
    }

    /// Visit a UTF8 string.
    fn visit_str(&mut self, v: &str) -> Result<(), Error> {
        self.visit_any((&v).to_value())
    }

    /// Visit a raw byte buffer.
    fn visit_bytes(&mut self, v: &[u8]) -> Result<(), Error> {
        self.visit_any((&v).to_value())
    }

    /// Visit standard arguments.
    fn visit_none(&mut self) -> Result<(), Error> {
        self.visit_any(().to_value())
    }

    /// Visit standard arguments.
    fn visit_fmt(&mut self, v: &fmt::Arguments) -> Result<(), Error> {
        self.visit_any(v.to_value())
    }
}

impl<'a, T: ?Sized> Visitor for &'a mut T
where
    T: Visitor,
{
    fn visit_any(&mut self, v: Value) -> Result<(), Error> {
        (**self).visit_any(v)
    }

    fn visit_i64(&mut self, v: i64) -> Result<(), Error> {
        (**self).visit_i64(v)
    }

    fn visit_u64(&mut self, v: u64) -> Result<(), Error> {
        (**self).visit_u64(v)
    }

    #[cfg(feature = "i128")]
    fn visit_i128(&mut self, v: i128) -> Result<(), Error> {
        (**self).visit_i128(v)
    }

    #[cfg(feature = "i128")]
    fn visit_u128(&mut self, v: u128) -> Result<(), Error> {
        (**self).visit_u128(v)
    }

    fn visit_f64(&mut self, v: f64) -> Result<(), Error> {
        (**self).visit_f64(v)
    }

    fn visit_bool(&mut self, v: bool) -> Result<(), Error> {
        (**self).visit_bool(v)
    }

    fn visit_char(&mut self, v: char) -> Result<(), Error> {
        (**self).visit_char(v)
    }

    fn visit_str(&mut self, v: &str) -> Result<(), Error> {
        (**self).visit_str(v)
    }

    fn visit_bytes(&mut self, v: &[u8]) -> Result<(), Error> {
        (**self).visit_bytes(v)
    }

    fn visit_none(&mut self) -> Result<(), Error> {
        (**self).visit_none()
    }

    fn visit_fmt(&mut self, args: &fmt::Arguments) -> Result<(), Error> {
        (**self).visit_fmt(args)
    }
}
```

### `Value`

A `Value` is an erased container for a `Visit`, with a potentially short-lived lifetime:

```rust
/// The value in a key-value pair.
pub struct Value<'v>(ValueInner<'v>);

enum ValueInner<'v> {
    Erased(&'v dyn ErasedVisit),
    Any(Any<'v>),
}

impl<'v> Value<'v> {
    /// Create a value.
    pub fn new(v: &'v impl Visit) -> Self {
        Value(ValueInner::Erased(v))
    }

    /// Create a value from an anonymous type.
    /// 
    /// The value must be provided with a compatible visit method.
    pub fn any<T>(v: &'v T, visit: fn(&T, &mut dyn Visitor) -> Result<(), Error>) -> Self
    where
        T: 'static,
    {
        Value(ValueInner::Any(Any::new(v, visit)))
    }

    /// Visit the contents of this value with a visitor.
    pub fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        match self.0 {
            ValueInner::Erased(v) => v.erased_visit(visitor),
            ValueInner::Any(ref v) => v.visit(visitor),
        }
    }
}
```

#### `ErasedVisit`

The `ErasedVisit` trait is an object-safe wrapper for the `Visit` trait. `Visit` itself isn't technically object-safe because it needs the non-object-safe `serde::Serialize` as a supertrait to carry in generic contexts:

```rust
#[cfg(not(feature = "kv_serde"))]
trait ErasedVisit: fmt::Debug {
    fn erased_visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error>;
}

#[cfg(feature = "kv_serde")]
trait ErasedVisit: fmt::Debug + erased_serde::Serialize {
    fn erased_visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error>;
}

impl<T: ?Sized> ErasedVisit for T
where
    T: Visit,
{
    fn erased_visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        self.visit(visitor)
    }
}
```

#### `Any`

Other logging frameworks that want to integrate with `log` might not want to pull in a `serde` dependency, and so they couldn't implement the `Visit` trait. The `Any` type uses some `std::fmt` inspired black-magic to allow values that don't implement the `Visit` trait to be erased in a `Value`. It does this by taking a borrowed value along with a function pointer that looks like `Visit::visit`:

```rust
struct Void {
    _priv: (),
    _oibit_remover: PhantomData<*mut dyn Fn()>,
}

struct Any<'a> {
    data: &'a Void,
    visit: fn(&Void, &mut dyn Visitor) -> Result<(), Error>,
}

impl<'a> Any<'a> {
    fn new<T>(data: &'a T, visit: fn(&T, &mut dyn Visitor) -> Result<(), Error>) -> Self
    where
        T: 'static,
    {
        unsafe {
            Any {
                data: mem::transmute::<&'a T, &'a Void>(data),
                visit: mem::transmute::<
                    fn(&T, &mut dyn Visitor) -> Result<(), Error>,
                    fn(&Void, &mut dyn Visitor) -> Result<(), Error>>
                    (visit),
            }
        }
    }

    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {
        (self.visit)(self.data, visitor)
    }
}
```

There's some scary code in `Any`, which is really just something like an ad-hoc trait object.

#### Formatting

`Value` always implements `Debug` and `Display` by forwarding to its inner value:

```rust
impl<'v> fmt::Debug for Value<'v> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            ValueInner::Erased(v) => v.fmt(f),
            ValueInner::Any(ref v) => {
                struct ValueFmt<'a, 'b>(&'a mut fmt::Formatter<'b>);

                impl<'a, 'b> Visitor for ValueFmt<'a, 'b> {
                    fn visit_any(&mut self, v: Value) -> Result<(), Error> {
                        write!(self.0, "{:?}", v)?;

                        Ok(())
                    }
                }

                let mut visitor = ValueFmt(f);
                v.visit(&mut visitor).map_err(|_| fmt::Error)
            }
        }
    }
}

impl<'v> fmt::Display for Value<'v> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
```

#### Serialization

When the `kv_serde` feature is enabled, `Value` implements the `serde::Serialize` trait by forwarding to its inner value:

```rust
impl<'v> Serialize for Value<'v> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            ValueInner::Erased(v) => {
                erased_serde::serialize(v, serializer)
            },
            ValueInner::Any(ref v) => {
                struct ErasedVisitSerde<S: Serializer> {
                    serializer: Option<S>,
                    ok: Option<S::Ok>,
                }

                impl<S> Visitor for ErasedVisitSerde<S>
                where
                    S: Serializer,
                {
                    fn visit_any(&mut self, v: Value) -> Result<(), Error> {
                        let ok = v.serialize(self.serializer.take().expect("missing serializer"))?;
                        self.ok = Some(ok);

                        Ok(())
                    }
                }

                let mut visitor = ErasedVisitSerde {
                    serializer: Some(serializer),
                    ok: None,
                };

                v.visit(&mut visitor).map_err(|e| e.into_serde())?;
                Ok(visitor.ok.expect("missing return value"))
            },
        }
    }
}
```

#### Ownership

The `Value` type borrows from its inner value.

#### Thread-safety

The `Value` type doesn't try to guarantee that values are `Send` or `Sync`, and doesn't offer any way of retaining that information when erasing.

### `Key`

A `Key` is a short-lived structure that can be represented as a UTF-8 string. This might be possible without allocating, or it might require a destination to write into:

```rust
/// A key in a key-value pair.
/// 
/// The key can be treated like `&str`.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key<'kvs> {
    inner: &'kvs str,
}

impl<'kvs> Borrow<str> for Key<'kvs> {
    fn to_key(&self) -> Key {
        Key { inner: self.inner }
    }
}

impl<'kvs> Key<'kvs> {
    /// Get a `Key` from a borrowed string.
    pub fn from_str(key: &'kvs (impl AsRef<str> + ?Sized)) -> Self {
        Key {
            inner: key.as_ref(),
        }
    }

    /// Get a borrowed string from a `Key`.
    pub fn as_str(&self) -> &str {
        &self.inner
    }
}

impl<'kvs> AsRef<str> for Key<'kvs> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(feature = "std")]
impl<'kvs> Borrow<str> for Key<'kvs> {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl<'kvs> Serialize for Key<'kvs> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.inner)
    }
}

impl<'kvs> Display for Key<'kvs> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<'kvs> Debug for Key<'kvs> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.inner.fmt(f)
    }
}
```

Other standard implementations could be added for any `K: Borrow<str>` in the same fashion.

#### Ownership

The `Key` type can either borrow or own its inner value.

#### Thread-safety

The `Key` type is probably `Send` + `Sync`, but that's not guaranteed.

### `source::Visitor`

The `Visitor` trait used by `Source` can visit a single key-value pair:

```rust
pub trait Visitor<'kvs> {
    fn visit_pair(&mut self, k: Key<'kvs>, v: Value<'kvs>) -> Result<(), Error>;
}

impl<'a, 'kvs, T: ?Sized> Visitor<'kvs> for &'a mut T
where
    T: Visitor<'kvs>,
{
    fn visit_pair(&mut self, k: Key<'kvs>, v: Value<'kvs>) -> Result<(), Error> {
        (*self).visit_pair(k, v)
    }
}
```

A `Visitor` may serialize the keys and values as it sees them. It may also do other work, like sorting or de-duplicating them. Operations that involve ordering keys will probably require allocations.

#### Implementors

There aren't any public implementors of `Visitor` in the `log` crate. Other crates that use key-value pairs will implement `Visitor`.

#### Object safety

The `Visitor` trait is object-safe.

### `Source`

The `Source` trait is a bit like `std::iter::Iterator`. It gives us a way to inspect some arbitrary collection of key-value pairs using an object-safe visitor pattern:

```rust
pub trait Source {
    fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error>;
}

impl<'a, T: ?Sized> Source for &'a T
where
    T: Source,
{
    fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error> {
        (*self).visit(visitor)
    }
}
```

`Source` doesn't make any assumptions about how many key-value pairs it contains or how they're visited. That means the visitor may observe keys in any order, and observe the same key multiple times.

#### Ownership

The `Source` trait is probably the point where having some way to convert from a borrowed to an owned variant would make the most sense.

We could add a method to `Source` that allowed it to be converted into an owned variant with a default implementation:

```rust
pub trait Source {
    fn to_owned(&self) -> OwnedSource {
        OwnedSource::serialized(self)
    }
}
```

The `OwnedSource` could then encapsulte some sharable `dyn Source + Send + Sync`:

```rust
#[derive(Clone)]
pub struct OwnedSource(Arc<dyn Source + Send + Sync>);

impl OwnedSource {
    fn new(impl Into<Arc<dyn Source + Send + Sync>>) -> Self {
        OwnedSource(source.into())
    }

    fn serialize(impl Source) -> Self {
        // Serialize the `Source` to something like
        // `Vec<(String, OwnedValue)>`
        // where `OwnedValue` is like `serde_json::Value`
        ...
    }
}
```

Other implementations of `Source` are encouraged to override the `to_owned` method if they could provide a more efficient implementation. As an example, if there's a `Source` that is already wrapped up in an `Arc` then it can implement `to_owned` by just cloning itself.

#### Adapters

Some useful adapters exist as provided methods on the `Source` trait. They're similar to adapters on the standard `Iterator` trait:

```rust
pub trait Source {
    ...

    /// Erase this `Source` so it can be used without
    /// requiring generic type parameters.
    fn erase(&self) -> ErasedSource
    where
        Self: Sized,
    {
        ErasedSource::erased(self)
    }

    /// An adapter to borrow self.
    fn by_ref(&self) -> &Self {
        self
    }

    /// Chain two `Source`s together.
    fn chain<KVS>(self, other: KVS) -> Chained<Self, KVS>
    where
        Self: Sized,
    {
        Chained(self, other)
    }

    /// Find the value for a given key.
    /// 
    /// If the key is present multiple times, this method will
    /// return the *last* value for the given key.
    /// 
    /// The default implementation will scan all key-value pairs.
    /// Implementors are encouraged provide a more efficient version
    /// if they can. Standard collections like `BTreeMap` and `HashMap`
    /// will do an indexed lookup instead of a scan.
    fn get<'kvs, Q>(&'kvs self, key: Q) -> Option<Value<'kvs>>
    where
        Q: Borrow<str>,
    {
        struct Get<'k, 'v>(Key<'k>, Option<Value<'v>>);

        impl<'k, 'kvs> Visitor<'kvs> for Get<'k, 'kvs> {
            fn visit_pair(&mut self, k: Key<'kvs>, v: Value<'kvs>) -> Result<(), Error> {
                if k == self.0 {
                    self.1 = Some(v);
                }

                Ok(())
            }
        }

        let mut visitor = Get(key.to_key(), None);
        let _ = self.visit(&mut visitor);

        visitor.1
    }

    /// Apply a function to each key-value pair.
    fn try_for_each<F, E>(self, f: F) -> Result<(), Error>
    where
        Self: Sized,
        F: FnMut(Key, Value) -> Result<(), E>,
        E: Into<Error>,
    {
        struct ForEach<F, E>(F, std::marker::PhantomData<E>);

        impl<'kvs, F, E> Visitor<'kvs> for ForEach<F, E>
        where
            F: FnMut(Key, Value) -> Result<(), E>,
            E: Into<Error>,
        {
            fn visit_pair(&mut self, k: Key<'kvs>, v: Value<'kvs>) -> Result<(), Error> {
                (self.0)(k, v).map_err(Into::into)
            }
        }

        self.visit(&mut ForEach(f, Default::default()))
    }

    /// Serialize the key-value pairs as a map.
    #[cfg(feature = "kv_serde")]
    fn serialize_as_map(self) -> SerializeAsMap<Self>
    where
        Self: Sized,
    {
        SerializeAsMap(self)
    }

    /// Serialize the key-value pairs as a map.
    #[cfg(feature = "kv_serde")]
    fn serialize_as_seq(self) -> SerializeAsSeq<Self>
    where
        Self: Sized,
    {
        SerializeAsSeq(self)
    }
}
```

- `by_ref` to get a reference to a `Source` within a method chain.
- `chain` to concatenate one source with another. This is useful for composing implementations of `Log` together for contextual logging.
- `get` to try find the value associated with a key.
- `try_for_each` to try execute some closure over all key-value pairs. This is a convenient way to do something with each key-value pair without having to create and implement a `Visitor`.
- `serialize_as_map` to get a serializable map. This is a convenient way to serialize key-value pairs without having to create and implement a `Visitor`.
- `serialize_as_seq` to get a serializable sequence of tuples. This is a convenient way to serialize key-value pairs without having to create and implement a `Visitor`.

None of these methods are required for the core API. They're helpful tools for working with key-value pairs with minimal machinery. Even if we don't necessarily include them right away it's worth having an API that can support them later without breakage.

#### Object safety

`Source` is not object-safe because of the provided adapter methods not being object-safe. The only required method, `visit`, is safe though, so an object-safe version of `Source` that forwards this method can be reasonably written.

```rust
/// An erased `Source`.
#[derive(Clone)]
pub struct ErasedSource<'a>(&'a dyn ErasedSourceBridge);

impl<'a> ErasedSource<'a> {
    /// Capture a `Source` and erase its concrete type.
    pub fn new(kvs: &'a impl Source) -> Self {
        ErasedSource(kvs)
    }
}

impl<'a> Default for ErasedSource<'a> {
    fn default() -> Self {
        ErasedSource(&(&[] as &[(&str, &dyn Visit)]))
    }
}

impl<'a> Source for ErasedSource<'a> {
    fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error> {
        self.0.erased_visit(visitor)
    }

    fn get<'kvs, Q>(&'kvs self, key: Q) -> Option<Value<'kvs>>
    where
        Q: Borrow<str>,
    {
        let key = key.to_key();
        self.0.erased_get(key.as_ref())
    }
}

/// A trait that erases a `Source` so it can be stored
/// in a `Record` without requiring any generic parameters.
trait ErasedSourceBridge {
    fn erased_visit<'kvs>(&'kvs self, visitor: &mut dyn Visitor<'kvs>) -> Result<(), Error>;
    fn erased_get<'kvs>(&'kvs self, key: &str) -> Option<Value<'kvs>>;
}

impl<KVS> ErasedSourceBridge for KVS
where
    KVS: Source + ?Sized,
{
    fn erased_visit<'kvs>(&'kvs self, visitor: &mut dyn Visitor<'kvs>) -> Result<(), Error> {
        self.visit(visitor)
    }

    fn erased_get<'kvs>(&'kvs self, key: &str) -> Option<Value<'kvs>> {
        self.get(key)
    }
}
```

#### Implementors

A `Source` containing a single key-value pair is implemented for a tuple of a key and value:

```rust
impl<K, V> Source for (K, V)
where
    K: Borrow<str>,
    V: Visit,
{
    fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error> {
        visitor.visit_pair(self.0.to_key(), self.1.to_value())
    }
}
```

A `Source` with multiple pairs is implemented for arrays of `Source`s:

```rust
impl<KVS> Source for [KVS] where KVS: Source {
    fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error> {
        for kv in self {
            kv.visit(&mut visitor)?;
        }

        Ok(())
    }
}
```

When `std` is available, `Source` is implemented for some standard collections too:

```rust
#[cfg(feature = "std")]
impl<KVS: ?Sized> Source for Box<KVS> where KVS: Source {
    fn visit<'kvs>(&'kvs self, visitor: &mut dyn Visitor<'kvs>) -> Result<(), Error> {
        (**self).visit(visitor)
    }
}

#[cfg(feature = "std")]
impl<KVS: ?Sized> Source for Arc<KVS> where KVS: Source  {
    fn visit<'kvs>(&'kvs self, visitor: &mut dyn Visitor<'kvs>) -> Result<(), Error> {
        (**self).visit(visitor)
    }
}

#[cfg(feature = "std")]
impl<KVS: ?Sized> Source for Rc<KVS> where KVS: Source  {
    fn visit<'kvs>(&'kvs self, visitor: &mut dyn Visitor<'kvs>) -> Result<(), Error> {
        (**self).visit(visitor)
    }
}

#[cfg(feature = "std")]
impl<KVS> Source for Vec<KVS> where KVS: Source {
    fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error> {
        self.as_slice().visit(visitor)
    }
}

#[cfg(feature = "std")]
impl<K, V> Source for collections::BTreeMap<K, V>
where
    K: Borrow<str> + Ord,
    V: Visit,
{
    fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error> {
        for (k, v) in self {
            visitor.visit_pair(k.to_key(), v.to_value())?;
        }

        Ok(())
    }

    fn get<'kvs, Q>(&'kvs self, key: Q) -> Option<Value<'kvs>>
    where
        Q: Borrow<str>,
    {
        let key = key.to_key();
        collections::BTreeMap::get(self, key.as_ref()).map(Visit::to_value)
    }
}

#[cfg(feature = "std")]
impl<K, V> Source for collections::HashMap<K, V>
where
    K: Borrow<str> + Eq + Hash,
    V: Visit,
{
    fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error> {
        for (k, v) in self {
            visitor.visit_pair(k.to_key(), v.to_value())?;
        }

        Ok(())
    }

    fn get<'kvs, Q>(&'kvs self, key: Q) -> Option<Value<'kvs>>
    where
        Q: Borrow<str>,
    {
        let key = key.to_key();
        collections::HashMap::get(self, key.as_ref()).map(Visit::to_value)
    }
}
```

The `BTreeMap` and `HashMap` implementations provide more efficient implementations of `Source::get`.

### `Record` and `RecordBuilder`

Structured key-value pairs can be set on a `RecordBuilder`:

```rust
impl<'a> RecordBuilder<'a> {
    /// Set key values
    pub fn key_values(&mut self, kvs: ErasedSource<'a>) -> &mut RecordBuilder<'a> {
        self.record.kvs = kvs;
        self
    }
}
```

These key-value pairs can then be accessed on the built `Record`:

```rust
#[derive(Clone, Debug)]
pub struct Record<'a> {
    ...

    kvs: ErasedSource<'a>,
}

impl<'a> Record<'a> {
    /// The key value pairs attached to this record.
    /// 
    /// Pairs aren't guaranteed to be unique (the same key may be repeated with different values).
    pub fn key_values(&self) -> ErasedSource {
        self.kvs.clone()
    }
}
```

## The `log!` macros

The `log!` macro will initially support a fairly spartan syntax for capturing structured data. The current `log!` macro looks like this:

```rust
log!(<unstructured message>);
```

This RFC proposes an additional semi-colon-separated part of the macro for capturing key-value pairs: 

```rust
log!(<unstructured message> ; <structured data>)
```

The `;` and structured values are optional. If they're not present then the behaviour of the `log!` macro is the same as it is today.

As an example, this is what a `log!` statement containing structured key-value pairs could look like:

```rust
info!(
    "This is the rendered {message}. It is not structured",
    message = "message";
    correlation = correlation_id,
    user = user
);
```

There's a *big* design space around the syntax for capturing log records we could explore, especially when you consider procedural macros. The syntax proposed here for the `log!` macro is not designed to be really ergonomic. It's designed to be *ok*, and to encourage an exploration of the design space by offering a consistent base that other macros could build off.

Having said that, there are a few unintrusive quality-of-life features that make the `log!` macros nicer to use with structured data.

### Expansion

Styructured key-value pairs in the `log!` macro expand to statements that borrow from their environment.

```rust
info!(
    "This is the rendered {message}. It is not structured",
    message = "message";
    correlation = correlation_id,
    user = user
);
```

Will expand to something like:

```rust
{
    let lvl = log::Level::Info;

    if lvl <= ::STATIC_MAX_LEVEL && lvl <= ::max_level() {
        let correlation &correlation_id;
        let user = &user;

        let kvs: &[(&str, &dyn::key_values::Visit)] =
            &[("correlation", &correlation), ("user", &user)];

        ::__private_api_log(
            ::std::fmt::Arguments::new_v1(
                &["This is the rendered ", ". It is not structured"],
                &match (&"message",) {
                    (arg0,) => [::std::fmt::ArgumentV1::new(arg0, ::std::fmt::Display::fmt)],
                },
            ),
            lvl,
            &("bin", "mod", "mod.rs", 13u32),
            &kvs,
        );
    }
};
```

# Drawbacks, rationale, and alternatives
[drawbacks]: #drawbacks

Structured logging is a non-trivial feature to support. It adds complexity and overhead to the `log` crate.

## The `Debug + Serialize` blanket implementation of `Visit`

Making sure the `Visit` trait doesn't drop any implementations when the blanket implementation from `kv_serde` replaces the concrete ones is subtle and nonstandard. We have to be especially careful of references and generics. Any mistakes made here can result in dependencies that become uncompilable depending on Cargo features with no workaround besides removing that impl. Using a macro to define the small fixed set, and keeping all impls local to a single module, could help catch these cases.

It's also possibly surprising that the way the `Visit` trait is implemented in the ecosystem is through an entirely unrelated combination of `serde` and `std` traits. At least it's surprising on the surface. For libraries that define loggable types, they just implement some standard traits for serialization without involving `log` at all. These are traits they should be considering anyway. For consumers of the  `log!` macro, they are mostly going to capture structured values for types they didn't produce, so having `serde` as the answer to _how can I log a `Url`, or a `Uuid`?_ sounds reasonable. It also means libraries defining types like `Url` and `Uuid` don't have yet another public serialization trait to implement.

If a library provides a datatype that you'd reasonably want to log, but it doesn't implement `serde::Serialize` then adding support for that type isn't just beneficial to you, but to anyone else that might want to serialize that type.

The real question for `serde` is whether or not depending on it as the general serialization framework in `log` creates the potential for some kind of ecosystem dichotomy if an alternative framework becomes popular where half the ecosystem uses `serde` and the other half uses something else that's incompatible. In that case `log` might not reasonably be able to support both without breakage if it goes down this path. The options for mitigating this in the design now is by either require all loggable types implement `Visit` explicitly, or just requiring callers opt in to `serde` support at the callsite in `log!`.

### Require all loggable types implement `Visit`

We could entirely punt on `serde` and just provide an API for simple values that implement the simple `Visit` trait. That avoids the potential serialization dichotomy in `log` altogether.

The problem here is that any pervasive public API has the chance to create rifts in the ecosystem. By creating a new fundamental API for logging via the `Visit` trait we're just expanding the potential for dichotomies.

It also means we need to re-invent `serde`'s support for complex datastructures, the datatypes that implement its traits, and the formats that support it. We'll effectively turn `log` into a serialization framework of its own, and have to introduce arbitrary limitations on the kinds of values that can be logged.

### Require callers opt in to `serde` support at the callsite

We could avoid a potential serialization dichotomy by requiring callers opt in to `serde` support. That way if a new framework came along it could be naturally supported in the same way. There are a few ways callers could opt in to `serde` in the `log!` macros. The specifics aren't really important, but it could look something like this:

```rust
use log::log_serde;

info!("A message"; user = log_serde!(user));
```

That way an alternative framework could be supported as:

```rust
use log::log_other_framework;

info!("A message"; user = log_other_framework!(user));
```

The problem with this approach is that it puts extra barriers in front of users that want to log. Instead of enabling crate features once and then logging structured values, each log statement needs to know how it can capture values. It also passes the burden of dealing with dichotomies onto every consumer of `log`. It seems like a reasonable idea from the perspective of the `log` crate, but is more hostile to end-users.

There are substantially more end-users of the `log` crate calling the `log!` macros than there are frameworks and sinks that need to interact with its API so it's worth prioritizing end-user experience. Anything that requires end-users to opt-in to the most common scenarios isn't ideal.

# Prior art
[prior-art]: #prior-art

Structured logging is a paradigm that's supported by logging frameworks in many language ecosystems.

## Rust

The `slog` library is a structured logging framework for Rust. Its API predates a stable `serde` crate so it defines its own traits that are similar to `serde::Serialize`. A log record consists of a rendered message and bag of structured key-value pairs. `slog` goes further than this RFC proposes by requiring callers of its `log!` macros to state whether key-values are owned or borrowed by the record, and whether the data is safe to share across threads.

This RFC proposes an API that's inspired by `slog`, but doesn't directly support distinguishing between owned or borrowed key-value pairs. Everything is borrowed. That means the only way to send a `Record` to another thread is to serialize it into a different type.

## Go

The `logrus` library is a structured logging framework for Go. It uses a similar separation of the textual log message from structured key-value pairs that this API proposes.

## .NET

The C# community has mostly standardised around using message templates for packaging a log message with structured key-value pairs. Instead of logging a rendered message and separate bag of structured data, the log record contains a template that allows key-value pairs to be interpolated from the same bag of structured data. It avoids duplicating the same information multiple times.

Supporting something like message templates in Rust using the `log!` macros would probably require procedural macros. A macro like that could be built on top of the API proposed by this RFC.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

# Appendix

## Public API

For context, ignoring the `log!` macros, this is roughly the additional public API this RFC proposes to support structured logging:

```rust
impl<'a> RecordBuilder<'a> {
    /// Set the key-value pairs on a log record.
    pub fn key_values(&mut self, kvs: ErasedSource<'a>) -> &mut RecordBuilder<'a>;
}

impl<'a> Record<'a> {
    /// Get the key-value pairs.
    pub fn key_values(&self) -> ErasedSource;

    /// Get a builder that's preconfigured from this record.
    pub fn to_builder(&self) -> RecordBuilder;
}

pub mod kv {
    pub mod source {
        pub use kv::Error;

        /// A source for key-value pairs.
        pub trait Source {
            /// Serialize the key value pairs.
            fn visit<'kvs>(&'kvs self, visitor: &mut dyn Visitor<'kvs>) -> Result<(), Error>;

            /// Erase this `Source` so it can be used without
            /// requiring generic type parameters.
            fn erase(&self) -> ErasedSource
            where
                Self: Sized {}

            /// Find the value for a given key.
            /// 
            /// If the key is present multiple times, this method will
            /// return the *last* value for the given key.
            fn get<'kvs, Q>(&'kvs self, key: Q) -> Option<Value<'kvs>>
            where
                Q: Borrow<str> {}

            /// An adapter to borrow self.
            fn by_ref(&self) -> &Self {}

            /// Chain two `Source`s together.
            fn chain<KVS>(self, other: KVS) -> Chain<Self, KVS>
            where
                Self: Sized {}

            /// Apply a function to each key-value pair.
            fn try_for_each<F, E>(self, f: F) -> Result<(), Error>
            where
                Self: Sized,
                F: FnMut(Key, Value) -> Result<(), E>,
                E: Into<Error> {}

            /// Serialize the key-value pairs as a map.
            fn serialize_as_map(self) -> SerializeAsMap<Self>
            where
                Self: Sized {}

            /// Serialize the key-value pairs as a sequence of tuples.
            fn serialize_as_seq(self) -> SerializeAsSeq<Self>
            where
                Self: Sized {}
        }

        /// A visitor for a set of key-value pairs.
        /// 
        /// The visitor is driven by an implementation of `Source`.
        /// The visitor expects keys and values that satisfy a given lifetime.
        pub trait Visitor<'kvs> {
            /// Visit a single key-value pair.
            fn visit_pair(&mut self, k: Key<'kvs>, v: Value<'kvs>) -> Result<(), Error>;
        }

        /// An erased `Source`.
        pub struct ErasedSource<'a> {}

        impl<'a> ErasedSource<'a> {
            /// Capture a `Source` and erase its concrete type.
            pub fn new(kvs: &'a impl Source) -> Self {}
        }

        impl<'a> Clone for ErasedSource<'a> {}
        impl<'a> Default for ErasedSource<'a> {}
        impl<'a> Source for ErasedSource<'a> {}

        /// A `Source` adapter that visits key-value pairs
        /// in sequence.
        /// 
        /// This is the result of calling `chain` on a `Source`.
        pub struct Chain<A, B> {}

        impl<A, B> Source for Chain<A, B>
        where
            A: Source,
            B: Source {}

        /// A `Source` adapter that can be serialized as
        /// a map using `serde`.
        /// 
        /// This is the result of calling `serialize_as_map` on
        /// a `Source`.
        pub struct SerializeAsMap<KVS> {}

        impl<KVS> Serialize for SerializeAsMap<KVS>
        where
            KVS: Source {}

        /// A `Source` adapter that can be serialized as
        /// a sequence of tuples using `serde`.
        /// 
        /// This is the result of calling `serialize_as_seq` on
        /// a `Source`.
        pub struct SerializeAsSeq<KVS> {}

        impl<KVS> Serialize for SerializeAsSeq<KVS>
        where
            KVS: Source {}

        impl<K, V> Source for (K, V)
        where
            K: Borrow<str>,
            V: kv::value::Visit {}

        impl<KVS> Source for [KVS]
        where
            KVS: Source {}

        #[cfg(feature = "std")]
        impl<KVS: ?Sized> Source for Box<KVS> where KVS: Source {}
        #[cfg(feature = "std")]
        impl<KVS: ?Sized> Source for Arc<KVS> where KVS: Source {}
        #[cfg(feature = "std")]
        impl<KVS: ?Sized> Source for Rc<KVS> where KVS: Source {}

        #[cfg(feature = "std")]
        impl<KVS> Source for Vec<KVS>
        where
            KVS: Source {}

        #[cfg(feature = "std")]
        impl<K, V> Source for BTreeMap<K, V>
        where
            K: Borrow<str> + Ord,
            V: kv::value::Visit {}

        #[cfg(feature = "std")]
        impl<K, V> Source for HashMap<K, V>
        where
            K: Borrow<str> + Eq + Hash,
            V: kv::value::Visit {}

        /// The key in a key-value pair.
        pub struct Key<'kvs> {}

        /// The value in a key-value pair.
        pub use kv::value::Value;
    }

    pub mod value {
        pub use kv::Error;

        /// An arbitrary structured value.
        pub struct Value<'v> {
            /// Create a new borrowed value.
            pub fn new(v: &'v impl Visit) -> Self {}

            /// Create a new borrowed value from an arbitrary type.
            pub fn any<T>(&'v T, fn(&T, &mut dyn Visitor) -> Result<(), Error>) -> Self {}

            /// Visit the value with the given serializer.
            pub fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error> {}
        }

        impl<'v> Debug for Value<'v> {}
        impl<'v> Display for Value<'v> {}

        /// A serializer for primitive values.
        pub trait Visitor {
            /// Visit an arbitrary value.
            fn visit_any(&mut self, v: Value) -> Result<(), Error>;

            /// Visit a signed integer.
            fn visit_i64(&mut self, v: i64) -> Result<(), Error> {}

            /// Visit an unsigned integer.
            fn visit_u64(&mut self, v: u64) -> Result<(), Error> {}

            /// Visit a floating point number.
            fn visit_f64(&mut self, v: f64) -> Result<(), Error> {}

            /// Visit a boolean.
            fn visit_bool(&mut self, v: bool) -> Result<(), Error> {}

            /// Visit a single character.
            fn visit_char(&mut self, v: char) -> Result<(), Error> {}

            /// Visit a UTF8 string.
            fn visit_str(&mut self, v: &str) -> Result<(), Error> {}

            /// Visit a raw byte buffer.
            fn visit_bytes(&mut self, v: &[u8]) -> Result<(), Error> {}

            /// Visit an empty value.
            fn visit_none(&mut self) -> Result<(), Error> {}

            /// Visit standard arguments.
            fn visit_fmt(&mut self, v: &fmt::Arguments) -> Result<(), Error> {}
        }

        impl<'a, T: ?Sized> Visitor for &'a mut T
        where
            T: Visitor {}

        /// Covnert a type into a value.
        /// 
        /// ** This trait can't be implemented manually **
        pub trait Visit: private::Sealed {
            /// Visit this value.
            fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error>;

            /// Convert a reference to this value into an erased `Value`.
            fn to_value(&self) -> Value
            where
                Self: Sized,
            {
                Value::new(self)
            }
        }

        #[cfg(not(feature = "kv_serde"))]
        impl Visit for u8 {}
        #[cfg(not(feature = "kv_serde"))]
        impl Visit for u16 {}
        #[cfg(not(feature = "kv_serde"))]
        impl Visit for u32 {}
        #[cfg(not(feature = "kv_serde"))]
        impl Visit for u64 {}
        #[cfg(not(feature = "kv_serde"))]
        impl Visit for u128 {}

        #[cfg(not(feature = "kv_serde"))]
        impl Visit for i8 {}
        #[cfg(not(feature = "kv_serde"))]
        impl Visit for i16 {}
        #[cfg(not(feature = "kv_serde"))]
        impl Visit for i32 {}
        #[cfg(not(feature = "kv_serde"))]
        impl Visit for i64 {}
        #[cfg(not(feature = "kv_serde"))]
        impl Visit for i128 {}

        #[cfg(not(feature = "kv_serde"))]
        impl Visit for f32 {}
        #[cfg(not(feature = "kv_serde"))]
        impl Visit for f64 {}

        #[cfg(not(feature = "kv_serde"))]
        impl Visit for char {}
        #[cfg(not(feature = "kv_serde"))]
        impl Visit for bool {}

        #[cfg(not(feature = "kv_serde"))]
        impl<T> Visit for Option<T>
        where
            T: Visit {}

        #[cfg(all(not(feature = "kv_serde"), feature = "std"))]
        impl<T: ?Sized> Visit for Box<T>
        where
            T: Visit {}

        #[cfg(not(feature = "kv_serde"))]
        impl<'a> Visit for &'a str {}
        #[cfg(all(not(feature = "kv_serde"), feature = "std"))]
        impl Visit for String {}

        #[cfg(not(feature = "kv_serde"))]
        impl<'a> Visit for &'a [u8] {}
        #[cfg(all(not(feature = "kv_serde"), feature = "std"))]
        impl Visit for Vec<u8> {}

        #[cfg(not(feature = "kv_serde"))]
        impl<'a, T> Visit for &'a T
        where
            T: Visit {}

        #[cfg(feature = "kv_serde")]
        impl<T> Visit for T
        where
            T: Debug + Serialize {}
    }

    pub use source::Source;

    /// An error encountered while visiting key-value pairs.
    pub struct Error {}

    impl Error {
        /// Create an error from a static message.
        pub fn msg(msg: &'static str) -> Self {}

        /// Get a reference to a standard error.
        #[cfg(feature = "std")]
        pub fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {}

        /// Convert into a standard error.
        #[cfg(feature = "std")]
        pub fn into_error(self) -> Box<dyn std::error::Error + Send + Sync> {}

        /// Convert into a `serde` error.
        #[cfg(feature = "kv_serde")]
        pub fn into_serde<E>(self) -> E
        where
            E: serde::ser::Error {}
    }

    #[cfg(not(feature = "std"))]
    impl From<std::fmt::Error> for Error {}

    #[cfg(feature = "std")]
    impl<E> From<E> for Error
    where
        E: std::error::Error {}

    #[cfg(feature = "std")]
    impl From<Error> for Box<dyn std::error::Error + Send + Sync> {}

    #[cfg(feature = "std")]
    impl AsRef<dyn std::error::Error + Send + Sync + 'static> for Error {}
}
```
