use super::super::{bound, invalid};
use census_store::{StoreError, StoreResult};
use html5gum::{DefaultEmitter, Token, Tokenizer};
use serde::Deserialize;

pub(super) struct MapOnly<T>(pub(super) T);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for MapOnly<T> {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct MapVisitor<T>(std::marker::PhantomData<T>);
        impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for MapVisitor<T> {
            type Value = MapOnly<T>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("an object-shaped provider target declaration")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                map: M,
            ) -> Result<Self::Value, M::Error> {
                T::deserialize(serde::de::value::MapAccessDeserializer::new(map)).map(MapOnly)
            }
        }
        decoder.deserialize_map(MapVisitor(std::marker::PhantomData))
    }
}

pub(super) fn script<T: serde::de::DeserializeOwned>(html: &str) -> StoreResult<T> {
    let mut emitter = DefaultEmitter::default();
    emitter.naively_switch_states(true);
    let body = Tokenizer::new_with_emitter(html, emitter)
        .take(4097)
        .enumerate()
        .try_fold(
            String::new(),
            |mut body, (index, token)| -> StoreResult<String> {
                bound(
                    index.checked_add(1).ok_or(StoreError::CounterOverflow)?,
                    4096,
                    "provider target JSON token count",
                )?;
                if let Token::String(value) =
                    token.map_err(|_| invalid("provider target JSON token failure"))?
                {
                    let text = std::str::from_utf8(&value)
                        .map_err(|_| invalid("provider target JSON encoding invalid"))?;
                    bound(
                        body.len()
                            .checked_add(text.len())
                            .ok_or(StoreError::CounterOverflow)?,
                        64 * 1024,
                        "provider target JSON bytes",
                    )?;
                    body.try_reserve(text.len())
                        .map_err(|_| invalid("provider target JSON allocation refused"))?;
                    body.push_str(text);
                }
                Ok(body)
            },
        )?;
    let MapOnly(value): MapOnly<T> = serde_json::from_str(&body).map_err(super::super::json)?;
    Ok(value)
}
