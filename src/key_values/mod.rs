//! Structured key-value pairs.

mod error;
pub mod source;
pub mod key;
pub mod value;

pub use self::error::KeyValueError;
pub use self::source::Source;
pub use self::key::Key;
pub use self::value::Value;
