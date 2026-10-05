use crate::directory::{census_state, field, skip_failed, skip_row, ReadOutcome};
use crate::CrawlResult;
use census_domain::school_directory::{
    CityName, SchoolDirectoryEntry, SchoolName, SourceLabel, Website,
};
use census_domain::UsJurisdiction;
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Deserialize)]
struct SparqlResponse {
    head: SparqlHead,
    results: SparqlResults,
}

#[derive(Deserialize)]
struct SparqlHead {
    vars: Vec<String>,
}

#[derive(Deserialize)]
struct SparqlResults {
    bindings: Vec<SparqlBinding>,
}

#[derive(Deserialize)]
struct SparqlBinding {
    #[serde(rename = "item")]
    item: Option<WikidataValue>,
    #[serde(rename = "itemLabel")]
    item_label: Option<WikidataValue>,
    website: Option<WikidataValue>,
    state: Option<WikidataValue>,
    city: Option<WikidataValue>,
}

#[derive(Deserialize)]
struct WikidataValue {
    value: String,
}

fn binding_value(binding: &Option<WikidataValue>) -> Option<&str> {
    binding.as_ref().map(|v| v.value.as_str())
}

fn qid_from_url(url: &str) -> Option<&str> {
    let prefix = "http://www.wikidata.org/entity/";
    url.strip_prefix(prefix).or_else(|| {
        let prefix_alt = "http://wikidata.org/entity/";
        url.strip_prefix(prefix_alt)
    })
}

fn is_valid_qid(qid: &str) -> bool {
    if !qid.starts_with('Q') {
        return false;
    }
    let Some(rest) = qid.get(1..) else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }
    rest.chars().all(|c| c.is_ascii_digit())
}

pub fn parse_sparql(json: &str) -> CrawlResult<ReadOutcome> {
    let response = decode_response(json)?;
    require_vars(&response)?;
    let mut outcome = ReadOutcome::new();
    let mut seen_qids = HashSet::new();
    for (index, binding) in response.results.bindings.iter().enumerate() {
        let Some(line) = index.checked_add(1) else {
            continue;
        };
        if let Some(entry) = row_entry(binding, line, &mut outcome, &mut seen_qids) {
            outcome.push(entry);
        }
    }
    Ok(outcome)
}

fn decode_response(json: &str) -> CrawlResult<SparqlResponse> {
    serde_json::from_str(json).map_err(|source| crate::CrawlError::Decode {
        url: "wikidata-sparql".to_string(),
        source,
    })
}

fn require_vars(response: &SparqlResponse) -> CrawlResult<()> {
    let required_vars = ["item", "itemLabel", "website"];
    for var in required_vars {
        if !response.head.vars.contains(&var.to_string()) {
            return Err(crate::CrawlError::Schema {
                url: "wikidata-sparql".to_string(),
                detail: format!("missing required SPARQL variable: {}", var),
            });
        }
    }
    Ok(())
}

fn row_entry(
    binding: &SparqlBinding,
    line: usize,
    outcome: &mut ReadOutcome,
    seen_qids: &mut HashSet<String>,
) -> Option<SchoolDirectoryEntry> {
    let qid = row_qid(binding, line, outcome)?;
    if !seen_qids.insert(qid.to_string()) {
        outcome.note(line, "item", format!("duplicate QID {} skipped", qid));
        return None;
    }
    let Some(item_label) = binding_value(&binding.item_label) else {
        outcome.skip(line, "itemLabel", "the row has no label");
        return None;
    };
    let name = skip_row(outcome, line, "school name", SchoolName::parse(item_label))?;
    let website = row_website(binding, line, outcome)?;
    let state = row_state(binding, line, outcome)?;
    let city = row_city(binding, line, outcome);
    match SchoolDirectoryEntry::weak(name, city, state, SourceLabel::Wikidata) {
        Ok(entry) => Some(entry.with_website(Some(website))),
        Err(error) => {
            outcome.skip(line, "entry", error.to_string());
            None
        }
    }
}

fn row_qid<'a>(
    binding: &'a SparqlBinding,
    line: usize,
    outcome: &mut ReadOutcome,
) -> Option<&'a str> {
    let Some(item_url) = binding_value(&binding.item) else {
        outcome.skip(line, "item", "the row has no item identifier");
        return None;
    };
    let Some(qid) = qid_from_url(item_url) else {
        outcome.skip(line, "item", format!("{} is not a QID URL", item_url));
        return None;
    };
    if !is_valid_qid(qid) {
        outcome.skip(line, "item", format!("{} is not a valid QID", qid));
        return None;
    }
    Some(qid)
}

fn row_website(binding: &SparqlBinding, line: usize, outcome: &mut ReadOutcome) -> Option<Website> {
    let raw = match binding_value(&binding.website) {
        Some(value) if !value.trim().is_empty() => value,
        Some(_) | None => {
            outcome.skip(line, "website", "the row has no website");
            return None;
        }
    };
    match Website::parse(raw) {
        Ok(Some(website)) => Some(website),
        Ok(None) => {
            outcome.skip(line, "website", "the row website is empty");
            None
        }
        Err(error) => {
            outcome.skip(line, "website", error.to_string());
            None
        }
    }
}

fn row_state(
    binding: &SparqlBinding,
    line: usize,
    outcome: &mut ReadOutcome,
) -> Option<Option<UsJurisdiction>> {
    match binding_value(&binding.state) {
        Some(raw) if !raw.trim().is_empty() => match census_state(raw.trim()) {
            Some(state) => Some(Some(state)),
            None => {
                outcome.skip(
                    line,
                    "state",
                    format!("{} is not a census jurisdiction", raw.trim()),
                );
                None
            }
        },
        _ => Some(None),
    }
}

fn row_city(binding: &SparqlBinding, line: usize, outcome: &mut ReadOutcome) -> Option<CityName> {
    match binding_value(&binding.city) {
        Some(raw) if !raw.trim().is_empty() => {
            skip_failed(outcome, line, field("city", CityName::parse(raw.trim())))
        }
        _ => None,
    }
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
