use serde::de::{DeserializeSeed, Error, Visitor};
use serde::Deserializer;
use std::fmt;

#[derive(Clone, Copy)]
pub(in crate::restate_services) struct Text;

impl<'de> DeserializeSeed<'de> for Text {
    type Value = String;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<String, D::Error> {
        deserializer.deserialize_str(self)
    }
}

impl<'de> Visitor<'de> for Text {
    type Value = String;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a string of at most 4096 bytes")
    }

    fn visit_str<E: Error>(self, value: &str) -> Result<String, E> {
        if value.len() > super::super::MAX_TEXT_BYTES {
            return Err(E::custom("text exceeds 4096 bytes"));
        }
        let mut output = String::new();
        output
            .try_reserve_exact(value.len())
            .map_err(|_| E::custom("text allocation"))?;
        output.push_str(value);
        Ok(output)
    }
}

pub(in super::super) fn read<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<String, D::Error> {
    Text.deserialize(deserializer)
}

pub(in crate::restate_services) fn optional<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    struct Optional;
    impl<'de> Visitor<'de> for Optional {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a bounded string or null")
        }

        fn visit_none<E: Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_some<D: Deserializer<'de>>(
            self,
            deserializer: D,
        ) -> Result<Self::Value, D::Error> {
            Text.deserialize(deserializer).map(Some)
        }
    }
    deserializer.deserialize_option(Optional)
}
