//! Sources for key-value pairs.

use kv::{Error, Key, ToKey, Value, ToValue};

/// A source of key-value pairs.
/// 
/// The source may be a single pair, a set of pairs, or a filter over a set of pairs.
/// Use the [`Visitor`](struct.Visitor.html) trait to inspect the structured data
/// in a source.
pub trait Source {
    /// Visit key-value pairs.
    /// 
    /// A source doesn't have to guarantee any ordering or uniqueness of pairs.
    fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error>;
}

impl<'a, T> Source for &'a T
where
    T: Source + ?Sized,
{
    fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error> {
        (**self).visit(visitor)
    }
}

impl<K, V> Source for (K, V)
where
    K: ToKey,
    V: ToValue,
{
    fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error> {
        visitor.visit_pair(self.0.to_key(), self.1.to_value())
    }
}

impl<S> Source for [S]
where
    S: Source,
{
    fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error> {
        for source in self {
            source.visit(visitor)?;
        }

        Ok(())
    }
}

impl<S> Source for Option<S>
where
    S: Source,
{
    fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error> {
        if let Some(ref source) = *self {
            source.visit(visitor)?;
        }

        Ok(())
    }
}

/// A visitor for the key-value pairs in a [`Source`](trait.Source.html).
pub trait Visitor<'kvs> {
    /// Visit a key-value pair.
    fn visit_pair(&mut self, key: Key<'kvs>, value: Value<'kvs>) -> Result<(), Error>;
}

impl<'a, 'kvs, T> Visitor<'kvs> for &'a mut T
where
    T: Visitor<'kvs> + ?Sized,
{
    fn visit_pair(&mut self, key: Key<'kvs>, value: Value<'kvs>) -> Result<(), Error> {
        (**self).visit_pair(key, value)
    }
}

#[cfg(feature = "std")]
mod std_support {
    use super::*;

    impl<S> Source for Box<S>
    where
        S: Source + ?Sized,
    {
        fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error> {
            (**self).visit(visitor)
        }
    }

    impl<S> Source for Vec<S>
    where
        S: Source,
    {
        fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error> {
            (**self).visit(visitor)
        }
    }

    impl<'kvs, V> Visitor<'kvs> for Box<V>
    where
        V: Visitor<'kvs> + ?Sized,
    {
        fn visit_pair(&mut self, key: Key<'kvs>, value: Value<'kvs>) -> Result<(), Error> {
            (**self).visit_pair(key, value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_is_object_safe() {
        fn _check(_: &Source) {}
    }

    #[test]
    fn visitor_is_object_safe() {
        fn _check(_: &Visitor) {}
    }
}
