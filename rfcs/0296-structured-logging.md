- Feature Name: `structured_logging`
- Start Date: 2019-03-11
- RFC PR: [log/rfcs#0296](https://github.com/rust-lang-nursery/log/pull/296)

# Summary
[summary]: #summary

Add support for structured logging to the `log` crate in both `std` and `no_std` environments, allowing log records to carry typed data beyond a textual message. This document serves as an introduction to what structured logging is all about, and as an RFC for an implementation in the `log` crate.

`log` will provide an API for capturing structured data that offloads complex serialization to de-facto standards in the ecosystem, but avoids integrating them too tightly, or forcing any specific framework on consumers.

The API is heavily inspired by `slog` and `tokio-trace`.

> NOTE: Code in this RFC uses recent language features like `impl Trait`, but can be implemented without them.

# Contents
- [Motivation](#motivation)
  - [What is structured logging?](#what-is-structured-logging)
  - [Why do we need structured logging in `log`?](#why-do-we-need-structured-logging-in-log)
- [Guide-level explanation](#guide-level-explanation)
  - [Logging structured key-value pairs](#logging-structured-key-value-pairs)
  - [Supporting key-value pairs in `Log` implementations](#supporting-key-value-pairs-in-log-implementations)
  - [Integrating log frameworks with `log`](#integrating-log-frameworks-with-log)
  - [How producers and consumers of structured values interact](#how-producers-and-consumers-of-structured-values-interact)
- [Reference-level explanation](#reference-level-explanation)
  - [Design considerations](#design-considerations)
  - [Cargo features](#cargo-features)
  - [A complete key-values API](#a-complete-key-values-api)
    - [`Error`](#error)
    - [`Value`](#value)
    - [`ToValue`](#tovalue)
    - [`Key`](#key)
    - [`ToKey`](#tokey)
    - [`Source`](#source)
    - [`Visitor`](#visitor)
    - [`Record` and `RecordBuilder`](#record-and-recordbuilder)
  - [A minimal key-values API](#a-minimal-key-values-api)
  - [The `log!` macros](#the-log-macros)
- [Drawbacks, rationale, and alternatives](#drawbacks-rationale-and-alternatives)
- [Prior art](#prior-art)
- [Unresolved questions](#unresolved-questions)

# Motivation
[motivation]: #motivation

## What is structured logging?

Information in log records can be traditionally captured as a blob of text, including a level, a message, and maybe a few other pieces of metadata. There's a lot of potentially valuable information we throw away when we format log records this way. Arbitrary textual representations often result in output that is neither easy for humans to read, nor for machines to parse.

Structured logs can retain their original structure in a machine-readable format. They can be changed programmatically within a logging pipeline before reaching their destination. Once there, they can be analyzed using common database tools.

As an example of structured logging, a textual record like this:

```
[INF 2018-09-27T09:32:03Z basic] [service: database, correlation: 123] Operation completed successfully in 18ms
```

could be represented as a structured record like this:

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

When log records are kept in a structured format like this, potentially interesting queries like _what are all records where the correlation is 123?_, or _how many errors were there in the last hour?_ can be computed efficiently.

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

On the surface there doesn't seem to be a lot of difference between `log` and `slog`, so why not just deprecate one in favor of the other? Conceptually, `log` and `slog` are different libraries that fill different roles, even if there's some overlap.

`slog` is a logging _framework_. It offers all the fundamental tools needed out-of-the-box to capture log records, define and implement the pieces of a logging pipeline, and pass them through that pipeline to an eventual destination. It has conventions and trade-offs baked into the design of its API. Loggers are treated explicitly as values in data structures and as arguments, and callers can control whether to pass owned or borrowed data.

`log` is a logging _facade_. It's only concerned with a standard, minimal API for capturing log records, and surfacing those records to some consumer. The tools provided by `log` are only those that are fundamental to the operation of the `log!` macro. From `log`'s point of view, a logging framework like `slog` is a black-box implementation of the `Log` trait. In this role, the `Log` trait can act as a common entry-point for capturing log records. That means the `Record` type can act as a common container for describing a log record. `log` has its own set of trade-offs baked into the design of its API. The `log!` macro assumes a single, global entry-point, and all data in a log record is borrowed from the call-site.

A healthy logging ecosystem needs both `log` and frameworks like `slog`. As a standard API, `log` can support a diverse but cohesive ecosystem of logging tools in Rust by acting as the glue between libraries, frameworks, and applications. A lot of libraries already depend on it. In order to really fulfill this role though, `log` needs to support structured logging so that libraries and their consumers can take advantage of it in a framework-agnostic way.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This section introduces `log`'s structured logging API through a tour of how structured records can be captured and consumed.

## Logging structured key-value pairs

Structured logging is supported in `log` by allowing typed key-value pairs to be associated with a log record. This support isn't surfaced in the `log!` macros initially, so you'll need to use a framework like `slog` or `tokio-trace` with `log` integration, or an alternative implementation of the `log!` macros to capture structured key-value pairs.

### What can be captured as a structured value?

A value can be captured as a structured value in a log record if it implements the `ToValue` trait:

```rust
pub trait ToValue {
    fn to_value<'v>(&'v self) -> Value<'v>;
}
```

where `Value` is a special container for structured data:

```rust
pub struct Value<'v>(_);

// A value can always be debugged
impl<'v> Debug for Value<'v> {
    ..
}
```

We'll look at `Value` in more detail later. For now, we can think of it as a container that normalizes capturing and emitting the structure of values.

Initially, that means a fixed set of primitive types from the standard library:

- Standard formats: `Arguments`
- Primitives: `bool`, `char`
- Unsigned integers: `u8`, `u16`, `u32`, `u64`, `u128`
- Signed integers: `i8`, `i16`, `i32`, `i64`, `i128`
- Strings: `&str`, `String`
- Paths: `&Path`, `PathBuf`
- Special types: `Option<T>`, `&T`, and `()`.

Each of these types implements `ToValue` in a way that opaquely retains some notion of their underlying structure. Using `u8` as an example:

```rust
impl ToValue for u8 {
    fn to_value(&self) -> Value {
        Value::from_any(self, |from, v| from.u64(*v as u64))
    }
}
```

The `Value::from_any` method accepts any type, `&T`, and an ad-hoc function that tells the `Value` what its structure is:

```rust
impl<'v> Value<'v> {
    pub fn from_any<T>(v: &'v T, from: fn(FromAny, &T) -> Result<(), Error>) -> Self {
        ..
    }
}

pub struct FromAny(_);

impl FromAny {
    pub fn debug(v: impl Debug) -> Result<(), Error> {
        ..
    }

    // Not publicly exposed. Used by internal implementations
    fn u64(v: u64) -> Result<(), Error> {
        ..
    }
}
```

Only being able to log primitive types from the standard library is a bit limiting though. What if `correlation_id` is a `uuid::Uuid`, and `user` is a struct, `User`, with its own fields?

#### Implementing `ToValue` for a simple value

A newtype structure like `uuid::Uuid` could implement the `ToValue` trait directly in terms of some underlying value that already implements `ToValue`:

```rust
impl ToValue for Uuid {
    fn to_value(&self) -> Value {
        self.as_u128().to_value()
    }
}
```

Alternatively, `uuid::Uuid` could provide a nicer implementation using its `Debug` implementation:

```rust
impl ToValue for Uuid {
    fn to_value(&self) -> Value {
        Value::from_debug(self)
    }
}
```

Finally, it could use the `Debug` implementation of its hyphenated format:

```rust
impl ToValue for Uuid {
    fn to_value(&self) -> Value {
        Value::from_any(self, |from, uuid| from.debug(uuid.to_hyphenated()))
    }
}
```

There's some subtlety in this last implementation. The actual value whose structure is captured is not the `&'v Uuid`, it's the owned `ToHyphenated<'v>`. This is why `Value::from_any` uses a separate function for capturing the structure of its values that doesn't depend on the lifetime of the given `&'v T`. It lets us capture a borrowed `Uuid` with the right lifetime `'v`, but materialize an owned `ToHyphenated` with the structure we want.

#### Implementing `ToValue` for a complex value

A structure like `User` is a bit different from a newtype like `uuid::Uuid`. It could be represented using `Debug`, but then the contents of its fields would be lost in an opaque and unstructured string. It would be better represented as a map of key-value pairs. However, complex values like maps and sequences aren't directly supported in `log`. They're offloaded to serialization frameworks like `serde` and `sval` that are capable of handling them effectively.

`serde` and `sval` have direct two-way integration with `log`'s `Value` type through optional Cargo features. Let's use `sval` as an example. It's a serialization framework that's built specifically for structured logging. Adding the `kv_sval` feature to `log` will enable it:

```toml
[dependencies.log]
features = ["kv_sval"]
```

Complex structures that derive `sval`'s `Value` trait can then implement `log`'s `ToValue` trait in terms of `sval`:

```rust
#[derive(Debug, Value)]
struct User {
    name: String,
}

impl ToValue for User {
    fn to_value(&self) -> Value {
        Value::from_sval(self)
    }
}
```

In this way, the underlying structure of a `User`, described as a map by `sval`, is retained when converting it into a `Value`. Using `serde` instead of `sval` is a similar story:

```toml
[dependencies.log]
features = ["kv_serde"]
```

```rust
#[derive(Debug, Serialize)]
struct User {
    name: String,
}

impl ToValue for User {
    fn to_value(&self) -> Value {
        Value::from_serde(self)
    }
}
```

## Supporting key-value pairs in `Log` implementations

Capturing structured logs is only half the story. Implementors of the `Log` trait also need to be able to work with any key-value pairs associated with a log record. Key-value pairs are accessible on a log record through the `Record::key_values` method:

```rust
impl Record {
    pub fn key_values(&self) -> impl Source;
}
```

where `Source` is a trait for iterating over the individual key-value pairs:

```rust
pub trait Source {
    // Get the value for a given key
    fn get<'kvs, Q>(&'kvs self, key: Q) -> Option<Value<'kvs>>
    where
        Q: ToKey,
    {
        ..
    }

    // Run a function for each key-value pair
    fn for_each<F>(self, f: F) -> Result<(), Error>
    where
        Self: Sized,
        F: FnMut(Key, Value),
    {
        ..
    }

    // Run a function for each key-value pair
    fn try_for_each<F, E>(self, f: F) -> Result<(), Error>
    where
        Self: Sized,
        F: FnMut(Key, Value) -> Result<(), E>,
        E: Into<Error>,
    {
        ..
    }

    // Serialize the source as a map of key-value pairs
    fn as_map(self) -> AsMap<Self>
    where
        Self: Sized,
    {
        ..
    }

    // Serialize the source as a sequence of key-value tuples
    fn as_seq(self) -> AsSeq<Self>
    where
        Self: Sized,
    {
        ..
    }

    // Other methods we'll look at later
}
```

### Writing key-value pairs as text

To demonstrate how to work with a `Source`, let's take the textual log format from before:

```
[INF 2018-09-27T09:32:03Z] Operation completed successfully in 18ms
module: "basic"
service: "database"
correlation: 123
took: 18
```

Each key-value pair, shown as a `$key: $value` line, can be formatted from the `Source` on a record using the `std::fmt` machinery:

```rust
use log::key_values::Source;

fn log_record(w: impl Write, r: &Record) -> io::Result<()> {
    // Write the first line of the log record
    ..

    // Write each key-value pair on a new line
    record
        .key_values()
        .for_each(|k, v| writeln!("{}: {}", k, v))?;

    Ok(())
}
```

In the above example, the `Source::for_each` method iterates over each key-value pair in the `Source` and writes them to the output stream.

### Writing key-value pairs as JSON

Let's look at a structured example. Take the following JSON map:

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

A `Source` can be serialized as a map using a serialization framework like `serde` or `sval`. Using `serde` for this example requires the `kv_serde` feature:

```toml
[dependencies.log]
features = ["kv_serde"]
```

Defining a serializable structure based on a log record for the previous JSON map could then be done using `serde_derive`, and then written using `serde_json`:

```rust
use log::key_values::Source;

fn log_record(w: impl Write, r: &Record) -> io::Result<()> {
    let r = SerializeRecord {
        lvl: r.level(),
        ts: epoch_millis(),
        msg: r.args().to_string(),
        kvs: r.key_values().as_map(),
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
    kvs: KVS,
}
```

This time, instead of using the `Source::for_each` method, we use the `Source::as_map` method to get an adapter that implements `serde::Serialize` by serializing each key-value pair as an entry in a `serde` map.

## Integrating log frameworks with `log`

The `Source` trait we saw previously describes some container for structured key-value pairs that can be iterated or serialized. Other log frameworks that want to integrate with the `log` crate should build `Record`s that contain some implementation of `Source` based on their own structured logging.

The previous section demonstrated some of the methods available on `Source` like `Source::try_for_each` and `Source::as_map`. Both of those methods are provided on top of a required lower-level `Source::visit` method, which looks something like this:

```rust
trait Source {
    fn visit<'kvs>(&'kvs self, visitor: &mut impl Visitor<'kvs>) -> Result<(), Error>;

    // Provided methods
}
```

where `Visitor` is another trait that accepts individual key-value pairs:

```rust
trait Visitor<'kvs> {
    fn visit_pair(&mut self, k: Key<'kvs>, v: Value<'kvs>) -> Result<(), Error>;
}
```

where `Key` is a container for a string and `Value` is the container for structured data we saw previously. The lifetime `'kvs` is threaded from the original borrow of the `Source` through to the `Key`s and `Value`s that a `Visitor` sees. That allows visitors to work with key-value pairs that can live for longer than a single call to `Visitor::visit_pair`.

Let's implement a `Source`. As an example, let's say our log framework captures its key-value pairs in a `BTreeMap`:

```rust
struct KeyValues {
    data: BTreeMap<String, serde_json::Value>,
}
```

The `Source` trait could be implemented for `KeyValues` like this:

```rust
use log::key_values::source::{self, Source};

impl Source for KeyValues {
    fn visit<'kvs>(&'kvs self, visitor: &mut impl source::Visitor<'kvs>) -> Result<(), source::Error> {
        for (k, v) in self.data {
            visitor.visit_pair(source::Key::from_str(k), source::Value::from_serde(v))
        }
    }
}
```

The `Key::from_str` method accepts any `T: Borrow<str>`. The `Value::from_serde` accepts any `T: serde::Serialize + Debug`.

A `Source` doesn't have to just contain key-value pairs directly like `BTreeMap<String, serde_json::Value>` though. It could act like an adapter that changes its pairs before emitting them, like we have for iterators in the standard library. As another example, the following `Source` doesn't store any key-value pairs of its own, instead it will sort and de-duplicate pairs read from another source by first reading them into a map before forwarding them on:

```rust
use log::key_values::source::{self, Source, Visitor};

pub struct SortRetainLast<KVS>(KVS);

impl<KVS> Source for SortRetainLast<KVS>
where
    KVS: Source,
{
    fn visit<'kvs>(&'kvs self, visitor: &mut impl source::Visitor<'kvs>) -> Result<(), source::Error> {
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

## How producers and consumers of structured values interact

The previous sections demonstrated some of the APIs for capturing and consuming structured data on log records. The `ToValue` trait and `Value::from_any` methods capture values into a common `Value` container. The `Source` trait allows these `Value`s to be consumed using `std::fmt`, `sval` or `serde`.

Values captured from any one supported framework can be represented by any other. That means a value can be captured in terms of `sval` and consumed in terms of `serde`, with its underlying structure retained.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This section details the nuts-and-bolts of the structured logging API.

## Design considerations

### Don't break anything

Allow structured logging to be added in the current `0.4.x` series of `log`. This gives us the option of including structured logging in an `0.4.x` release, or bumping the minor crate version without introducing any actual breaking changes.

### Don't create a public dependency

Don't create a new serialization API that requires `log` to become a public dependency of any library that wants their data types to be logged. Logging is a truly cross-cutting concern, so if `log` was a public dependency it would become even more difficult to develop its API without compounding churn. Any traits that are expected to be publicly implemented should be narrowly scoped to make backwards compatibility easier.

### Support arbitrary producers and arbitrary consumers

Provide an API that's suitable for two independent logging frameworks to integrate through if they want. Producers of structured data and consumers of structured data should be able to use different serialization frameworks opaquely and still get good results. As an example, a producer of a `log::Record` should be able to log a map that implements `sval::Value`, and the implementor of the receiving `Log` trait should be able to format that map using `serde::Serialize`.

### Remain object safe

`log` is already designed to be object-safe so this new structured logging API needs to be object-safe too.

### Enable the next round of `log` development

Once structured logging is available, there will be a lot of new ways to hold `log` and new concepts to try out, such as a procedural-macro-based `log!` implementation and explicit treatment of `std::error::Error`. The APIs introduced by this RFC should enable others to build these features in external crates, and look at integrating that work back into `log` in the future.

## Cargo features

Structured logging will be supported in either `std` or `no_std` contexts by default.

```toml
[features]
std = []
kv_sval = ["sval"]
kv_serde = ["std", "serde", "erased-serde", "sval"]
```

### `kv_sval` and `kv_serde`

Using default features, implementors of the `Log` trait will be able to format structured data (in the form of `Value`s) using the `std::fmt` machinery.

`sval` is a new serialization framework that's specifically designed with structured logging in mind. It's `no_std` and object-safe, but isn't stable and requires `rustc` `1.31.0`. Using the `kv_sval` feature, any `Value` will also implement `sval::Value` so its underlying structure will be visible to consumers of structured data using `sval::Stream`s.

`serde` is the de-facto general-purpose serialization framework for Rust. It's widely supported and stable, but requires some significant runtime machinery in order to be object-safe. Using the `kv_serde` feature, any `Value` will also implement `serde::Serialize` so its underlying structure will be visible to consumers of structured data using `serde::Serializer`s.

## A complete key-values API

The following section details the public API for structured values in `log`, along with possible future extensions. Actual implementation details are excluded for brevity unless they're particularly noteworthy. See the original comment on the [RFC issue](https://github.com/rust-lang-nursery/log/pull/296#issue-222687727) for a reference implementation.

### `Error`

Just about the only things you can do with a structured value are format it or serialize it. Serialization and writing might fail, so to allow errors to get carried back to callers there needs to be a general error type that they can early return with:

```rust
pub struct Error(Inner);

impl Error {
    pub fn msg(msg: &'static str) -> Self {
        ..
    }
}

impl Debug for Error {
    ..
}

impl Display for Error {
    ..
}

impl From<fmt::Error> for Error {
    ..
}

impl From<Error> for fmt::Error {
    ..
}

#[cfg(feature = "kv_sval")]
mod sval_support {
    impl From<sval::Error> for Error {
        ..
    }

    impl Error {
        pub fn into_sval(self) -> sval::Error {
            ..
        }
    }
}

#[cfg(feature = "kv_serde")]
mod serde_support {
    impl Error {
        pub fn into_serde<E>(self) -> E
        where
            E: serde::ser::Error,
        {
            ..
        }
    }
}

#[cfg(feature = "std")]
mod std_support {
    impl Error {
        pub fn custom(err: impl fmt::Display) -> Self {
            ..
        }
    }

    impl From<io::Error> for Error {
        ..
    }

    impl From<Error> for io::Error {
        ..
    }

    impl error::Error for Error {
        ..
    }
}
```

There's no really universal way to handle errors in a logging pipeline. Knowing that some error occurred, and knowing where, should be enough for implementations of `Log` to decide how to handle it. The `Error` type doesn't try to be a general-purpose error management tool, it tries to make it easy to early-return with other errors.

To make it possible to carry any arbitrary `S::Error` type, where we don't know how long the value can live for and whether it's `Send` or `Sync`, without extra work, the `Error` type does not attempt to store the error value itself. It just converts it into a `String`.

### `Value`

A `Value` is an erased container for some type whose structure can be visited, with a potentially short-lived lifetime:

```rust
pub struct Value<'v>(_);

impl<'v> Value<'v> {
    pub fn from_any<T>(v: &'v T, from: FromAnyFn<T>) -> Self {
        ..
    }

    pub fn from_debug(v: &'v impl Debug) -> Self {
        Self::from_any(v, |from, v| from.debug(v))
    }

    #[cfg(feature = "kv_sval")]
    pub fn from_sval(v: &'v (impl sval::Value + Debug)) -> Self {
        Self::from_any(v, |from, v| from.sval(v))
    }

    #[cfg(feature = "kv_serde")]
    pub fn from_serde(v: &'v (impl serde::Serialize + Debug)) -> Self {
        Self::from_any(v, |from, v| from.serde(v))
    }
}

impl<'v> Debug for Value<'v> {
    ..
}

impl<'v> Display for Value<'v> {
    ..
}

#[cfg(feature = "kv_sval")]
impl<'v> sval::Value for Value<'v> {
    ..
}

#[cfg(feature = "kv_serde")]
impl<'v> serde::Serialize for Value<'v> {
    ..
}

type FromAnyFn<T> = fn(FromAny, &T) -> Result<(), Error>;
```

The `FromAny` type is like a visitor that accepts values with a particular structure, but doesn't require those values satisfy any lifetime constraints:

```rust
pub struct FromAny<'a>(_);

impl<'a> FromAny<'a> {
    pub fn debug(self, v: impl Debug) -> Result<(), Error> {
        ..
    }

    #[cfg(feature = "kv_sval")]
    pub fn sval(self, v: impl sval::Value + Debug) -> Result<(), Error> {
        ..
    }

    #[cfg(feature = "kv_serde")]
    pub fn serde(self, v: impl serde::Serialize + Debug) -> Result<(), Error> {
        ..
    }

    fn value(self, v: Value) -> Result<(), Error> {
        ..
    }

    fn u64(self, v: u64) -> Result<(), Error> {
        ..
    }

    fn u128(self, v: u128) -> Result<(), Error> {
        ..
    }

    fn i64(self, v: i64) -> Result<(), Error> {
        ..
    }

    fn i128(self, v: i128) -> Result<(), Error> {
        ..
    }

    fn f64(self, v: f64) -> Result<(), Error> {
        ..
    }

    fn bool(self, v: bool) -> Result<(), Error> {
        ..
    }

    fn char(self, v: char) -> Result<(), Error> {
        ..
    }

    fn none(self) -> Result<(), Error> {
        ..
    }

    fn str(self, v: &str) -> Result<(), Error> {
        ..
    }
}
```

#### Erasing values in `Value::from_any`

Internally, the `Value` type uses similar machinery to `std::fmt::Argument` for pairing an erased incoming type with a function for operating on it:

```rust
pub struct Value<'v>(Inner<'v>);

impl<'v> Value<'v> {
    pub fn from_any<T>(v: &'v T, from: FromAnyFn<T>) -> Self {
        Value(Inner::new(v, from))
    }
}

struct Void {
    _priv: (),
    _oibit_remover: PhantomData<*mut dyn Fn()>,
}

#[derive(Clone, Copy)]
struct Inner<'a> {
    data: &'a Void,
    from: FromAnyFn<Void>,
}

type FromAnyFn<T> = fn(FromAny, &T) -> Result<(), Error>;

impl<'a> Inner<'a> {
    fn new<T>(data: &'a T, from: FromAnyFn<T>) -> Self {
        unsafe {
            Inner {
                data: mem::transmute::<&'a T, &'a Void>(data),
                from: mem::transmute::<FromAnyFn<T>, FromAnyFn<Void>>(from),
            }
        }
    }

    // Backend is an internal trait that bridges supported serialization frameworks
    fn visit(&self, backend: &mut dyn Backend) -> Result<(), Error> {
        (self.from)(FromAny(backend), self.data)
    }
}
```

The benefit of the `Value::from_any` approach over a dedicated trait is that `Value::from_any` doesn't make any constraints on the incoming `&'v T` besides needing to satisfy the `'v` lifetime. That makes it possible to materialize newtypes from the borrowed `&'v T` to satisfy serialization constraints for cases where the caller doesn't own `T` and can't implement traits on it.

#### Ownership

The `Value` type borrows from its inner value.

#### Thread-safety

The `Value` type doesn't try to guarantee that values are `Send` or `Sync`, and doesn't offer any way of retaining that information when erasing.

### `ToValue`

The `ToValue` trait represents a type that can be converted into a `Value`:

```rust
pub trait ToValue {
    fn to_value(&self) -> Value;
}
```

It's the generic trait bound that macros capturing structured values can require.

#### Object safety

The `ToValue` trait is object-safe.

#### Implementors

`ToValue` is implemented for fundamental primitive types:

```rust
impl<'v> ToValue for Value<'v> {
    fn to_value(&self) -> Value {
        Value(self.0)
    }
}

impl<'a, T> ToValue for &'a T
where
    T: ToValue,
{
    fn to_value(&self) -> Value {
        (**self).to_value()
    }
}

impl ToValue for () {
    fn to_value(&self) -> Value {
        Value::from_any(self, |from, _| from.none())
    }
}

impl ToValue for u8 {
    fn to_value(&self) -> Value {
        Value::from_any(self, |from, v| from.u64(*v as u64))
    }
}

impl ToValue for u16 {
    fn to_value(&self) -> Value {
        Value::from_any(self, |from, v| from.u64(*v as u64))
    }
}

impl ToValue for u32 {
    fn to_value(&self) -> Value {
        Value::from_any(self, |from, v| from.u64(*v as u64))
    }
}

impl ToValue for u64 {
    fn to_value(&self) -> Value {
        Value::from_any(self, |from, v| from.u64(*v))
    }
}

impl ToValue for u128 {
    fn to_value(&self) -> Value {
        Value::from_any(self, |from, v| from.u128(*v))
    }
}

impl ToValue for i8 {
    fn to_value(&self) -> Value {
        Value::from_any(self, |from, v| from.i64(*v as i64))
    }
}

impl ToValue for i16 {
    fn to_value(&self) -> Value {
        Value::from_any(self, |from, v| from.i64(*v as i64))
    }
}

impl ToValue for i32 {
    fn to_value(&self) -> Value {
        Value::from_any(self, |from, v| from.i64(*v as i64))
    }
}

impl ToValue for i64 {
    fn to_value(&self) -> Value {
        Value::from_any(self, |from, v| from.i64(*v))
    }
}

impl ToValue for i128 {
    fn to_value(&self) -> Value {
        Value::from_any(self, |from, v| from.i128(*v))
    }
}

impl ToValue for f32 {
    fn to_value(&self) -> Value {
        Value::from_any(self, |from, v| from.f64(*v as f64))
    }
}

impl ToValue for f64 {
    fn to_value(&self) -> Value {
        Value::from_any(self, |from, v| from.f64(*v))
    }
}

impl ToValue for bool {
    fn to_value(&self) -> Value {
        Value::from_any(self, |from, v| from.bool(*v))
    }
}

impl ToValue for char {
    fn to_value(&self) -> Value {
        Value::from_any(self, |from, v| from.char(*v))
    }
}

impl<T> ToValue for Option<T>
where
    T: ToValue,
{
    fn to_value(&self) -> Value {
        Value::from_any(self, |from, v| match v {
            Some(ref v) => from.value(v.to_value()),
            None => from.none(),
        })
    }
}

impl<'a> ToValue for &'a str {
    fn to_value(&self) -> Value {
        Value::from_any(self, |from, v| from.str(*v))
    }
}

impl<'a> ToValue for &'a Arguments {
    fn to_value(&self) -> Value {
        Value::from_any(self, |from, v| from.debug(*v))
    }
}
```

When `std` is available, `ToValue` is implemented for additional types:

```rust
impl<T: ?Sized> ToValue for Box<T> where T: ToValue {
    fn to_value(&self) -> Value {
        (**self).to_value()
    }
}

impl<T: ?Sized> ToValue for Arc<KVS> where KVS: ToValue  {
    fn to_value(&self) -> Value {
        (**self).to_value()
    }
}

impl<T: ?Sized> ToValue for Rc<KVS> where KVS: ToValue  {
    fn to_value(&self) -> Value {
        (**self).to_value()
    }
}

impl ToValue for String {
    fn to_value(&self) -> Value {
        Value::from_any(self, |from, v| from.str(*v))
    }
}

impl<'a> ToValue for &'a Path {
    fn to_value(&self) -> Value {
        Value::from_any(self, |from, v| from.debug(*v))
    }
}

impl ToValue for PathBuf {
    fn to_value(&self) -> Value {
        Value::from_any(self, |from, v| from.debug(*v))
    }
}
```

Other implementations for `std` types can be added in the same fashion.

### `Key`

A `Key` is a short-lived structure that can be represented as a UTF-8 string:

```rust
pub struct Key<'k>(_);

impl<'k> Key<'k> {
    pub fn from_str(key: &'k (impl Borrow<str> + ?Sized)) -> Self {
        ..
    }

    pub fn as_str(&self) -> &str {
        ..
    }
}

impl<'k> AsRef<str> for Key<'k> {
    ..
}

impl<'k> Borrow<str> for Key<'k> {
    ..
}

impl<'k> From<&'k str> for Key<'k> {
    ..
}

impl<'k> PartialEq for Key<'k> {
    ..
}

impl<'k> Eq for Key<'k> {}

impl<'k> PartialOrd for Key<'k> {
    ..
}

impl<'k> Ord for Key<'k> {
    ..
}

impl<'k> Hash for Key<'k> {
    ..
}

impl<'k> Debug for Key<'k> {
    ..
}

impl<'k> Display for Key<'k> {
    ..
}
```

When `std` is available, a `Key` can also contain an owned `String`:

```rust
impl<'k> Key<'k> {
    pub fn from_owned(key: impl Into<String>) -> Self {
        ..
    }
}

impl ToKey for String {
    fn to_key(&self) -> Key {
        Key::from_str(self, None)
    }
}

impl<'k> From<String> for Key<'k> {
    ..
}
```

When the `kv_sval` or `kv_serde` features are enabled, a `Key` can be serialized using `sval` or `serde`:

```rust
#[cfg(feature = "kv_sval")]
mod sval_support {
    impl<'k> sval::Value for Key<'k> {
        ..
    }
}

#[cfg(feature = "kv_serde")]
mod serde_support {
    impl<'k> Serialize for Key<'k> {
        ..
    }
}
```

Other standard implementations could be added for any `K: Borrow<str>` in the same fashion.

#### Ownership

The `Key` type can either borrow or own its inner value.

#### Thread-safety

The `Key` type is probably `Send` and `Sync`, but that's not guaranteed.

#### Extensibility: Adding an index to keys

The `Key` type could be extended to hold an optional index into a source. This could be used to retrieve a specific key-value pair more efficiently than scanning. There's some subtlety to consider though when the sources of keys are combined in various ways that might invalidate a previous index.

### `ToKey`

The `ToKey` trait represents a type that can be converted into a `Key`:

```rust
pub trait ToKey {
    fn to_key(&self) -> Key;
}
```

#### Object safety

The `ToKey` trait is object-safe.

#### Implementors

The `ToKey` trait is implemented for common string containers in the standard library:

```rust
impl<'a, T: ?Sized> ToKey for &'a T
where
    T: ToKey,
{
    fn to_key(&self) -> Key {
        (**self).to_key()
    }
}

impl ToKey for str {
    fn to_key(&self) -> Key {
        Key::from_str(self, None)
    }
}

impl<'k> ToKey for Key<'k> {
    fn to_key(&self) -> Key {
        Key::from_str(self)
    }
}
```

### `Source`

The `Source` trait is a bit like `std::iter::Iterator` for key-value pairs. It gives us a way to inspect some arbitrary collection of key-value pairs using an object-safe visitor pattern:

```rust
pub trait Source {
    fn visit<'kvs>(&'kvs self, visitor: &mut impl Visitor<'kvs>) -> Result<(), Error>;

    fn erase(&self) -> ErasedSource
    where
        Self: Sized,
    {
        ..
    }

    fn get<'kvs, Q>(&'kvs self, key: Q) -> Option<Value<'kvs>>
    where
        Q: ToKey,
    {
        ..
    }

    fn by_ref(&self) -> &Self {
        ..
    }

    fn chain<KVS>(self, other: KVS) -> Chained<Self, KVS>
    where
        Self: Sized,
    {
        ..
    }

    fn for_each<F>(self, f: F) -> Result<(), Error>
    where
        Self: Sized,
        F: FnMut(Key, Value),
    {
        ..
    }

    fn try_for_each<F, E>(self, f: F) -> Result<(), Error>
    where
        Self: Sized,
        F: FnMut(Key, Value) -> Result<(), E>,
        E: Into<Error>,
    {
        ..
    }

    #[cfg(any(feature = "kv_serde", feature = "kv_sval"))]
    fn as_map(self) -> AsMap<Self>
    where
        Self: Sized,
    {
        ..
    }

    #[cfg(any(feature = "kv_serde", feature = "kv_sval"))]
    fn as_seq(self) -> AsSeq<Self>
    where
        Self: Sized,
    {
        ..
    }
}
```

The `Source` trait is the main extensibility point for structured logging in the `log` crate that's used by the `Record` type. It doesn't make any assumptions about how many key-value pairs it contains or how they're visited. That means the visitor may observe keys in any order, and observe the same key multiple times.

#### Adapters

Some useful adapters exist as provided methods on the `Source` trait. They're similar to adapters on the standard `Iterator` trait:

- `by_ref` to get a reference to a `Source` within a method chain.
- `chain` to concatenate one source with another. This is useful for composing implementations of `Log` together for contextual logging.
- `get` to try find the value associated with a key. This is useful for well-defined key-value pairs that a framework built over `log` might want to provide, like timestamps or message templates.
- `for_each` to execute some closure over all key-value pairs. This is a convenient way to do something with each key-value pair without having to create and implement a `Visitor`. One potential downside of `for_each` is the `Result` return value, which seems surprising when the closure itself can't fail. The `Source::for_each` call might itself fail if the underlying `visit` call fails when iterating over its key-value pairs. This shouldn't be common though, so when paired with `try_for_each`, it might be reasonable to make `for_each` return a `()` and rely on `try_for_each` for surfacing any fallibility.
- `try_for_each` is like `for_each`, but takes a fallible closure.
- `as_map` to get a serializable map. This is a convenient way to serialize key-value pairs without having to create and implement a `Visitor`.
- `as_seq` is like `as_map`, but for serializing as a sequence of tuples.

None of these methods are required for the core API. They're helpful tools for working with key-value pairs with minimal machinery. Even if we don't necessarily include them right away it's worth having an API that can support them later without breakage.

#### Object safety

`Source` is not object-safe because of the provided adapter methods not being object-safe. The only required method, `visit`, is safe though, so an object-safe version of `Source` that forwards this method can be reasonably written:

```rust
#[derive(Clone, Copy)]
pub struct ErasedSource<'a>(&'a dyn ErasedSourceBridge);

impl<'a> ErasedSource<'a> {
    pub fn erased(kvs: &'a impl Source) -> Self {
        ErasedSource(kvs)
    }

    pub fn empty() -> Self {
        ErasedSource(&(&[] as &[(&str, Value)]))
    }
}

impl<'a> Source for ErasedSource<'a> {
    fn visit<'kvs>(&'kvs self, visitor: &mut impl Visitor<'kvs>) -> Result<(), Error> {
        self.0.erased_visit(visitor)
    }

    fn get<'kvs, Q>(&'kvs self, key: Q) -> Option<Value<'kvs>>
    where
        Q: ToKey,
    {
        self.0.erased_get(key.to_key())
    }
}

trait ErasedSourceBridge {
    fn erased_visit<'kvs>(&'kvs self, visitor: &mut impl Visitor<'kvs>) -> Result<(), Error>;
    fn erased_get<'kvs>(&'kvs self, key: Key) -> Option<Value<'kvs>>;
}

impl<KVS> ErasedSourceBridge for KVS
where
    KVS: Source + ?Sized,
{
    fn erased_visit<'kvs>(&'kvs self, visitor: &mut impl Visitor<'kvs>) -> Result<(), Error> {
        self.visit(visitor)
    }

    fn erased_get<'kvs>(&'kvs self, key: Key) -> Option<Value<'kvs>> {
        self.get(key)
    }
}
```

#### Implementors

A `Source` containing a single key-value pair is implemented for a tuple of a key and value:

```rust
impl<K, V> Source for (K, V)
where
    K: ToKey,
    V: ToValue,
{
    fn visit<'kvs>(&'kvs self, visitor: &mut impl Visitor<'kvs>) -> Result<(), Error>
    {
        visitor.visit_pair(self.0.to_key(), self.1.to_value())
    }
}
```

A `Source` with multiple pairs is implemented for arrays of `Source`s:

```rust
impl<KVS> Source for [KVS] where KVS: Source {
    fn visit<'kvs>(&'kvs self, visitor: &mut impl Visitor<'kvs>) -> Result<(), Error> {
        for kv in self {
            kv.visit(visitor)?;
        }

        Ok(())
    }
}
```

When `std` is available, `Source` is implemented for some standard collections too:

```rust
impl<KVS: ?Sized> Source for Box<KVS> where KVS: Source {
    fn visit<'kvs>(&'kvs self, visitor: &mut impl Visitor<'kvs>) -> Result<(), Error> {
        (**self).visit(visitor)
    }
}

impl<KVS: ?Sized> Source for Arc<KVS> where KVS: Source  {
    fn visit<'kvs>(&'kvs self, visitor: &mut impl Visitor<'kvs>) -> Result<(), Error> {
        (**self).visit(visitor)
    }
}

impl<KVS: ?Sized> Source for Rc<KVS> where KVS: Source  {
    fn visit<'kvs>(&'kvs self, visitor: &mut impl Visitor<'kvs>) -> Result<(), Error> {
        (**self).visit(visitor)
    }
}

impl<KVS> Source for Vec<KVS> where KVS: Source {
    fn visit<'kvs>(&'kvs self, visitor: &mut impl Visitor<'kvs>) -> Result<(), Error> {
        self.as_slice().visit(visitor)
    }
}

impl<K, V> Source for BTreeMap<K, V>
where
    K: ToKey + Borrow<str> + Ord,
    V: ToValue,
{
    fn visit<'kvs>(&'kvs self, visitor: &mut impl Visitor<'kvs>) -> Result<(), Error>
    {
        for (k, v) in self {
            visitor.visit_pair(k.borrow().to_key(), v.to_value())?;
        }

        Ok(())
    }

    fn get<'kvs, Q>(&'kvs self, key: Q) -> Option<Value<'kvs>>
    where
        Q: ToKey,
    {
        BTreeMap::get(self, key.to_key().borrow()).map(|v| v.to_value())
    }
}

impl<K, V> Source for HashMap<K, V>
where
    K: ToKey + Borrow<str> + Eq + Hash,
    V: ToValue,
{
    fn visit<'kvs>(&'kvs self, visitor: &mut impl Visitor<'kvs>) -> Result<(), Error>
    {
        for (k, v) in self {
            visitor.visit_pair(k.borrow().to_key(), v.to_value())?;
        }

        Ok(())
    }

    fn get<'kvs, Q>(&'kvs self, key: Q) -> Option<Value<'kvs>>
    where
        Q: ToKey,
    {
        HashMap::get(self, key.to_key().borrow()).map(|v| v.to_value())
    }
}
```

The `BTreeMap` and `HashMap` implementations provide more efficient implementations of `Source::get`.

#### Extensibility: Sending `Source`s between threads

Implementations of `Log` might want to offload the processing of records to some background thread. The record would need to be converted into some owned representation before being sent across threads. This is straightforward for the existing borrowed string metadata on a record, but less so for any structured data. The `Source` trait is the point where having some way to convert from a borrowed to an owned set of key-value pairs would make the most sense because that's where the knowledge of the underlying key-value storage is.

A new provided method could be added to the `Source` trait that allows it to be converted into an owned variant that is `Send + Sync + 'static`:

```rust
pub trait Source {
    ..

    fn to_owned(&self) -> OwnedSource {
        OwnedSource::collect(self)
    }
}

#[derive(Clone)]
pub struct OwnedSource(Arc<dyn ErasedSource + Send + Sync>);

impl OwnedSource {
    pub fn new(impl Into<Arc<impl Source + Send + Sync>>) -> Self {
        OwnedSource(source.into())
    }

    pub fn collect(impl Source) -> Self {
        // Serialize the `Source` to something like
        // `Vec<(String, OwnedValue)>`
        // where `OwnedValue` is like `serde_json::Value`
        ..
    }
}
```

Other implementations of `Source` would be encouraged to override the `to_owned` method if they could provide a more efficient implementation. As an example, if there's a `Source` that is already wrapped up in an `Arc` then it can implement `to_owned` by just cloning itself.

### `Visitor`

The `Visitor` trait used by `Source` can visit a single key-value pair:

```rust
pub trait Visitor<'kvs> {
    fn visit_pair(&mut self, k: Key<'kvs>, v: Value<'kvs>) -> Result<(), Error>;
}

impl<'a, 'kvs, T: ?Sized> Visitor<'kvs> for &'a mut T
where
    T: Visitor<'kvs>
{
    ..
}
```

A `Visitor` may serialize the keys and values as it sees them. It may also do other work, like sorting or de-duplicating them. Operations that involve ordering keys will probably require allocations.

#### Implementors

There aren't any public implementors of `Visitor` in the `log` crate. Other crates that use key-value pairs will implement `Visitor`, or use the adapter methods on `Source` and never need to touch `Visitor`s directly.

#### Object safety

The `Visitor` trait is object-safe.

### `Record` and `RecordBuilder`

Structured key-value pairs can be set on a `RecordBuilder` as an implementation of a `Source`:

```rust
impl<'a> RecordBuilder<'a> {
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
    ..

    kvs: ErasedSource<'a>,
}

impl<'a> Record<'a> {
    pub fn key_values(&self) -> ErasedSource {
        self.kvs.clone()
    }
}
```

## A minimal key-values API

The following API is just the fundamental pieces of what's proposed by this RFC. Everything else could be implemented on top of this subset without introducing breakage. It also offers the freedom to move in a different direction entirely:

```rust
impl<'a> RecordBuilder<'a> {
    pub fn key_values(&mut self, kvs: ErasedSource<'a>) -> &mut RecordBuilder<'a> {
        ..
    }
}

impl<'a> Record<'a> {
    pub fn key_values(&self) -> ErasedSource {
        ..
    }
}

pub struct Error(_);

impl Error {
    pub fn msg(msg: &'static str) -> Self {
        ..
    }
}

impl Debug for Error {
    ..
}

impl Display for Error {
    ..
}

impl From<fmt::Error> for Error {
    ..
}

impl From<Error> for fmt::Error {
    ..
}

#[cfg(feature = "std")]
impl Error {
    pub fn custom(err: impl fmt::Display) -> Self {
        ..
    }
}

#[cfg(feature = "std")]
impl From<io::Error> for Error {
    ..
}

#[cfg(feature = "std")]
impl From<Error> for io::Error {
    ..
}

#[cfg(feature = "std")]
impl error::Error for Error {
    ..
}

pub struct Value<'v>(_);

impl<'v> Value<'v> {
    pub fn from_debug(value: &'v impl Debug) -> Self {
        ..
    }
}

impl<'v> Debug for Value<'v> {
    ..
}

impl<'v> Display for Value<'v> {
    ..
}

pub struct Key<'k>(_);

impl<'k> Key<'k> {
    pub fn from_str(key: &'k (impl Borrow<str> + ?Sized)) -> Self {
        ..
    }

    pub fn as_str(&self) -> &str {
        ..
    }
}

pub trait Source {
    fn visit<'kvs>(&'kvs self, visitor: &mut impl Visitor<'kvs>) -> Result<(), Error>;

    fn erase(&self) -> ErasedSource
    where
        Self: Sized,
    {
        ..
    }
}

pub struct ErasedSource<'a>(_);

pub trait Visitor<'kvs> {
    fn visit_pair(&mut self, k: Key<'kvs>, v: Value<'kvs>) -> Result<(), Error>;
}
```

## The `log!` macros

The existing `log!` macros will not be changed in the initial implementation of structured logging. Instead, `log` will rely on new `log!` macro implementations and existing structured frameworks like `slog` and `tokio-trace` to capture structured key-value pairs.

It's expected that an external library will be created to explore new implementations of the `log!` macros that are structured by design, rather than attempting to graft structured logging support onto the existing macros. The result of this work should eventually find its way back into the `log` crate.

# Drawbacks, rationale, and alternatives
[drawbacks]: #drawbacks

## Supporting structured logging at all

Structured logging is a non-trivial feature to support. It adds complexity and overhead to the `log` crate. The alternative, not supporting structured logging, is not a suitable long-term solution for `log` unless we plan to eventually deprecate it.

## Internalizing `sval` and `serde`

Values captured from any one supported framework can be represented by any other. That means a value can be captured by `sval` can be consumed by `serde`, with its underlying structure retained. This is done through an internal one-to-one integration from each framework to each other framework.

The choice of `sval` as a supported framework is because it's purpose-built for serializing values in structured logging. The choice of `serde` as a supported framework is because it's the de-facto standard in the Rust ecosystem and already used within `log`.

### Drawbacks

The one-to-one bridge between serialization frameworks within `log` makes the effort needed to support them increase exponentially with each addition, and discourages it from supporting more than a few.

It also introduces direct coupling between `log` and these frameworks. For `sval` specifically, this is risky because it's not currently stable and breaking changes are a possibility.

The mechanism suggested in this RFC for erasing values in `Value::from_any` relies on unsafe code. It's the same as what's used in `std::fmt`, but in `std::fmt` the machinery isn't directly exposed to callers outside of unstable features.

### Alternatives

#### Build a serialization contract in `log`

Instead of internalizing a few serialization frameworks, `log` could provide a public common contract for them to conform to:

```rust
// Instead of `Value::from_any` + `FromAny`

pub trait Visit {
    fn visit(&self, visitor: &mut dyn Visitor) -> Result<(), Error>;
}

pub trait Visitor {
    fn u64(&mut self, v: u64) -> Result<(), Error>;
    fn i64(&mut self, v: i64) -> Result<(), Error>;

    ..
}
```

This is fairly straightforward for primitive types like integers and strings, but becomes much more involved when dealing with complex values likes maps and sequences. Not supporting these complex structures is limiting, and reduces `log`s interoperability with other frameworks that do. A serialization framework needs to do more than just provide a contract, its API needs to work to support implementations on either side of that contract, otherwise it won't gain adoption.

Maintaining a useful serialization framework is a distraction for `log`. Serialization of structured values is a complex, necessary, but not primary function of `log`, so it should avoid owning that contract and the baggage that comes along with it if it can. That's why the `sval` library was created; to manage the necessary complexity of building a serialization framework that's suitable for structured logging externally from the `log` crate.

Within the `log` crate itself, internalizing fundamental serialization frameworks reduces the effort needed from building a complete framework down to shimming an existing framework. These shims would exist in the wider `log` ecosystem in either case. The effort of managing breaking changes in supported serialization frameworks isn't less than the effort of managing breaking changes in a common contract provided by `log`. The owner of that contract, whether it's `log` or `serde` or `sval`, has to consider the churn introduced by breakage. As a downstream consumer of that breakage, `log` is in the same boat as its consumers.

#### Just pick an existing framework

Instead of building a common shim around several serialization frameworks, `log` could just pick one and bake it in directly. This would have the benefit of offering the best end-user experience for existing users of that framework when interacting with the `log!` macros and `Source`s. It also means accepting the trade-offs that framework makes. For `serde`, that means requiring the standard library for boxing values. For `sval`, that means accepting churn as it matures.

The API proposed using the `Value` type as an opaque container along with APIs for specific frameworks is a reasonable middle-ground between baking in a specific framework and only offering a contract without any direct support. It gives producers of structured data a way to plug their data using a framework of choice into a `Value`, either directly or through compatibility with a supported framework. It gives consumers of structured data a first-class experience for plugging `Value`s into their framework of choice by deriving the appropriate traits. It also gives `log` room to add and deprecate support for frameworks if mindshare shifts in the future.

# Prior art
[prior-art]: #prior-art

Structured logging is a paradigm that's supported by logging frameworks in many language ecosystems.

## Rust

The `slog` library is a structured logging framework for Rust. Its API predates a stable `serde` crate so it defines its own traits that are similar to `serde::Serialize`. A log record consists of a rendered message and bag of structured key-value pairs. `slog` goes further than this RFC proposes by requiring callers of its `log!` macros to state whether key-values are owned or borrowed by the record, and whether the data is safe to share across threads.

This RFC proposes an API that's inspired by `slog`, but doesn't directly support distinguishing between owned or borrowed key-value pairs. Everything is borrowed. That means the only way to send a `Record` to another thread is to serialize it into a different type.

## Go

The `logrus` library is a structured logging framework for Go. It uses a similar separation of the textual log message from structured key-value pairs that this API proposes.

## .NET

The C# community has mostly standardized around using message templates for packaging a log message with structured key-value pairs. Instead of logging a rendered message and separate bag of structured data, the log record contains a template that allows key-value pairs to be interpolated from the same bag of structured data. It avoids duplicating the same information multiple times.

Supporting something like message templates in Rust using the `log!` macros would probably require procedural macros. A macro like that could be built on top of the API proposed by this RFC.

# Unresolved questions
[unresolved-questions]: #unresolved-questions
