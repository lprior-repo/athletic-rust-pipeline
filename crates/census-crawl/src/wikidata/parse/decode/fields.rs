use super::{check_size, DirectoryError};
use serde::de::{DeserializeSeed, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::value::RawValue;
use std::fmt;

pub(super) struct Text(pub(super) String);

pub(super) struct TextVisitor<'a>(pub(super) &'a mut Option<DirectoryError>);

impl<'de> DeserializeSeed<'de> for TextVisitor<'_> {
    type Value = Text;
    fn deserialize<D: Deserializer<'de>>(self, decoder: D) -> Result<Text, D::Error> {
        decoder.deserialize_str(self)
    }
}

impl Visitor<'_> for TextVisitor<'_> {
    type Value = Text;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a string of at most 1024 bytes")
    }
    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Text, E> {
        owned_text(value).map_err(|error| {
            *self.0 = Some(error);
            E::custom("wikidata field admission refused")
        })
    }
}

fn owned_text(value: &str) -> Result<Text, DirectoryError> {
    check_size("wikidata field bytes", value.len(), 1024)?;
    let mut text = String::new();
    text.try_reserve_exact(value.len())
        .map_err(|_| DirectoryError::Allocation {
            resource: "wikidata field",
        })?;
    text.push_str(value);
    Ok(Text(text))
}

pub(crate) fn text(raw: &RawValue) -> Result<String, DirectoryError> {
    let mut failure = None;
    let mut decoder = serde_json::Deserializer::from_str(raw.get());
    let decoded = TextVisitor(&mut failure).deserialize(&mut decoder);
    if let Some(error) = failure {
        return Err(error);
    }
    decoded
        .map(|text| text.0)
        .map_err(|_| DirectoryError::UnsupportedValue {
            field: "wikidata field",
            value: "field value is not a string".to_string(),
        })
}

#[derive(Deserialize)]
struct Head<'a> {
    #[serde(borrow)]
    vars: &'a RawValue,
}

pub(super) fn validate_head(raw: &RawValue) -> Result<(), DirectoryError> {
    let invalid = || DirectoryError::UnsupportedValue {
        field: "SPARQL head",
        value: "missing or invalid required variables".to_string(),
    };
    let head: Head<'_> = serde_json::from_str(raw.get()).map_err(|_| invalid())?;
    let mut decoder = serde_json::Deserializer::from_str(head.vars.get());
    let mut failure = None;
    let mask = Variables(&mut failure).deserialize(&mut decoder);
    if let Some(error) = failure {
        return Err(error);
    }
    let mask = mask.map_err(|_| invalid())?;
    decoder.end().map_err(|_| invalid())?;
    if mask != 7 {
        return Err(invalid());
    }
    Ok(())
}

struct Variables<'a>(&'a mut Option<DirectoryError>);

impl<'de> DeserializeSeed<'de> for Variables<'_> {
    type Value = u8;
    fn deserialize<D: Deserializer<'de>>(self, decoder: D) -> Result<u8, D::Error> {
        decoder.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for Variables<'_> {
    type Value = u8;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SPARQL variable names")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<u8, A::Error> {
        let mut field_failure = None;
        let result = std::iter::from_fn(|| {
            sequence
                .next_element_seed(TextVisitor(&mut field_failure))
                .transpose()
        })
        .enumerate()
        .try_fold(0, |mask, (index, var)| {
            if let Err(error) = check_size("SPARQL variables", index.saturating_add(1), 128) {
                *self.0 = Some(error);
                return Err(serde::de::Error::custom(
                    "SPARQL variables admission refused",
                ));
            }
            Ok(mask | variable_bit(&var?.0))
        });
        if field_failure.is_some() {
            *self.0 = field_failure;
        }
        result
    }
}

fn variable_bit(name: &str) -> u8 {
    match name {
        "item" => 1,
        "itemLabel" => 2,
        "website" => 4,
        _ => 0,
    }
}
