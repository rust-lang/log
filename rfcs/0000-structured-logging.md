# Summary
[summary]: #summary

Add support for structured logging to the `log` crate in both `std` and `no_std` environments, allowing log records to carry typed data beyond a textual message. This document serves as an introduction to what structured logging is all about, and as an RFC for an implementation in the `log` crate.

`log` will depend on `serde` for structured logging support, which will be enabled by default. See [the implications for `log` users](#implications-for-dependents) for some more details. The API is heavily inspired by the `slog` logging framework.

> NOTE: Code in this RFC uses recent language features like `impl Trait`, but can be implemented without them.

# Contents

- [Motivation](#motivation)
  - [What is structured logging?](#what-is-structured-logging)
  - [Why do we need structured logging in `log`?](#why-do-we-need-structured-logging-in-log)
- [Guide-level explanation](#guide-level-explanation)
  - [Capturing structured logs](#capturing-structured-logs)
  - [Consuming structured logs](#consuming-structured-logs)
- [Reference-level explanation](#reference-level-explanation)
  - [Design considerations](#design-considerations)
  - [Implications for dependents](#implications-for-dependents)
  - [Cargo features](#cargo-features)
  - [Public API](#public-api)
    - [`Record` and `RecordBuilder`](#record-and-recordbuilder)
    - [`ToValue`](#tovalue)
    - [`Value`](#value)
    - [`ToKey`](#tokey)
    - [`Key`](#key)
    - [`Visitor`](#visitor)
    - [`Error`](#error)
    - [`KeyValueSource`](#keyvaluesource)
  - [The `log!` macros](#the-log-macros)
- [Drawbacks](#drawbacks)
  - [`Display + Serialize`](#display--serialize)
  - [`serde`](#serde)
- [Rationale and alternatives](#rationale-and-alternatives)
  - [Just use `Display`](#just-use-display)
  - [Don't use a blanket implementation](#dont-use-a-blanket-implementation)
  - [Define our own serialization trait](#define-our-own-serialization-trait)
  - [Don't enable structured logging by default](#dont-enable-structured-logging-by-default)
- [Prior art](#prior-art)
  - [Rust](#rust)
  - [Go](#go)
  - [.NET](#net)
- [Unresolved questions](#unresolved-questions)

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

On the surface there doesn't seem to be a lot of difference between `log` and `slog`, so why not just deprecate one in favour of the other? Conceptually, `log` and `slog` are different libraries that fill different use-cases, even if there's some overlap.

`slog` is a logging _framework_. It offers all the fundamental tools needed out-of-the-box to capture log records, define and implement the composable pieces of a logging pipeline, and pass them through that pipeline to an eventual destination. It has conventions and trade-offs baked into the design of its API. Loggers are treated explicitly as values in data structures and as arguments, and callers can control whether to pass owned or borrowed data.

`log` is a logging _facade_. It's only concerned with a standard, minimal API for capturing log records, and surfacing those records to some consumer. The tools provided by `log` are only those that are fundamental to the operation of the `log!` macro. From `log`'s point of view, a logging framework like `slog` is a black-box implementation of the `Log` trait. In this role, the `Log` trait can act as a common entrypoint for capturing log records. That means the `Record` type can act as a common container for describing a log record. `log` has its own set of trade-offs baked into the design of its API. The `log!` macro assumes a single, global entrypoint, and all data in a log record is borrowed from the callsite.

A healthy logging ecosystem needs both `log` and frameworks like `slog`. As a standard API, `log` can support a diverse but cohesive ecosystem of logging tools in Rust by acting as the glue between libraries, frameworks, and applications. A lot of libraries already depend on it. In order to really fulfil this role though, `log` needs to support structured logging so that libraries and their consumers can take advantage of it in a framework-agnostic way.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Capturing structured logs

Structured logging is supported in `log` by allowing typed key-value pairs to be associated with a log record.

### Structured vs unstructured

A `;` separates structured key-value pairs from values that are replaced into the message:

```rust
info!(
    "This is the rendered {message}. It is not structured",
    message = "message";
    correlation = correlation_id,
    user
);
```

Any `value` or `key = value` expressions before the `;` in the macro will be interpolated into the message as unstructured text. This is the `log!` macro we have today. Any `value` or `key = value` expressions after the `;` will be captured as structured key-value pairs. These structured key-value pairs can be inspected or serialized, retaining some notion of their original type. That means in the above example, the `message` key is unstructured, and the `correlation` and `user` keys are structured:

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

Any type that implements both `Display` and `serde::Serialize` can be used as the value in a structured key-value pair. In the previous example, the values used in the macro require the following trait bounds:

```
info!(
    "This is the rendered {message}. It is not structured",
    message = "message";
              ^^^^^^^^^
              Display

    correlation = correlation_id,
                  ^^^^^^^^^^^^^^
                  Display + Serialize

    user
    ^^^^
    Display + Serialize
);
```

### Logging data that isn't `Display + Serialize`

The `Display + Serialize` bounds we've been using thus far are mostly accurate, but don't tell the whole story. Structured values don't _technically_ require `Display + Serialize`. They require a trait for capturing structured values, `ToValue`, which we implement for any type that also implements `Display + Serialize`. So in truth the trait bounds required by the macro look like this:

```
info!(
    "This is the rendered {message}. It is not structured",
    message = "message";
              ^^^^^^^^^
              Display

    correlation = correlation_id,
                  ^^^^^^^^^^^^^^
                  ToValue

    user
    ^^^^
    ToValue
);
```

The `ToValue` trait makes it possible for types to be logged even if they don't implement `Display + Serialize`; they only need to implement `ToValue`. One of the goals of this RFC is to avoid creating another fundamental trait that needs to be implemented throughout the ecosystem though. So instead of implementing `ToValue` directly, we provide a few helper macros to wrap a value satisfying one set of trait bounds into a value that satisfies `ToValue`. The following example uses these helper macros to change the trait bounds required by `correlation_id` and `user`:

```rust
info!(
    "This is the rendered {message}. It is not structured",
    message = "message";
    correlation = log_fmt!(correlation_id, Debug::fmt),
    user = log_serde!(user)
);
```

They now look like this:

```
info!(
    "This is the rendered {message}. It is not structured",
    message = "message";
              ^^^^^^^^^
              Display

    correlation = log_fmt!(correlation_id, Debug::fmt),
                           ^^^^^^^^^^^^^^
                           Debug

    user = log_serde!(user)
                      ^^^^
                      Serialize
);
```

This same pattern can be applied to other types in the standard library and wider ecosystem that are likely to be logged, but don't typically satisfy `Display + Serialize`. Some examples are `Path`, and implementations of `Error` or `Fail`.

So why do we use `Display + Serialize` as the effective bounds in the first place? Practically, it's required to keep a consistent API in `no_std` environments, where an object-safe wrapper over `serde` requires standard library features. `Display` is a natural trait to lean on when `Serialize` isn't available, so requiring both means there's no change in the trait bounds when records can be captured using `serde` and when they can't (later on we'll see how we can still use `serde` for primitive types in `no_std` environments).

`Display` also suggests the data we are logging should have some canonical representation that's useful for someone to look at. Finding small but sufficient representations of potentially large pieces of state to log will make those logs easier to ingest and analyze later.

## Consuming structured logs

Capturing structured logs is only half the story. Implementors of the `Log` trait also need to be able to work with any key-value pairs associated with a log record.

### Using `Visitor` to print or serialize key-value pairs

Structured key-value pairs can be inspected using the `Visitor` trait. Take the terminal log format from before:

```
[INF 2018-09-27T09:32:03Z] Operation completed successfully in 18ms
module: "basic"
service: "database"
correlation: 123
took: 18
```

Each key-value pair, shown as `$key: $value`, can be written using a `Visitor` that hands values to a `serde::Serializer`. The implementation of that `Visitor` could look like this:

```rust
fn write_pretty(w: impl Write, r: &Record) -> io::Result<()> {
    // Write the first line of the log record
    ...

    // Write each key-value pair using the `WriteKeyValues` visitor
    record
        .key_values()
        .visit(&mut WriteKeyValues(w))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.into_error()))
}

struct WriteKeyValues<W>(W);

impl<'kvs, W> Visitor<'kvs> for WriteKeyValues<W>
where
    W: Write,
{
    fn visit_pair(&mut self, k: Key<'kvs>, v: Value<'kvs>) -> Result<(), Error> {
        // Write the key
        // `Key` is a wrapper around a string
        // It can be formatted directly
        write!(&mut self.0, "{}: ", k)?;

        // Write the value
        // `Value` is a wrapper around `serde::Serialize`
        // It can't be formatted directly
        v.serialize(&mut Serializer::new(&mut self.0))?;

        Ok(())
    }
}
```

Needing `serde` for the human-centric format above seems a bit unnecessary. The value becomes clearer when using a machine-centric format for the entire log record, like json. Take the following json format:

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

Defining a serializable structure for this format could be done using `serde_derive`, and then written using `serde_json`:

```rust
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

There's no explicit `Visitor` in this case, because the `serialize_as_map` method wraps one up internally so all key-value pairs are serializable as a map using `serde`.

### Using `KeyValueSource` to capture key-value pairs

What exactly are the key-value pairs on a record? The previous examples used a `key_values()` method on `Record` to get _something_ that could be visited or serialized using a `Visitor`. That something is an implementation of a trait, `KeyValueSource`, which holds the actual `Key` and `Value` pairs:

```
fn write_pretty(w: impl Write, r: &Record) -> io::Result<()> {
    ...

    record
        .key_values()
        ^^^^^^^^^^^^^
        `Record::key_values` returns `impl KeyValueSource`

        .visit(&mut WriteKeyValues(w))
        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
        `KeyValueSource::visit` takes `&mut Visitor` 
}
```

As an example of a `KeyValueSource`, the `log!` macros can capture key-value pairs into an array of `(&str, &dyn ToValue)` tuples that can be visited in sequence:

```rust
impl<'a> KeyValueSource for [(&'a str, &'a dyn ToValue)] {
    fn visit<'kvs>(&'kvs self, visitor: &mut dyn Visitor<'kvs>) {
        for &(k, v) in self {
            visitor.visit_pair(k.to_key(), v.to_value())?;
        }

        Ok(())
    }
}
```

A `KeyValueSource` doesn't have to just contain key-value pairs directly like this though. It could also act like an adapter, like we have for iterators in the standard library. As another example, the following `KeyValueSource` doesn't store any key-value pairs of its own, it will sort and de-duplicate pairs read from another source by first reading them into a map before forwarding them on:

```rust
pub struct SortRetainLast<KVS>(KVS);

impl<KVS> KeyValueSource for SortRetainLast<KVS>
where
    KVS: KeyValueSource,
{
    fn visit<'kvs>(&'kvs self, visitor: &mut dyn Visitor<'kvs>) {
        // `Seen` is a visitor that will capture key-value pairs
        // in a `BTreeMap`. We use it internally to sort and de-duplicate
        // the key-value pairs that `SortRetainLast` is wrapping.
        struct Seen<'kvs>(BTreeMap<Key<'kvs>, Value<'kvs>>);

        impl<'kvs> Visitor<'kvs> for Seen<'kvs> {
            fn visit_pair<'vis>(&'vis mut self, k: Key<'kvs>, v: Value<'kvs>) {
                self.0.insert(k, v);
            }
        }

        // Visit the inner source and collect its key-value pairs into `seen`
        let mut seen = Seen(BTreeMap::new());
        self.0.visit(&mut seen);

        // Iterate through the seen key-value pairs in order
        // and pass them to the `visitor`.
        for (k, v) in seen.0 {
            visitor.visit_pair(k, v);
        }
    }
}
```

This API is similar to `serde` and `slog`, and is very flexible. Other structured logging concepts like contextual logging, where a record is enriched with information from its envirnment, can be built on top of `KeyValueSource` and `Visitor`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Design considerations

### Don't break anything

Allow structured logging to be added in the current `0.4.x` series of `log`.

### Leverage the ecosystem

Rather than trying to define our own serialization trait and require libraries in the ecosystem implement it, we leverage `serde`. Any new types that emerge in the ecosystem don't have another fundamental trait they need to think about.

### Object safety

`log` is already designed to be object-safe so this new structured logging API needs to be object-safe too.

### Borrowed vs owned

`Record` borrows all data from the call-site so records need to be handled directly on-thread as they're produced. On the one hand that means that log records need to be serialized before they can be sent across threads. On the other hand it means callers don't need to make assumptions about whether records need to be owned or borrowed.

## Implications for dependents

Dependents of `log` will notice the following:

In `no_std` environments (which is the default for `log`):

- `serde` will enter the `Cargo.lock` if it wasn't there already. This will impact compile-times.
- Artifact size of `log` will increase.

In `std` environments (which is common when using `env_logger` and other crates that implement `log`):

- `serde` and `erased-serde` will enter the `Cargo.lock` if it wasn't there already. This will impact compile-times.
- Artifact size of `log` will increase.

In either case, `serde` will become a public dependency of the `log` crate, so any breaking changes to `serde` will result in breaking changes to `log`.

## Cargo features

Structured logging will be supported in either `std` or `no_std` contexts using Cargo features:

```toml
[features]
# support structured logging by default
default = ["structured"]

# semantic name for the structured logging feature
structured = ["serde"]

# when `std` is available, always support structured logging
# with `erased-serde`
std = ["structured", "erased-serde"]
```

Using default features, structured logging will be supported by `log` in `no_std` environments. Structured logging will always be available when using the `std` feature (usually pulled in by libraries that implement the `Log` trait).

## Public API

For context, ignoring the `log!` macros, this is roughly the additional public API this RFC proposes to support structured logging:

```rust
impl<'a> RecordBuilder<'a> {
    /// Set the key-value pairs on a log record.
    #[cfg(feature = "serde")]
    pub fn key_values(&mut self, kvs: ErasedKeyValues<'a>) -> &mut RecordBuilder<'a>;
}

impl<'a> Record<'a> {
    /// Get the key-value pairs.
    #[cfg(feature = "serde")]
    pub fn key_values(&self) -> ErasedKeyValues;

    /// Get a builder that's preconfigured from this record.
    pub fn to_builder(&self) -> RecordBuilder;
}

#[cfg(features = "serde")]
pub mod key_values {
    /// A visitor for a set of key-value pairs.
    /// 
    /// The visitor is driven by an implementation of `KeyValueSource`.
    /// The visitor expects keys and values that satisfy a given lifetime.
    pub trait Visitor<'kvs> {
        /// Visit a single key-value pair.
        fn visit_pair(&mut self, k: Key<'kvs>, v: Value<'kvs>) -> Result<(), Error>;
    }

    /// A source for key-value pairs.
    pub trait KeyValueSource {
        /// Serialize the key value pairs.
        fn visit<'kvs>(&'kvs self, visitor: &mut dyn Visitor<'kvs>) -> Result<(), Error>;

        /// Erase this `KeyValueSource` so it can be used without
        /// requiring generic type parameters.
        fn erase(&self) -> ErasedKeyValueSource
        where
            Self: Sized {}

        /// Find the value for a given key.
        /// 
        /// If the key is present multiple times, this method will
        /// return the *last* value for the given key.
        fn get<'kvs, Q>(&'kvs self, key: Q) -> Option<Value<'kvs>>
        where
            Q: ToKey {}

        /// An adapter to borrow self.
        fn by_ref(&self) -> &Self {}

        /// Chain two `KeyValueSource`s together.
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
    }

    /// An erased `KeyValueSource`.
    pub struct ErasedKeyValueSource<'a> {}

    impl<'a> ErasedKeyValueSource<'a> {
        /// Capture a `KeyValueSource` and erase its concrete type.
        pub fn new(kvs: &'a impl KeyValueSource) -> Self {}
    }

    impl<'a> Clone for ErasedKeyValueSource<'a> {}
    impl<'a> Default for ErasedKeyValueSource<'a> {}
    impl<'a> KeyValueSource for ErasedKeyValueSource<'a> {}

    /// A `KeyValueSource` adapter that visits key-value pairs
    /// in sequence.
    /// 
    /// This is the result of calling `chain` on a `KeyValueSource`.
    pub struct Chain<A, B> {}

    impl<A, B> KeyValueSource for Chain<A, B>
    where
        A: KeyValueSource,
        B: KeyValueSource {}

    /// A `KeyValueSource` adapter that can be serialized as
    /// a map using `serde`.
    /// 
    /// This is the result of calling `serialize_as_map` on
    /// a `KeyValueSource`.
    pub struct SerializeAsMap<KVS> {}

    impl<KVS> Serialize for SerializeAsMap<KVS>
    where
        KVS: KeyValueSource {}

    impl<K, V> KeyValueSource for (K, V)
    where
        K: ToKey,
        V: ToValue {}

    impl<KVS> KeyValueSource for [KVS]
    where
        KVS: KeyValueSource {}

    #[cfg(feature = "std")]
    impl<KVS> KeyValueSource for Vec<KVS>
    where
        KVS: KeyValueSource {}

    #[cfg(feature = "std")]
    impl<K, V> KeyValueSource for BTreeMap<K, V>
    where
        K: Borrow<str> + Ord,
        V: ToValue {}

    #[cfg(feature = "std")]
    impl<K, V> KeyValueSource for HashMap<K, V>
    where
        K: Borrow<str> + Eq + Hash,
        V: ToValue {}

    /// A type that can be converted into a borrowed key.
    pub trait ToKey {
        /// Perform the conversion.
        fn to_key(&self) -> Key;
    }

    impl ToKey for str {}

    #[cfg(feature = "std")]
    impl ToKey for String {}

    #[cfg(feature = "std")]
    impl<'a> ToKey for Cow<'a, str> {}

    /// A key in a key-value pair.
    /// 
    /// The key can be treated like `&str`.
    pub struct Key<'kvs> {}

    impl<'kvs> Key<'kvs> {
        /// Get a key from a borrowed string.
        pub fn from_str(key: &'a (impl AsRef<str> + ?Sized)) -> Self;

        /// Get a reference to the key as a string.
        pub fn as_str(&self) -> &str;
    }

    impl<'kvs> Serialize for Key<'kvs> {}
    impl<'kvs> PartialEq for Key<'kvs> {}
    impl<'kvs> Eq for Key<'kvs> {}
    impl<'kvs> PartialOrd for Key<'kvs> {}
    impl<'kvs> Ord for Key<'kvs> {}
    impl<'kvs> Hash for Key<'kvs> {}
    impl<'kvs> AsRef<str> for Key<'kvs> {}

    #[cfg(feature = "std")]
    impl<'kvs> Borrow<str> for Key<'kvs> {}

    /// A type that can be converted into a borrowed value.
    pub trait ToValue {
        /// Perform the conversion.
        fn to_value(&self) -> Value;
    }

    impl<T> ToValue for T
    where
        T: Display + Serialize {}

    /// A value in a key-value pair.
    /// 
    /// The value can be treated like `serde::Serialize`.
    pub struct Value<'kvs> {}

    impl<'kvs> Value<'kvs> {
        /// Get a value that will choose either the `Display`
        /// or `Serialize` implementation based on the platform.
        /// 
        /// If the standard library is available, the `Serialize`
        /// implementation will be used. If the standard library
        /// is not available, the `Display` implementation will
        /// probably be used.
        pub fn new(v: &'kvs (impl Display + Serialize)) -> Self;

        /// Get a value that can be serialized as a string using
        /// its `Display` implementation.
        pub fn from_display(v: &'kvs impl Display) -> Self;

        /// Get a value that can be serialized as structured
        /// data using its `Serialize` implementation.
        #[cfg(feature = "std")]
        pub fn from_serde(v: &'kvs impl Serialize) -> Self;
    }

    impl<'kvs> Serialize for Value<'kvs> {}
    impl<'kvs> ToValue for Value<'kvs> {}
    impl<'a, 'kvs> ToValue for &'a Value<'kvs> {}

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
        pub fn into_serde<E>(self) -> E
        where
            E: serde::ser::Error {}
    }

    impl<E> From<E> for Error
    where
        E: std::error::Error {}

    impl From<Error> for Box<dyn std::error::Error + Send + Sync> {}
    impl AsRef<dyn std::error::Error + Send + Sync + 'static> for Error {}
}
```

### `Record` and `RecordBuilder`

Structured key-value pairs can be set on a `RecordBuilder`:

```rust
impl<'a> RecordBuilder<'a> {
    /// Set key values
    #[cfg(feature = "serde")]
    pub fn key_values(&mut self, kvs: ErasedKeyValueSource<'a>) -> &mut RecordBuilder<'a> {
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

    #[cfg(feature = "serde")]
    kvs: ErasedKeyValueSource<'a>,
}

impl<'a> Record<'a> {
    /// The key value pairs attached to this record.
    /// 
    /// Pairs aren't guaranteed to be unique (the same key may be repeated with different values).
    #[cfg(feature = "serde")]
    pub fn key_values(&self) -> ErasedKeyValueSource {
        self.kvs.clone()
    }
}
```

### `ToValue`

A `ToValue` is a potentially long-lived structure that can be converted into a `Value`:

```rust
/// A type that can be converted into a borrowed value.
pub trait ToValue {
    /// Perform the conversion.
    fn to_value(&self) -> Value;
}

impl<'a> ToValue for &'a dyn ToValue {
    fn to_value(&self) -> Value {
        (*self).to_value()
    }
}
```

#### Implementors

`ToValue` requires a blanket implementation to be most useful. This covers any `V: Display + Serialize`:

```rust
impl<T: serde::Serialize + fmt::Display> ToValue for T {
    fn to_value(&self) -> Value {
        Value::new(self)
    }
}
```

### `Value`

A `Value` is a short-lived structure that can be serialized using `serde`. This might require losing some type information about the underlying value and serializing it as a string:

```rust
/// A value in a key-value pair.
/// 
/// The value can be treated like `serde::Serialize`.
pub struct Value<'kvs> {
    inner: ValueInner<'kvs>,
}

#[derive(Clone, Copy)]
enum ValueInner<'kvs> {
    /// The value will be serialized as a string
    /// using its `Display` implementation.
    Display(&'kvs dyn fmt::Display),
    /// The value will be serialized as a structured
    /// type using its `Serialize` implementation.
    #[cfg(feature = "erased-serde")]
    Serde(&'kvs dyn erased_serde::Serialize),
    /// The value will be serialized as a structured
    /// type using its primitive `Serialize` implementation.
    #[cfg(not(feature = "erased-serde"))]
    Primitive(&'kvs dyn ToPrimitive),
}

impl<'kvs> ToValue for Value<'kvs> {
    fn to_value(&self) -> Value {
        Value { inner: self.inner }
    }
}

impl<'a, 'kvs> ToValue for &'a Value<'kvs> {
    fn to_value(&self) -> Value {
        Value { inner: self.inner }
    }
}

impl<'kvs> Serialize for Value<'kvs> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.inner {
            ValueInner::Display(v) => serializer.collect_str(&v),

            #[cfg(feature = "erased-serde")]
            ValueInner::Serde(v) => v.serialize(serializer),

            #[cfg(not(feature = "erased-serde"))]
            ValueInner::Primitive(v) => {
                // We expect `Value::new` to correctly determine
                // whether or not a value is a simple primitive
                let v = v
                    .to_primitive()
                    .ok_or_else(|| S::Error::custom("captured value is not primitive"))?;

                v.serialize(serializer)
            },
        }
    }
}
```

#### Capturing values

Methods on `Value` allow it to capture and erase types that implement combinations of `Serialize` and `Display`:

```rust
impl<'kvs> Value<'kvs> {
    /// Create a new value.
    /// 
    /// The value must implement both `serde::Serialize` and `fmt::Display`.
    /// Either implementation will be used depending on whether the standard
    /// library is available, but is exposed through the same API.
    /// 
    /// In environments where the standard library is available, the `Serialize`
    /// implementation will be used.
    /// 
    /// In environments where the standard library is not available, some
    /// primitive stack-based values can retain their structure instead of falling
    /// back to `Display`.
    pub fn new(v: &'kvs (impl Serialize + Display)) -> Self {
        Value {
            inner: {
                #[cfg(feature = "erased-serde")]
                {
                    ValueInner::Serde(v)
                }

                #[cfg(not(feature = "erased-serde"))]
                {
                    // Try capture a primitive value
                    if v.to_primitive().is_some() {
                        ValueInner::Primitive(v)
                    } else {
                        ValueInner::Display(v)
                    }
                }
            }
        }
    }

    /// Get a `Value` from a displayable reference.
    pub fn from_display(v: &'kvs impl Display) -> Self {
        Value {
            inner: ValueInner::Display(v),
        }
    }

    /// Get a `Value` from a serializable reference.
    #[cfg(feature = "erased-serde")]
    pub fn from_serde(v: &'kvs impl Serialize) -> Self {
        Value {
            inner: ValueInner::Serde(v),
        }
    }
}
```

`Value::new` will choose either the `Serialize` or `Display` implementation, depending on whether `std` is available. If it is, `Value::new` will use `Serialize` and is equivalent to `Value::from_serde`, if it's not `Value::new` will use `Display` and is equivalent to `Value::from_display`.

`Value::new` can use the `Serialize` implementation for some fixed set of primitives, like `i32` and `bool` even if `std` is not available though. This can be done by capturing those values at the point that the serializer is known into a `Primitive` wrapper:

```rust
/// Convert a value into a primitive with a known type.
/// 
/// The `ToPrimitive` trait lets us pass trait objects around
/// that are always the same size, rather than bloating values
/// to the size of the largest primitive.
pub trait ToPrimitive {
    /// Perform the conversion.
    fn to_primitive(&self) -> Option<Primitive>;
}

impl<T> ToPrimitive for T
where
    T: Serialize,
{
    fn to_primitive(&self) -> Option<Primitive> {
       self.serialize(PrimitiveSerializer).ok()
    }
}

#[derive(Clone, Copy)]
pub struct Primitive(PrimitiveInner);

#[derive(Clone, Copy)]
enum PrimitiveInner {
    Unsigned(u64),
    Signed(i64),
    Float(f64),
    Bool(bool),
    Char(char),

    #[cfg(feature = "i128")]
    BigUnsigned(u128),
    
    #[cfg(feature = "i128")]
    BigSigned(i128),
}

impl Serialize for Primitive {}

struct PrimitiveSerializer;

impl Serializer for PrimitiveSerializer {
    type Ok = Primitive;
    type Error = Invalid;

    ...
}
```

The `Primitive` type stored in a `Value` is another trait object. This is just to ensure the size of `Value` doesn't grow to the size of `Primitive`'s largest variant, which is a 128bit number.

The `Value::from_serde` method requires `std` because it uses `erased_serde` as an object-safe wrapper around `serde`, which itself requires `std`.

#### Ownership

The `Value` type borrows from its inner value.

#### Thread-safety

The `Value` type doesn't try to guarantee that values are `Send` or `Sync`, and doesn't offer any way of retaining that information when erasing.

### `ToKey`

A `ToKey` is a potentially long-lived structure that can be converted into a `Key`:

```rust
/// A type that can be converted into a borrowed key.
pub trait ToKey {
    /// Perform the conversion.
    fn to_key(&self) -> Key;
}

impl<'a, K: ?Sized> ToKey for &'a K
where
    K: ToKey,
{
    fn to_key(&self) -> Key {
        (*self).to_key()
    }
}
```

#### Implementors

`ToKey` is implemented for `str`. This is supported in both `std` and `no_std` contexts:

```rust
impl ToKey for str {
    fn to_key(&self) -> Key {
        Key::from_str(self)
    }
}
```

When `std` is available, `Key` is also implemented for other string containers:

```rust
#[cfg(feature = "std")]
impl ToKey for String {
    fn to_key(&self) -> Key {
        Key::from_str(self)
    }
}

#[cfg(feature = "std")]
impl<'a> ToKey for borrow::Cow<'a, str> {
    fn to_key(&self) -> Key {
        Key::from_str(self.as_ref())
    }
}
```

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

impl<'kvs> ToKey for Key<'kvs> {
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

Other standard implementations could be added for any `K: AsRef<str>` in the same fashion.

#### Ownership

The `Key` type borrows its inner value.

#### Thread-safety

The `Key` type is probably `Send` + `Sync`, but that's not guaranteed.

### `Visitor`

The `Visitor` trait used by `KeyValueSource` can visit a single key-value pair:

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

There aren't any public implementors of `Visitor` in the `log` crate, but the `KeyValueSource::try_for_each` and `KeyValueSource::serialize_as_map` methods use the trait internally.

Other crates that use key-value pairs will implement `Visitor`.

#### Object safety

The `Visitor` trait is object-safe.

### `Error`

Just about the only thing you can do with a `Value` in the `Visitor::visit_pair` method is serialize it with `serde`. Serialization might fail, so to allow errors to get carried back to callers the `visit_pair` method needs to return a `Result`.

```rust
pub struct Error(Inner);

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

impl AsRef<dyn std::error::Error + Send + Sync + 'static> for Error {
    fn as_ref(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        self.as_error()
    }
}

enum Inner {
    Static(&'static str),
    #[cfg(feature = "std")]
    Owned(String),
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

There's no really universal way to handle errors in a logging pipeline. Knowing that some error occurred, and knowing where, should be enough for implementations of `Log` to decide how to handle it. The `Error` type doesn't try to be a general-purpose error management tool, it tries to make it easy to early-return with other errors encountered during `Visitor::visit_pair`.

To make it possible to carry any arbitrary `S::Error` type, where we don't know how long the value can live for and whether it's `Send` or `Sync`, without extra work, the `Error` type does not attempt to store the error value itself. It just converts it into a `String`.

### `KeyValueSource`

The `KeyValueSource` trait is a bit like `Serialize`. It gives us a way to inspect some arbitrary collection of key-value pairs using a visitor pattern:

```rust
pub trait KeyValueSource {
    fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error>;

    ...
}

impl<'a, T: ?Sized> KeyValueSource for &'a T
where
    T: KeyValueSource,
{
    fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error> {
        (*self).visit(visitor)
    }
}
```

`KeyValueSource` doesn't make any assumptions about how many key-value pairs it contains or how they're visited. That means the visitor may observe keys in any order, and observe the same key multiple times.

#### Adapters

Some useful adapters exist as provided methods on the `KeyValueSource` trait. They're similar to adapters on the standard `Iterator` trait:

```rust
pub trait KeyValueSource {
    ...

    /// Erase this `KeyValueSource` so it can be used without
    /// requiring generic type parameters.
    fn erase(&self) -> ErasedKeyValueSource
    where
        Self: Sized,
    {
        ErasedKeyValueSource::erased(self)
    }

    /// An adapter to borrow self.
    fn by_ref(&self) -> &Self {
        self
    }

    /// Chain two `KeyValueSource`s together.
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
        Q: ToKey,
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
    fn serialize_as_map(self) -> SerializeAsMap<Self>
    where
        Self: Sized,
    {
        SerializeAsMap(self)
    }
}
```

- `by_ref` to get a reference to a `KeyValueSource` within a method chain.
- `chain` to concatenate one source with another. This is useful for composing implementations of `Log` together for contextual logging.
- `get` to try find the value associated with a key.
- `try_for_each` to try execute some closure over all key-value pairs. This is a convenient way to do something with each key-value pair without having to create and implement a `Visitor`.
- `serialize_as_map` to get a serializable map. This is a convenient way to serialize key-value pairs without having to create and implement a `Visitor`.

None of these methods are required for the core API. They're helpful tools for working with key-value pairs with minimal machinery. Even if we don't necessarily include them right away it's worth having an API that can support them later without breakage.

#### Object safety

`KeyValueSource` is not object-safe because of the provided adapter methods not being object-safe. The only required method, `visit`, is safe though, so an object-safe version of `KeyValueSource` that forwards this method can be reasonably written.

```rust
/// An erased `KeyValueSource`.
#[derive(Clone)]
pub struct ErasedKeyValueSource<'a>(&'a dyn ErasedKeyValueSourceBridge);

impl<'a> ErasedKeyValueSource<'a> {
    /// Capture a `KeyValueSource` and erase its concrete type.
    pub fn new(kvs: &'a impl KeyValueSource) -> Self {
        ErasedKeyValueSource(kvs)
    }
}

impl<'a> Default for ErasedKeyValueSource<'a> {
    fn default() -> Self {
        ErasedKeyValueSource(&(&[] as &[(&str, &dyn ToValue)]))
    }
}

impl<'a> KeyValueSource for ErasedKeyValueSource<'a> {
    fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error> {
        self.0.erased_visit(visitor)
    }

    fn get<'kvs, Q>(&'kvs self, key: Q) -> Option<Value<'kvs>>
    where
        Q: ToKey,
    {
        let key = key.to_key();
        self.0.erased_get(key.as_ref())
    }
}

/// A trait that erases a `KeyValueSource` so it can be stored
/// in a `Record` without requiring any generic parameters.
trait ErasedKeyValueSourceBridge {
    fn erased_visit<'kvs>(&'kvs self, visitor: &mut dyn Visitor<'kvs>) -> Result<(), Error>;
    fn erased_get<'kvs>(&'kvs self, key: &str) -> Option<Value<'kvs>>;
}

impl<KVS> ErasedKeyValueSourceBridge for KVS
where
    KVS: KeyValueSource + ?Sized,
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

A `KeyValueSource` with a single pair is implemented for a tuple of a key and value:

```rust
impl<K, V> KeyValueSource for (K, V)
where
    K: ToKey,
    V: ToValue,
{
    fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error> {
        visitor.visit_pair(self.0.to_key(), self.1.to_value())
    }
}
```

A `KeyValueSource` with multiple pairs is implemented for arrays of `KeyValueSource`s:

```rust
impl<KVS> KeyValueSource for [KVS] where KVS: KeyValueSource {
    fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error> {
        for kv in self {
            kv.visit(&mut visitor)?;
        }

        Ok(())
    }
}
```

When `std` is available, `KeyValueSource` is implemented for some standard collections too:

```rust
#[cfg(feature = "std")]
impl<KVS> KeyValueSource for Vec<KVS> where KVS: KeyValueSource {
    fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error> {
        self.as_slice().visit(visitor)
    }
}

#[cfg(feature = "std")]
impl<K, V> KeyValueSource for collections::BTreeMap<K, V>
where
    K: Borrow<str> + Ord,
    V: ToValue,
{
    fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error> {
        for (k, v) in self {
            visitor.visit_pair(k.to_key(), v.to_value())?;
        }

        Ok(())
    }

    fn get<'kvs, Q>(&'kvs self, key: Q) -> Option<Value<'kvs>>
    where
        Q: ToKey,
    {
        let key = key.to_key();
        collections::BTreeMap::get(self, key.as_ref()).map(ToValue::to_value)
    }
}

#[cfg(feature = "std")]
impl<K, V> KeyValueSource for collections::HashMap<K, V>
where
    K: Borrow<str> + Eq + Hash,
    V: ToValue,
{
    fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error> {
        for (k, v) in self {
            visitor.visit_pair(k.to_key(), v.to_value())?;
        }

        Ok(())
    }

    fn get<'kvs, Q>(&'kvs self, key: Q) -> Option<Value<'kvs>>
    where
        Q: ToKey,
    {
        let key = key.to_key();
        collections::HashMap::get(self, key.as_ref()).map(ToValue::to_value)
    }
}
```

The `BTreeMap` and `HashMap` implementations provide more efficient implementations of `KeyValueSource::get`.

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

        let kvs: &[(&str, &dyn::key_values::ToValue)] =
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

### Logging values that aren't `ToValue`

Not every type a user might want to log will satisfy the default `Display + Serialize` bound. To reduce friction in these cases, there are a few helper macros that can be used to tweak the way structured data is captured.

The pattern here is pretty general, you could imagine other macros being created for capturing other useful trait implementors, like `T: Fail`.

#### `log_serde!`

The `log_serde!` macro allows any type that implements just `Serialize` to be logged:

```rust
info!(
    "A log statement";
    user = log_serde!(user)
);
```

The macro definition looks like this:

```rust
macro_rules! log_serde {
    ($v:expr) => {
        $crate::adapter::log_serde(&$v)
    }
}

#[cfg(feature = "erased-serde")]
pub fn log_serde(v: impl Serialize) -> impl ToValue {
    struct SerdeAdapter<T>(T);

    impl<T> ToValue for SerdeAdapter<T>
    where
        T: Serialize,
    {
        fn to_value(&self) -> Value {
            Value::from_serde(&self.0)
        }
    }

    SerdeAdapter(v)
}
```

#### `log_fmt!`

The `log_fmt!` macro allows any type to be logged as a formatted string:

```rust
info!(
    "A log statement";
    user = log_fmt!(user, Debug::fmt)
);
```

The macro definition looks like this:

```rust
macro_rules! log_fmt {
    ($v:expr, $f:expr) => {
        $crate::adapter::log_fmt(&$v, $f)
    }
}

pub fn log_fmt<T>(value: T, adapter: impl Fn(&T, &mut fmt::Formatter) -> fmt::Result) -> impl ToValue {
    struct FmtAdapter<T, F> {
        value: T,
        adapter: F,
    }

    impl<T, F> Display for FmtAdapter<T, F>
    where
        F: Fn(&T, &mut Formatter) -> fmt::Result,
    {
        fn fmt(&self, f: &mut Formatter) -> fmt::Result {
            (self.adapter)(&self.value, f)
        }
    }

    impl<T, F> ToValue for FmtAdapter<T, F>
    where
        F: Fn(&T, &mut Formatter) -> fmt::Result,
    {
        fn to_value(&self) -> Value {
            Value::from_display(self)
        }
    }

    FmtAdapter { value, adapter }
}
```

#### `log_path!`

The `log_path!` macro allows `Path` and `PathBuf` to be logged:

```rust
info!(
    "A log statement";
    path = log_path!(path)
);
```

The macro definition looks like this:

```rust
macro_rules! log_path {
    ($v:expr) => {
        $crate::adapter::log_path(&$v)
    }
}

#[cfg(feature = "std")]
pub fn log_path(v: impl AsRef<Path>) -> impl ToValue {
    #[derive(Debug)]
    struct PathAdapter<T>(T);

    impl<T> Display for PathAdapter<T>
    where
        T: AsRef<Path>,
    {
        fn fmt(&self, f: &mut Formatter) -> fmt::Result {
            let path = self.0.as_ref();

            match path.to_str() {
                Some(path) => Display::fmt(path, f),
                None => Debug::fmt(path, f),
            }
        }
    }

    impl<T> serde::Serialize for PathAdapter<T>
    where
        T: AsRef<Path>,
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer
        {
            serializer.collect_str(&self)
        }
    }

    PathAdapter(v)
}
```

# Drawbacks
[drawbacks]: #drawbacks

Structured logging is a non-trivial feature to support.

## `Display + Serialize`

Using `Display + Serialize` as a blanket implementation for `ToValue` means we immediately need to work around some values that will probably be logged, but don't satisfy the bound:

- `Path` and `PathBuf`
- Most `std::error::Error`s
- Most `failure::Fail`s

Using `Debug + Serialize` would allow `Path`s to be logged with no extra effort, but `Display` is probably a more appropriate trait to use.

In `no_std` contexts, more machinery is required to try retain the structure of primitive values, rather than falling back to string serialization through the `Display` bound.

## `serde`

Using `serde` as the serialization framework for structured logging introduces a lot of complexity that consumers of key-value pairs need to deal with. Inspecting values requires an implementation of a `Serializer`, which is a complex trait.

Having `serde` enabled by default means it'll be effectively impossible to compile `log` in any reasonably sized dependency graph without also compiling `serde`.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Just use `Display`

This API could be a lot simpler if key-value pairs were only required to implement the `Display` trait. Unfortunately, this doesn't really provide structured logging, because `Display` doesn't retain any structure. Take the json example from before:

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

If key-value pairs were implemented as `Display` then this json object would look like this:

```json
{
    "ts": "1538040723000",
    "lvl": "INFO",
    "msg": "Operation completed successfully in 18ms",
    "module": "basic",
    "service": "database",
    "correlation": "123",
    "took": "18"
}
```

Now without knowing that `ts` is a number we can't reasonably use that field for querying over ranges of events.

## Don't use a blanket implementation

Without a blanket implementation of `ToValue`, a set of primitive and standard types could manually implement the trait in `log`. That could include `Path` and `PathBuf`, which don't currently satify the blanket `Display + Serialize` bounds. This would require libraries providing types in the ecosystem to depend on `log` and implement `ToValue` themselves.

## Define our own serialization trait

Rather than rely on `serde`, define our own simplified, object-safe serialization trait. This would avoid the complexity of erasing `Serialize` implementations, but would introduce the same ecosystem leakage as not using a blanket implementation. It would also be a trait that looks a lot like `Serialize`, without necessarily keeping up with improvements that are made in `serde`.

## Don't enable structured logging by default

Make structured logging a purely optional feature, so it wouldn't necessarily need to support `no_std` environments at all and could avoid the `Display + Serialize` blanket implementation bounds on `ToValue`. This seems appealing, but isn't ideal because it makes a fundamental modern logging feature much less discoverable.

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

- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?

-----

