use serde::de::{Error, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;

pub(super) struct Text<const MAX: usize>(pub(super) String);

impl<'de, const MAX: usize> Deserialize<'de> for Text<MAX> {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct Read<const MAX: usize>;
        impl<const MAX: usize> Visitor<'_> for Read<MAX> {
            type Value = Text<MAX>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("bounded text")
            }
            fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
                if value.len() > MAX {
                    return Err(E::custom("MHSAA PDF text bound exceeded"));
                }
                let mut text = String::new();
                text.try_reserve_exact(value.len()).map_err(E::custom)?;
                text.push_str(value);
                Ok(Text(text))
            }
        }
        decoder.deserialize_str(Read::<MAX>)
    }
}

pub(super) fn headers<'de, D: Deserializer<'de>>(
    decoder: D,
) -> Result<Vec<(String, String)>, D::Error> {
    struct Read;
    impl<'de> Visitor<'de> for Read {
        type Value = Vec<(String, String)>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("at most32 Raw request headers")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut source: A) -> Result<Self::Value, A::Error> {
            let mut held = Vec::new();
            while let Some((name, value)) = source.next_element::<(Text<128>, Text<2048>)>()? {
                if held.len() == 32 {
                    return Err(A::Error::custom("MHSAA PDF request header bound exceeded"));
                }
                held.try_reserve_exact(1).map_err(A::Error::custom)?;
                held.push((name.0, value.0));
            }
            Ok(held)
        }
    }
    decoder.deserialize_seq(Read)
}

pub(super) fn bytes<'de, D: Deserializer<'de>>(decoder: D) -> Result<Vec<u8>, D::Error> {
    struct Read;
    impl<'de> Visitor<'de> for Read {
        type Value = Vec<u8>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("bounded Raw header dump bytes")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut source: A) -> Result<Self::Value, A::Error> {
            let mut held = Vec::new();
            while let Some(byte) = source.next_element::<u8>()? {
                if held.len() == 16 * 1024 {
                    return Err(A::Error::custom("MHSAA PDF Raw header dump bound exceeded"));
                }
                held.try_reserve_exact(1).map_err(A::Error::custom)?;
                held.push(byte);
            }
            Ok(held)
        }
    }
    decoder.deserialize_seq(Read)
}
