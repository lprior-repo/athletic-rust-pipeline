use super::{check_size, read_row, ReadOutcome};
use crate::{CrawlError, CrawlResult};
use census_domain::school_directory::DirectoryError;
use serde::de::{DeserializeSeed, IgnoredAny, MapAccess, Visitor};
use serde::Deserializer;
use serde_json::value::RawValue;
use std::fmt;

mod fields;
mod results;
pub(super) use fields::text;
use fields::{validate_head, Text, TextVisitor};
use results::Results;

pub(super) fn read(json: &str) -> CrawlResult<ReadOutcome> {
    let mut outcome = ReadOutcome::new();
    let mut decoder = serde_json::Deserializer::from_str(json);
    let result = Envelope(&mut outcome)
        .deserialize(&mut decoder)
        .and_then(|()| decoder.end());
    if let Err(source) = result {
        retain_failure(&mut outcome, source)?;
    }
    Ok(outcome)
}

fn retain_failure(outcome: &mut ReadOutcome, source: serde_json::Error) -> CrawlResult<()> {
    if no_rows(outcome) {
        return Err(match outcome.unfinished() {
            Some((_, error)) => error.clone().into(),
            None => envelope_error(source),
        });
    }
    outcome.stop(
        0,
        DirectoryError::Representation {
            detail: "SPARQL envelope or remaining bindings were refused".to_string(),
        },
    );
    Ok(())
}

fn no_rows(outcome: &ReadOutcome) -> bool {
    let counts = outcome.counts();
    counts.entries == 0 && counts.skipped == 0 && counts.notes == 0
}

fn envelope_error(source: serde_json::Error) -> CrawlError {
    if source.is_data() {
        CrawlError::Schema {
            url: "wikidata-sparql".to_string(),
            detail: "SPARQL envelope is missing or invalid".to_string(),
        }
    } else {
        CrawlError::Decode {
            url: "wikidata-sparql".to_string(),
            source,
        }
    }
}

struct Envelope<'a>(&'a mut ReadOutcome);

impl<'de> DeserializeSeed<'de> for Envelope<'_> {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, decoder: D) -> Result<(), D::Error> {
        decoder.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for Envelope<'_> {
    type Value = ();
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a SPARQL result envelope")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        let mut head = false;
        let mut results = false;
        std::iter::from_fn(|| match next_key(&mut map, self.0) {
            Ok(Some(key)) => Some(envelope_field(
                &mut map,
                key,
                self.0,
                &mut head,
                &mut results,
            )),
            Ok(None) => None,
            Err(error) => Some(Err(error)),
        })
        .try_for_each(core::convert::identity)?;
        if !head || !results {
            return Err(serde::de::Error::custom("SPARQL head or results missing"));
        }
        Ok(())
    }
}

fn next_key<'de, A: MapAccess<'de>>(
    map: &mut A,
    outcome: &mut ReadOutcome,
) -> Result<Option<Text>, A::Error> {
    let mut failure = None;
    let result = map.next_key_seed(TextVisitor(&mut failure));
    if let Some(error) = failure {
        outcome.stop(0, error);
    }
    result
}

fn envelope_field<'de, A: MapAccess<'de>>(
    map: &mut A,
    key: Text,
    outcome: &mut ReadOutcome,
    head: &mut bool,
    results: &mut bool,
) -> Result<(), A::Error> {
    match key.0.as_str() {
        "head" if !*head => {
            read_head(map, outcome)?;
            *head = true;
        }
        "results" if !*results => {
            map.next_value_seed(Results(outcome))?;
            *results = true;
        }
        "head" | "results" => {
            return Err(serde::de::Error::custom("duplicate SPARQL envelope field"))
        }
        _ => {
            map.next_value::<IgnoredAny>()?;
        }
    }
    Ok(())
}

fn read_head<'de, A: MapAccess<'de>>(
    map: &mut A,
    outcome: &mut ReadOutcome,
) -> Result<(), A::Error> {
    if let Err(error) = validate_head(map.next_value::<&RawValue>()?) {
        if error.is_resource() {
            outcome.stop(0, error);
        }
        return Err(serde::de::Error::custom("SPARQL head refused"));
    }
    Ok(())
}
