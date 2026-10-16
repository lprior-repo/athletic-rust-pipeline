mod text_length;

use crate::public_identity::paged_generation::{admission, ObjectRefV2};
use census_domain::model::StoreIdentity;
use serde::{
    de::{Error, Visitor},
    Deserialize, Deserializer,
};
use serde_json::value::RawValue;

pub(super) fn declaration<'de, D: Deserializer<'de>>(parser: D) -> Result<String, D::Error> {
    string(parser, 65_536)
}
pub(super) fn head<'de, D: Deserializer<'de>>(parser: D) -> Result<String, D::Error> {
    string(parser, 1024)
}
pub(super) fn history<'de, D: Deserializer<'de>>(parser: D) -> Result<String, D::Error> {
    string(parser, crate::MAX_PUBLIC_IDENTITY_HISTORY_BYTES)
}
pub(super) fn witness<'de, D: Deserializer<'de>>(parser: D) -> Result<String, D::Error> {
    string(
        parser,
        crate::public_identity::local_adjudication::MAX_WITNESS_BYTES,
    )
}
pub(super) fn receipt<'de, D: Deserializer<'de>>(parser: D) -> Result<String, D::Error> {
    string(parser, 65_536)
}
pub(super) fn key<'de, D: Deserializer<'de>>(parser: D) -> Result<String, D::Error> {
    base64(parser, 1024)
}
pub(super) fn value<'de, D: Deserializer<'de>>(parser: D) -> Result<String, D::Error> {
    base64(parser, crate::MAX_JOURNAL_VALUE_BYTES)
}

pub(super) fn decision<'de, D: Deserializer<'de>>(parser: D) -> Result<Option<String>, D::Error> {
    let raw = <&RawValue>::deserialize(parser)?;
    if raw.get() == "null" {
        return Ok(None);
    }
    text(raw, 65_536, 6).map(Some)
}
pub(super) fn identity<'de, D: Deserializer<'de>>(parser: D) -> Result<StoreIdentity, D::Error> {
    let raw = <&RawValue>::deserialize(parser)?;
    bounded_raw::<D::Error>(raw, 1024)?;
    serde_json::from_str(raw.get()).map_err(D::Error::custom)
}
pub(super) fn object<'de, D: Deserializer<'de>>(parser: D) -> Result<ObjectRefV2, D::Error> {
    let raw = <&RawValue>::deserialize(parser)?;
    bounded_raw::<D::Error>(raw, 65_536)?;
    let mut parser = serde_json::Deserializer::from_str(raw.get());
    let object = admission::decode_object(&mut parser).map_err(D::Error::custom)?;
    parser.end().map_err(D::Error::custom)?;
    Ok(object)
}
fn string<'de, D: Deserializer<'de>>(parser: D, maximum: usize) -> Result<String, D::Error> {
    text(<&RawValue>::deserialize(parser)?, maximum, 6)
}
fn base64<'de, D: Deserializer<'de>>(parser: D, binary: usize) -> Result<String, D::Error> {
    let maximum = binary
        .checked_add(2)
        .and_then(|n| n.checked_div(3))
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| D::Error::custom("frozen string bound overflow"))?;
    text(<&RawValue>::deserialize(parser)?, maximum, 1)
}
fn text<E: Error>(raw: &RawValue, maximum: usize, expansion: usize) -> Result<String, E> {
    let encoded = maximum
        .checked_mul(expansion)
        .and_then(|n| n.checked_add(2))
        .ok_or_else(|| E::custom("frozen string bound overflow"))?;
    bounded_raw::<E>(raw, encoded)?;
    text_length::require::<E>(raw.get(), maximum)?;
    let mut parser = serde_json::Deserializer::from_str(raw.get());
    let value = parser
        .deserialize_str(BoundedText(maximum))
        .map_err(E::custom)?;
    parser.end().map_err(E::custom)?;
    Ok(value)
}
fn bounded_raw<E: Error>(raw: &RawValue, maximum: usize) -> Result<(), E> {
    if raw.get().len() > maximum {
        return Err(E::custom("frozen encoded field ceiling"));
    }
    Ok(())
}
struct BoundedText(usize);
impl Visitor<'_> for BoundedText {
    type Value = String;
    fn expecting(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str("frozen text within its field byte ceiling")
    }
    fn visit_str<E: Error>(self, value: &str) -> Result<String, E> {
        if value.len() > self.0 {
            return Err(E::custom("frozen decoded field ceiling"));
        }
        let mut owned = String::new();
        owned.try_reserve_exact(value.len()).map_err(E::custom)?;
        owned.push_str(value);
        Ok(owned)
    }
}

#[cfg(test)]
mod tests;
