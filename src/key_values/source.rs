//! Sources of structured key-value pairs.

use std::fmt;
use std::borrow::Borrow;
use std::marker::PhantomData;

use super::value;

use super::private::{value, value_owned};

#[doc(inline)]
pub use super::private::{Key, ToValue, Value, ValueOwned};

#[doc(inline)]
pub use super::Error;

use std::borrow::ToOwned;

// TODO: Would it be safe to remove the `ToOwned` bounds in `no_std`?
// TODO: We might need to make some methods private so it can't be
// implemented
impl<T> ToValue for T
where
    T: value::Value + Send + Sync + ToOwned,
    T::Owned: value::Value + Send + Sync + 'static,
{
    fn to_value(&self) -> Value {
        value(self)
    }

    fn to_owned(&self) -> ValueOwned {
        value_owned(self.to_owned())
    }
}

impl<'a> ToValue for &'a dyn ToValue {
    fn to_value(&self) -> Value {
        (**self).to_value()
    }

    fn to_owned(&self) -> ValueOwned {
        (**self).to_owned()
    }
}

fn ensure_to_value<'a>() {
    fn is_to_value<T: ToValue + ?Sized>() {}

    is_to_value::<&'a dyn ToValue>();
    is_to_value::<&'a str>();
}

/// A source for key value pairs that can be serialized.
pub trait Source {
    /// Serialize the key value pairs.
    fn visit<'kvs>(&'kvs self, visitor: &mut dyn Visitor<'kvs>) -> Result<(), Error>;

    /// Erase this `Source` so it can be used without
    /// requiring generic type parameters.
    fn erase(&self) -> ErasedSource
    where
        Self: Sized,
    {
        ErasedSource::erased(self)
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

        let mut visitor = Get(Key::new(&key), None);
        let _ = self.visit(&mut visitor);

        visitor.1
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

    /// Apply a function to each key-value pair.
    fn try_for_each<F, E>(self, f: F) -> Result<(), Error>
    where
        Self: Sized,
        F: FnMut(Key, Value) -> Result<(), E>,
        E: Into<Error>,
    {
        struct ForEach<F, E>(F, PhantomData<E>);

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
    #[cfg(feature = "structured_serde")]
    fn serialize_as_map(self) -> SerializeAsMap<Self>
    where
        Self: Sized,
    {
        SerializeAsMap(self)
    }

    /// Serialize the key-value pairs as a sequence.
    #[cfg(feature = "structured_serde")]
    fn serialize_as_seq(self) -> SerializeAsSeq<Self>
    where
        Self: Sized,
    {
        SerializeAsSeq(self)
    }
}

impl<'a, T: ?Sized> Source for &'a T
where
    T: Source,
{
    fn visit<'kvs>(&'kvs self, visitor: &mut dyn Visitor<'kvs>) -> Result<(), Error> {
        (*self).visit(visitor)
    }

    fn get<'kvs, Q>(&'kvs self, key: Q) -> Option<Value<'kvs>>
    where
        Q: Borrow<str>,
    {
        (*self).get(key)
    }
}

/// A visitor for key value pairs.
/// 
/// The lifetime of the keys and values is captured by the `'kvs` type.
pub trait Visitor<'kvs> {
    /// Visit a key value pair.
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

/// A chain of two `Source`s.
#[derive(Debug)]
pub struct Chained<A, B>(A, B);

impl<A, B> Source for Chained<A, B>
where
    A: Source,
    B: Source,
{
    fn visit<'kvs>(&'kvs self, visitor: &mut dyn Visitor<'kvs>) -> Result<(), Error> {
        self.0.visit(visitor)?;
        self.1.visit(visitor)?;

        Ok(())
    }
}

/// Serialize the key-value pairs as a map.
#[derive(Debug)]
#[cfg(feature = "structured_serde")]
pub struct SerializeAsMap<KVS>(KVS);

