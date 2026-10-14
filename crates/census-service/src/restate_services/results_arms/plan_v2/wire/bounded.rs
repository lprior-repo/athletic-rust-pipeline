use serde::de::{DeserializeSeed, Error, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::{fmt, marker::PhantomData};

pub(in crate::restate_services) struct Sequence<S> {
    pub(in crate::restate_services) seed: S,
    pub(in crate::restate_services) limit: usize,
}

impl<'de, S> Visitor<'de> for Sequence<S>
where
    S: DeserializeSeed<'de> + Copy,
{
    type Value = Vec<S::Value>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a bounded sequence")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut output = Vec::new();
        for _ in 0..self.limit {
            let Some(value) = sequence.next_element_seed(self.seed)? else {
                return Ok(output);
            };
            grow(&mut output, self.limit)?;
            output.push(value);
        }
        match sequence.next_element_seed(Reject)? {
            None => Ok(output),
            Some(()) => Err(A::Error::custom("sequence limit")),
        }
    }
}

fn grow<T, E: Error>(output: &mut Vec<T>, limit: usize) -> Result<(), E> {
    if output.len() < output.capacity() {
        return Ok(());
    }
    let remaining = limit
        .checked_sub(output.len())
        .ok_or_else(|| E::custom("sequence limit"))?;
    let additional = output.capacity().max(4).min(remaining);
    output
        .try_reserve_exact(additional)
        .map_err(|_| E::custom("sequence allocation"))
}

#[derive(Clone, Copy)]
struct Reject;

impl<'de> DeserializeSeed<'de> for Reject {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, _: D) -> Result<(), D::Error> {
        Err(D::Error::custom("sequence limit"))
    }
}

struct Value<T>(PhantomData<T>);

impl<T> Copy for Value<T> {}
impl<T> Clone for Value<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'de, T: Deserialize<'de>> DeserializeSeed<'de> for Value<T> {
    type Value = T;

    fn deserialize<D: Deserializer<'de>>(self, decoder: D) -> Result<T, D::Error> {
        T::deserialize(decoder)
    }
}

pub(in super::super) fn rows<'de, D, T>(decoder: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    decoder.deserialize_seq(Sequence {
        seed: Value(PhantomData),
        limit: 65_536,
    })
}

pub(in super::super) fn pins<'de, D, T>(decoder: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    decoder.deserialize_seq(Sequence {
        seed: Value(PhantomData),
        limit: 12,
    })
}

pub(in super::super) fn texts<'de, D: Deserializer<'de>>(
    decoder: D,
) -> Result<Vec<String>, D::Error> {
    decoder.deserialize_seq(Sequence {
        seed: super::text::Text,
        limit: 65_536,
    })
}

pub(in super::super) fn sources<'de, D: Deserializer<'de>>(
    decoder: D,
) -> Result<Vec<String>, D::Error> {
    decoder.deserialize_seq(Sequence {
        seed: super::text::Text,
        limit: 64,
    })
}

pub(in crate::restate_services) fn quarantines<'de, D: Deserializer<'de>>(
    decoder: D,
) -> Result<Vec<super::Quarantine>, D::Error> {
    decoder.deserialize_seq(Sequence {
        seed: Value(PhantomData),
        limit: 25,
    })
}
