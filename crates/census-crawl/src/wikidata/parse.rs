use crate::directory::ReadOutcome;
use crate::{CrawlError, CrawlResult};
use census_domain::school_directory::DirectoryError;
use serde::Deserialize;
use serde_json::value::RawValue;
use std::collections::HashSet;

mod decode;
mod rows;
use rows::row_entry;

#[derive(Deserialize)]
struct SparqlBinding<'a> {
    #[serde(borrow)]
    item: Option<&'a RawValue>,
    #[serde(borrow, rename = "itemLabel")]
    item_label: Option<&'a RawValue>,
    #[serde(borrow)]
    website: Option<&'a RawValue>,
    #[serde(borrow)]
    state: Option<&'a RawValue>,
    #[serde(borrow)]
    city: Option<&'a RawValue>,
}

#[derive(Deserialize)]
struct WikidataValue<'a> {
    #[serde(borrow)]
    value: &'a RawValue,
}

pub fn parse_sparql(json: &str) -> CrawlResult<ReadOutcome> {
    if json.len() > 8 * 1024 * 1024 {
        return Err(CrawlError::Resource {
            resource: "wikidata body bytes",
            requested: json.len(),
            limit: 8 * 1024 * 1024,
        });
    }
    decode::read(json)
}

fn read_row(
    raw: &RawValue,
    line: usize,
    outcome: &mut ReadOutcome,
    seen: &mut HashSet<String>,
) -> Result<(), DirectoryError> {
    check_size("wikidata row bytes", raw.get().len(), 16 * 1024)?;
    let binding = match serde_json::from_str::<SparqlBinding<'_>>(raw.get()) {
        Ok(binding) => binding,
        Err(_) => {
            outcome.skip(line, "row", "binding is not a school field object")?;
            return Ok(());
        }
    };
    if let Some(entry) = row_entry(&binding, line, outcome, seen)? {
        outcome.push(entry)?;
    }
    Ok(())
}

fn value(raw: Option<&RawValue>) -> Result<Option<String>, DirectoryError> {
    raw.map(|raw| {
        let value = serde_json::from_str::<WikidataValue<'_>>(raw.get()).map_err(|_| {
            DirectoryError::UnsupportedValue {
                field: "wikidata field",
                value: "field must carry a string value".to_string(),
            }
        })?;
        decode::text(value.value)
    })
    .transpose()
}

fn check_size(
    resource: &'static str,
    requested: usize,
    limit: usize,
) -> Result<(), DirectoryError> {
    if requested > limit {
        return Err(DirectoryError::Capacity {
            resource,
            requested,
            limit,
        });
    }
    Ok(())
}

pub fn sparql_query() -> String {
    "SELECT ?item ?itemLabel ?website ?state ?city \
     WHERE { \
       ?item wdt:P31/wdt:P279* wd:Q3914; \
             wdt:P856 ?website. \
       OPTIONAL { ?item wdt:P131/wdt:P131* ?location. ?location wdt:P31 wd:Q3455524. ?location rdfs:label ?city. FILTER(lang(?city) = 'en') } \
       OPTIONAL { ?item wdt:P131/wdt:P131* ?stateLocation. ?stateLocation wdt:P31/wdt:P279* wd:Q3455524. ?stateLocation rdfs:label ?state. FILTER(lang(?state) = 'en') } \
       SERVICE wikibase:label { bd:serviceParam wikibase:language 'en'. } \
     } \
     LIMIT 10000".to_string()
}
