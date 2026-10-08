use super::{check_size, next_key, read_row, DirectoryError, ReadOutcome, Text};
use serde::de::{DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::Deserializer;
use serde_json::value::RawValue;
use std::collections::HashSet;
use std::fmt;

pub(super) struct Results<'a>(pub(super) &'a mut ReadOutcome);

impl<'de> DeserializeSeed<'de> for Results<'_> {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, decoder: D) -> Result<(), D::Error> {
        decoder.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for Results<'_> {
    type Value = ();
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SPARQL bindings")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        let mut bindings = false;
        std::iter::from_fn(|| match next_key(&mut map, self.0) {
            Ok(Some(key)) => Some(results_field(&mut map, key, self.0, &mut bindings)),
            Ok(None) => None,
            Err(error) => Some(Err(error)),
        })
        .try_for_each(core::convert::identity)?;
        if !bindings {
            return Err(serde::de::Error::custom("bindings missing"));
        }
        Ok(())
    }
}

fn results_field<'de, A: MapAccess<'de>>(
    map: &mut A,
    key: Text,
    outcome: &mut ReadOutcome,
    bindings: &mut bool,
) -> Result<(), A::Error> {
    match key.0.as_str() {
        "bindings" if !*bindings => {
            map.next_value_seed(Rows(outcome))?;
            *bindings = true;
        }
        "bindings" => return Err(serde::de::Error::custom("duplicate bindings")),
        _ => {
            map.next_value::<IgnoredAny>()?;
        }
    }
    Ok(())
}

struct Rows<'a>(&'a mut ReadOutcome);

impl<'de> DeserializeSeed<'de> for Rows<'_> {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, decoder: D) -> Result<(), D::Error> {
        decoder.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for Rows<'_> {
    type Value = ();
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("bounded school bindings")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<(), A::Error> {
        let mut seen = HashSet::new();
        std::iter::from_fn(|| {
            let row = sequence.next_element::<&RawValue>();
            row.transpose()
        })
        .enumerate()
        .try_for_each(|(index, raw)| read_binding(raw, index.saturating_add(1), self.0, &mut seen))
    }
}

fn read_binding<E: serde::de::Error>(
    raw: Result<&RawValue, E>,
    line: usize,
    outcome: &mut ReadOutcome,
    seen: &mut HashSet<String>,
) -> Result<(), E> {
    let raw = raw.map_err(|_| DirectoryError::Representation {
        detail: "binding JSON is malformed".to_string(),
    });
    let result = check_size("wikidata rows", line, 20_000)
        .and_then(|()| read_row(raw?, line, outcome, seen));
    result.map_err(|error| {
        outcome.stop(line, error);
        E::custom("school bindings admission refused")
    })
}
