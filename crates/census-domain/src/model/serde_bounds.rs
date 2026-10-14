use serde::de::{DeserializeSeed, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;
use std::marker::PhantomData;

pub(in crate::model) fn sequence<'de, D, T, const MAX: usize>(
    deserializer: D,
) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    deserializer.deserialize_seq(BoundedSequence::<T, MAX>(PhantomData))
}

struct BoundedSequence<T, const MAX: usize>(PhantomData<T>);

impl<'de, T: Deserialize<'de>, const MAX: usize> Visitor<'de> for BoundedSequence<T, MAX> {
    type Value = Vec<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "at most {MAX} sequence entries")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        if sequence.size_hint().is_some_and(|size| size > MAX) {
            return Err(serde::de::Error::custom(
                "sequence exceeds its element ceiling",
            ));
        }
        let mut values = Vec::new();
        values
            .try_reserve_exact(MAX)
            .map_err(|_| serde::de::Error::custom("sequence allocation failed"))?;
        for _ in 0..MAX {
            match sequence.next_element()? {
                Some(value) => values.push(value),
                None => return Ok(values),
            }
        }
        if sequence.next_element_seed(ExcessElement)?.is_some() {
            return Err(serde::de::Error::custom(
                "sequence exceeds its element ceiling",
            ));
        }
        Ok(values)
    }
}

struct ExcessElement;

impl<'de> DeserializeSeed<'de> for ExcessElement {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, _deserializer: D) -> Result<Self::Value, D::Error> {
        Err(serde::de::Error::custom(
            "sequence exceeds its element ceiling",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    type TestResult = Result<(), Box<dyn std::error::Error>>;
    const MAX: usize = 3;

    thread_local! {
        static DECODED: Cell<usize> = const { Cell::new(0) };
    }

    #[derive(Debug, PartialEq, Eq)]
    struct Counted(u8);

    impl<'de> Deserialize<'de> for Counted {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            DECODED.with(|count| -> Result<(), D::Error> {
                let next = count
                    .get()
                    .checked_add(1)
                    .ok_or_else(|| serde::de::Error::custom("decode counter overflow"))?;
                count.set(next);
                Ok(())
            })?;
            u8::deserialize(deserializer).map(Self)
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    struct Bounded(Vec<Counted>);

    impl<'de> Deserialize<'de> for Bounded {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            sequence::<D, Counted, MAX>(deserializer).map(Self)
        }
    }

    #[test]
    fn exact_maximum_is_admitted_and_excess_elements_are_never_decoded() -> TestResult {
        DECODED.set(0);
        let admitted: Bounded = serde_json::from_str("[1,2,3]")?;
        check!(eq; admitted, Bounded(vec![Counted(1), Counted(2), Counted(3)]));
        check!(eq; DECODED.get(), MAX);
        for encoded in ["[1,2,3,4]", r#"[1,2,3,{"hostile":[[[[["not-a-byte"]]]]]}]"#] {
            DECODED.set(0);
            let error = serde_json::from_str::<Bounded>(encoded)
                .err()
                .ok_or("excess sequence element unexpectedly admitted")?;
            check!(eq; error.classify(), serde_json::error::Category::Data);
            check!(eq; DECODED.get(), MAX);
        }
        Ok(())
    }
}
