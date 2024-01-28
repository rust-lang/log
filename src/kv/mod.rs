//! **UNSTABLE:** Structured logging.
//!
//! This module is unstable and breaking changes may be made
//! at any time. See [the tracking issue](https://github.com/rust-lang-nursery/log/issues/328)
//! for more details.
//!
//! Add the `kv_unstable` feature to your `Cargo.toml` to enable
//! this module:
//!
//! ```toml
//! [dependencies.log]
//! features = ["kv_unstable"]
//! ```
//!
//! # Structured logging in `log`
//!
//! Structured logging enhances traditional text-based log records with user-defined
//! attributes. Structured logs can be analyzed using a variety of tranditional
//! data processing techniques, without needing to find and parse attributes from
//! unstructured text first.
//!
//! In `log`, user-defined attributes are part of a [`Source`] on the [`LogRecord`].
//! Each attribute is a pair of [`Key`] and [`Value`]. Keys are strings and values
//! are a datum of any type that can be formatted or serialized. Simple types like
//! strings, booleans, and numbers are supported, as well as arbitrarily complex
//! structures involving nested objects and sequences.
//!
//! ## Adding attributes to log records
//!
//! Attributes appear after the message format in the `log!` macros:
//!
//! ```
//! ..
//! ```
//!
//! ## Working with attributes on log records
//!
//! Use the [`LogRecord::source`] method to access user-defined attributes.
//! Individual attributes can be pulled from the source:
//!
//! ```
//! ..
//! ```
//!
//! This is convenient when an attribute of interest is known in advance.
//! All attributes can also be enumerated using a [`Visitor`]:
//!
//! ```
//! ..
//! ```
//!
//! [`Value`]s in attributes have methods for conversions to common types:
//!
//! ```
//! ..
//! ```
//!
//! Values also have their own [`value::Visitor`] type:
//!
//! ```
//! ..
//! ```
//!
//! Visitors on values are lightweight and suitable for detecting primitive types.
//! To serialize a value, you can also use either `serde` or `sval`:
//! 
//! ```
//! ..
//! ```
//! 
//! If you're in a no-std environment, you can use `sval`. In other cases, you can use `serde`.
//!
//! Values can also always be formatted using the standard `Debug` and `Display`
//! traits:
//! 
//! ```
//! ..
//! ```

mod error;
mod key;
pub mod source;

pub mod value;

pub use self::error::Error;
pub use self::key::{Key, ToKey};
pub use self::source::{Source, Visitor};

#[doc(inline)]
pub use self::value::{ToValue, Value};
