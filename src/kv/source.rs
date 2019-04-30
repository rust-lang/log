//! Sources for key-value pairs.

use kv::{Error, Key, ToKey, Value, ToValue};

/// A source of key-value pairs.
/// 
/// The source may be a single pair, a set of pairs, or a filter over a set of pairs.
/// Use the [`Visitor`](trait.Visitor.html) trait to inspect the structured data
/// in a source.
pub trait Source {
    /// Visit key-value pairs.
    /// 
    /// A source doesn't have to guarantee any ordering or uniqueness of key-value pairs.
    /// If the given visitor returns an error then the source may early-return with it,
    /// even if there are more key-value pairs.
    /// 
    /// # Implementation notes
    /// 
    /// A source should yield the same key-value pairs to a subsequent visitor unless
    /// that visitor itself fails.
    fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error>;

    /// Count the number of key-value pairs that can be visited.
    /// 
    /// # Implementation notes
    /// 
    /// A source that knows the number of key-value pairs upfront may provide a more
    /// efficient implementation.
    /// 
    /// A subsequent call to `visit` should yield the same number of key-value pairs
    /// to the visitor, unless that visitor fails part way through.
    fn count(&self) -> usize {
        struct Count(usize);

        impl<'kvs> Visitor<'kvs> for Count {
            fn visit_pair(&mut self, _: Key<'kvs>, _: Value<'kvs>) -> Result<(), Error> {
                self.0 += 1;

                Ok(())
            }
        }

        let mut count = Count(0);
        let _ = self.visit(&mut count);
        count.0
    }
}

impl<'a, T> Source for &'a T
where
    T: Source + ?Sized,
{
    fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error> {
        (**self).visit(visitor)
    }

    fn count(&self) -> usize {
        (**self).count()
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

    fn count(&self) -> usize {
        1
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

    fn count(&self) -> usize {
        self.len()
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

    fn count(&self) -> usize {
        self.as_ref().map(Source::count).unwrap_or(0)
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

        fn count(&self) -> usize {
            (**self).count()
        }
    }

    impl<S> Source for Vec<S>
    where
        S: Source,
    {
        fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error> {
            (**self).visit(visitor)
        }

        fn count(&self) -> usize {
            (**self).count()
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

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn count() {
            assert_eq!(1, Source::count(&Box::new(("a", 1))));
            assert_eq!(2, Source::count(&vec![("a", 1), ("b", 2)]));
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

    #[test]
    fn count() {
        struct OnePair {
            key: &'static str,
            value: i32,
        }

        impl Source for OnePair {
            fn visit<'kvs>(&'kvs self, visitor: &mut Visitor<'kvs>) -> Result<(), Error> {
                visitor.visit_pair(self.key.to_key(), self.value.to_value())
            }
        }

        assert_eq!(1, Source::count(&("a", 1)));
        assert_eq!(2, Source::count(&[("a", 1), ("b", 2)] as &[_]));
        assert_eq!(0, Source::count(&Option::None::<(&str, i32)>));
        assert_eq!(1, Source::count(&OnePair { key: "a", value: 1 }));
    }
}
