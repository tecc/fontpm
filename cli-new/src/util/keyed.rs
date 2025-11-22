use serde::{de, ser};
use std::{fmt, marker};

pub struct AsKeyedValues<T>(marker::PhantomData<T>);
impl<K, V> AsKeyedValues<(K, V)> {
    pub fn serialize<'a, S, T>(
        value: &'a T,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
        &'a T: IntoIterator,
        <&'a T as IntoIterator>::Item: ValuePair<V>,
        V: 'a + Keyed<K> + ser::Serialize,
    {
        let iter = value.into_iter();

        let mut seq = serializer.serialize_seq(iter.size_hint().1)?;
        for value in iter {
            ser::SerializeSeq::serialize_element(&mut seq, value.value())?;
        }
        ser::SerializeSeq::end(seq)
    }

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: de::Deserializer<'de>,
        T: FromIterator<(K, V)>,
        V: Keyed<K> + de::Deserialize<'de>,
    {
        struct KeyedMapVisitor<T, K, V>(marker::PhantomData<(T, K, V)>);

        impl<'de, T, K, V> de::Visitor<'de> for KeyedMapVisitor<T, K, V>
        where
            T: FromIterator<(K, V)>,
            V: Keyed<K> + de::Deserialize<'de>,
        {
            type Value = T;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a list")
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                struct Iter<'de, K, V, A>(
                    A,
                    marker::PhantomData<(K, V, &'de ())>,
                );
                impl<'de, K, V, A> Iterator for Iter<'de, K, V, A>
                where
                    A: de::SeqAccess<'de>,
                    V: Keyed<K> + de::Deserialize<'de>,
                {
                    type Item = Result<(K, V), A::Error>;
                    fn next(&mut self) -> Option<Self::Item> {
                        de::SeqAccess::next_element(&mut self.0)
                            .transpose()
                            .map(|a| a.map(|value: V| (value.key(), value)))
                    }
                    fn size_hint(&self) -> (usize, Option<usize>) {
                        de::SeqAccess::size_hint(&self.0)
                            .map(|x| (x, Some(x)))
                            .unwrap_or((0, None))
                    }
                }
                Iter(seq, marker::PhantomData).collect::<Result<T, A::Error>>()
            }
        }

        deserializer.deserialize_seq(KeyedMapVisitor(marker::PhantomData))
    }
}

pub trait ValuePair<V> {
    fn value(&self) -> &V;
}
impl<K, V> ValuePair<V> for (K, V) {
    fn value(&self) -> &V {
        &self.1
    }
}
impl<'a, K, V> ValuePair<V> for dashmap::mapref::multiple::RefMulti<'a, K, V>
where
    K: std::hash::Hash + Eq,
{
    fn value(&self) -> &V {
        dashmap::mapref::multiple::RefMulti::value(&self)
    }
}

pub trait Keyed<K> {
    fn key(&self) -> K;
}
impl<T, K> Keyed<K> for &T
where
    T: Keyed<K>,
{
    fn key(&self) -> K {
        T::key(self)
    }
}
impl<T, K> Keyed<K> for Box<T>
where
    T: Keyed<K>,
{
    fn key(&self) -> K {
        T::key(self.as_ref())
    }
}
impl<T, K> Keyed<K> for std::rc::Rc<T>
where
    T: Keyed<K>,
{
    fn key(&self) -> K {
        T::key(self.as_ref())
    }
}
impl<T, K> Keyed<K> for std::sync::Arc<T>
where
    T: Keyed<K>,
{
    fn key(&self) -> K {
        T::key(self.as_ref())
    }
}
