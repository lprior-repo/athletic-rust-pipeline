use crate::{domain::evidence::Sport, model::SourceRecord};
use anyhow::{bail, Result};
use serde::Serialize;
use std::collections::HashSet;
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

const MAX_QUERY_BYTES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct SearchQuery {
    pub query: String,
    pub sport: Sport,
    pub stage: u8,
}

impl<'de> serde::Deserialize<'de> for SearchQuery {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct RawQuery {
            query: String,
            sport: Sport,
            stage: u8,
        }
        let raw = RawQuery::deserialize(deserializer)?;
        Self::new(&raw.query, raw.sport, raw.stage).map_err(serde::de::Error::custom)
    }
}

impl SearchQuery {
    pub fn new(query: &str, sport: Sport, stage: u8) -> Result<Self> {
        let query = compact(query);
        validate_query(&query)?;
        Ok(Self {
            query,
            sport,
            stage,
        })
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.query
    }

    #[must_use]
    pub fn filter(&self) -> &'static str {
        match self.sport {
            Sport::TrackField => "a:tf",
            Sport::CrossCountry => "a:xc",
        }
    }

    #[must_use]
    pub fn cache_identity(&self) -> String {
        format!("{}\n{}", self.filter(), self.query.to_ascii_lowercase())
    }

    #[must_use]
    pub fn sport(&self) -> Sport {
        self.sport
    }
    #[must_use]
    pub fn stage(&self) -> u8 {
        self.stage
    }
}

pub fn query_plan(record: &SourceRecord) -> Result<Vec<SearchQuery>> {
    let (first, last) = source_name(record)?;
    build_queries(query_variants(record, &first, &last))
}

fn source_name(record: &SourceRecord) -> Result<(String, String)> {
    let first = field(record, &["Person First", "first_name"])
        .map(str::to_owned)
        .ok_or_else(|| {
            anyhow::anyhow!("no search query: source record has an incomplete athlete name")
        })?;
    let last = field(record, &["Person Last", "last_name"])
        .map(str::to_owned)
        .ok_or_else(|| {
            anyhow::anyhow!("no search query: source record has an incomplete athlete name")
        })?;
    if compact(&format!("{first} {last}")).is_empty() {
        bail!("no search query: source record has a blank athlete name")
    }
    Ok((first, last))
}

fn query_variants(record: &SourceRecord, first: &str, last: &str) -> Vec<(u8, String)> {
    let name = compact(&format!("{first} {last}"));
    let context = contexts(record, &name);
    let normalized = normalize_name(&name);
    let reversed = compact(&format!("{last} {first}"));
    let initial = first
        .chars()
        .next()
        .map(|value| compact(&format!("{value} {last}")));
    std::iter::once((0, name))
        .chain(context.into_iter().map(|value| (1, value)))
        .chain(
            [normalized, reversed]
                .into_iter()
                .chain(initial)
                .map(|value| (2, value)),
        )
        .collect()
}

fn build_queries(candidates: Vec<(u8, String)>) -> Result<Vec<SearchQuery>> {
    candidates
        .into_iter()
        .try_fold(
            (Vec::new(), HashSet::new()),
            |(mut queries, mut seen), (stage, query)| {
                [Sport::TrackField, Sport::CrossCountry]
                    .into_iter()
                    .try_for_each(|sport| {
                        let request = SearchQuery::new(&query, sport, stage)?;
                        if seen.insert(request.cache_identity()) {
                            queries.push(request);
                        }
                        Ok::<(), anyhow::Error>(())
                    })?;
                Ok::<_, anyhow::Error>((queries, seen))
            },
        )
        .map(|(queries, _)| queries)
}

fn contexts(record: &SourceRecord, name: &str) -> Vec<String> {
    let school = field(record, &["Schools Name", "school"]);
    let city = field(record, &["Address Mailing / Permanent City", "city"]);
    let region = field(record, &["Address Mailing / Permanent Region", "state"]);
    [
        school.map(|value| compact(&format!("{name} {value}"))),
        match (city, region) {
            (Some(city), Some(region)) => Some(compact(&format!("{name} {city} {region}"))),
            (Some(city), None) => Some(compact(&format!("{name} {city}"))),
            (None, Some(region)) => Some(compact(&format!("{name} {region}"))),
            (None, None) => None,
        },
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn field<'a>(record: &'a SourceRecord, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| record.fields.get(*key))
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn compact(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_name(value: &str) -> String {
    value
        .nfkd()
        .filter(|character| !is_combining_mark(*character))
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn validate_query(query: &str) -> Result<()> {
    if query.is_empty() {
        bail!("search query must be nonempty")
    }
    if query.len() > MAX_QUERY_BYTES {
        bail!("search query exceeds {MAX_QUERY_BYTES} bytes")
    }
    if query.chars().any(char::is_control) {
        bail!("search query contains a control character")
    }
    Ok(())
}