/// Serialize the key-value pairs as a sequence.
#[derive(Debug)]
#[cfg(feature = "structured_serde")]
pub struct SerializeAsSeq<KVS>(KVS);

impl<K, V> Source for (K, V)
where
    K: Borrow<str>,
    V: ToValue,
{
    fn visit<'kvs>(&'kvs self, visitor: &mut dyn Visitor<'kvs>) -> Result<(), Error>
    {
        visitor.visit_pair(Key::new(&self.0), self.1.to_value())
    }
}

impl<KVS> Source for [KVS] where KVS: Source {
    fn visit<'kvs>(&'kvs self, visitor: &mut dyn Visitor<'kvs>) -> Result<(), Error> {
        for kv in self {
            kv.visit(visitor)?;
        }

        Ok(())
    }
}

/// A key value source on a `Record`.
#[derive(Clone)]
pub struct ErasedSource<'a>(&'a dyn ErasedSourceBridge);

impl<'a> ErasedSource<'a> {
    pub fn erased(kvs: &'a impl Source) -> Self {
        ErasedSource(kvs)
    }

    pub fn empty() -> Self {
        ErasedSource(&(&[] as &[(&str, ValueOwned)]))
    }
}

impl<'a> fmt::Debug for ErasedSource<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Source").finish()
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
        self.0.erased_get(key.borrow())
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

#[cfg(feature = "structured_serde")]
mod serde_support {
    use super::*;

    use serde::ser::{Serialize, Serializer, SerializeMap, SerializeSeq};

    impl<KVS> Serialize for SerializeAsMap<KVS>
    where
        KVS: Source,
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut map = serializer.serialize_map(None)?;

            self.0
                .by_ref()
                .try_for_each(|k, v| map.serialize_entry(&k, &v))
                .map_err(Error::into_serde)?;

            map.end()
        }
    }

    impl<KVS> Serialize for SerializeAsSeq<KVS>
    where
        KVS: Source,
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut seq = serializer.serialize_seq(None)?;

            self.0
                .by_ref()
                .try_for_each(|k, v| seq.serialize_element(&(&k, &v)))
                .map_err(Error::into_serde)?;

            seq.end()
        }
    }
}

#[cfg(feature = "structured_serde")]
pub use self::serde_support::*;

#[cfg(feature = "std")]
mod std_support {
    use super::*;

    use std::hash::Hash;
    use std::collections::{HashMap, BTreeMap};

    impl<KVS> Source for Vec<KVS> where KVS: Source {
        fn visit<'kvs>(&'kvs self, visitor: &mut dyn Visitor<'kvs>) -> Result<(), Error> {
            self.as_slice().visit(visitor)
        }
    }

    impl<K, V> Source for BTreeMap<K, V>
    where
        K: Borrow<str> + Ord,
        V: ToValue,
    {
        fn visit<'kvs>(&'kvs self, visitor: &mut dyn Visitor<'kvs>) -> Result<(), Error>
        {
            for (k, v) in self {
                visitor.visit_pair(Key::new(k), v.to_value())?;
            }

            Ok(())
        }

        fn get<'kvs, Q>(&'kvs self, key: Q) -> Option<Value<'kvs>>
        where
            Q: Borrow<str>,
        {
            BTreeMap::get(self, key.borrow()).map(|v| v.to_value())
        }
    }

    impl<K, V> Source for HashMap<K, V>
    where
        K: Borrow<str> + Eq + Hash,
        V: ToValue,
    {
        fn visit<'kvs>(&'kvs self, visitor: &mut dyn Visitor<'kvs>) -> Result<(), Error>
        {
            for (k, v) in self {
                visitor.visit_pair(Key::new(k), v.to_value())?;
            }

            Ok(())
        }

        fn get<'kvs, Q>(&'kvs self, key: Q) -> Option<Value<'kvs>>
        where
            Q: Borrow<str>,
        {
            HashMap::get(self, key.borrow()).map(|v| v.to_value())
        }
    }
}

#[cfg(feature = "std")]
pub use self::std_support::*;
