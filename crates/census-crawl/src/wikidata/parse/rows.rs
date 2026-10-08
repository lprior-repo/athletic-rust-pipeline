use super::{check_size, value, SparqlBinding};
use crate::directory::{census_state, issue_detail, skip_optional, skip_row, ReadOutcome};
use census_domain::school_directory::{
    CityName, DirectoryError, SchoolDirectoryEntry, SchoolName, SourceLabel, Website,
};
use census_domain::UsJurisdiction;
use serde_json::value::RawValue;
use std::collections::HashSet;

pub(super) fn row_entry(
    binding: &SparqlBinding<'_>,
    line: usize,
    outcome: &mut ReadOutcome,
    seen: &mut HashSet<String>,
) -> Result<Option<SchoolDirectoryEntry>, DirectoryError> {
    let Some(name) = row_name(binding, line, outcome, seen)? else {
        return Ok(None);
    };
    let Some(website) = row_website(binding.website, line, outcome)? else {
        return Ok(None);
    };
    let Some(state) = row_state(binding.state, line, outcome)? else {
        return Ok(None);
    };
    let city = row_city(binding.city, line, outcome)?;
    let entry = SchoolDirectoryEntry::weak(name, city, state, SourceLabel::Wikidata);
    Ok(skip_row(outcome, line, "entry", entry)?.map(|entry| entry.with_website(Some(website))))
}

fn row_name(
    binding: &SparqlBinding<'_>,
    line: usize,
    outcome: &mut ReadOutcome,
    seen: &mut HashSet<String>,
) -> Result<Option<SchoolName>, DirectoryError> {
    if identify(binding.item, line, outcome, seen)?.is_none() {
        return Ok(None);
    }
    let Some(name) = required(binding.item_label, line, "itemLabel", outcome)? else {
        return Ok(None);
    };
    skip_row(outcome, line, "school name", SchoolName::parse(&name))
}

fn identify(
    raw: Option<&RawValue>,
    line: usize,
    outcome: &mut ReadOutcome,
    seen: &mut HashSet<String>,
) -> Result<Option<()>, DirectoryError> {
    let Some(item) = required(raw, line, "item", outcome)? else {
        return Ok(None);
    };
    let Some(qid) = qid_from_url(&item).filter(|qid| valid_qid(qid)) else {
        outcome.skip(line, "item", "the item is not a valid QID URL")?;
        return Ok(None);
    };
    if seen.contains(qid) {
        outcome.note(line, "item", "duplicate QID skipped")?;
        return Ok(None);
    }
    remember(seen, qid)?;
    Ok(Some(()))
}

fn required(
    raw: Option<&RawValue>,
    line: usize,
    field: &'static str,
    outcome: &mut ReadOutcome,
) -> Result<Option<String>, DirectoryError> {
    match value(raw) {
        Ok(Some(value)) if !value.trim().is_empty() => Ok(Some(value)),
        Err(error) if error.is_resource() => Err(error),
        Ok(_) | Err(_) => {
            outcome.skip(line, field, "the row has no usable required string value")?;
            Ok(None)
        }
    }
}

fn row_website(
    raw: Option<&RawValue>,
    line: usize,
    outcome: &mut ReadOutcome,
) -> Result<Option<Website>, DirectoryError> {
    let Some(raw) = required(raw, line, "website", outcome)? else {
        return Ok(None);
    };
    match Website::parse(&raw) {
        Ok(Some(website)) => Ok(Some(website)),
        Ok(None) => {
            outcome.skip(line, "website", "the row has no usable website")?;
            Ok(None)
        }
        Err(error) => {
            outcome.skip(line, "website", issue_detail(format_args!("{error}"))?)?;
            Ok(None)
        }
    }
}

fn row_state(
    raw: Option<&RawValue>,
    line: usize,
    outcome: &mut ReadOutcome,
) -> Result<Option<Option<UsJurisdiction>>, DirectoryError> {
    let parsed = value(raw).and_then(parse_state);
    skip_row(outcome, line, "state", parsed)
}

fn parse_state(raw: Option<String>) -> Result<Option<UsJurisdiction>, DirectoryError> {
    match raw {
        Some(raw) if !raw.trim().is_empty() => {
            census_state(raw.trim())
                .map(Some)
                .ok_or(DirectoryError::UnsupportedValue {
                    field: "state",
                    value: raw,
                })
        }
        _ => Ok(None),
    }
}

fn row_city(
    raw: Option<&RawValue>,
    line: usize,
    outcome: &mut ReadOutcome,
) -> Result<Option<CityName>, DirectoryError> {
    let raw = skip_optional(outcome, line, "city", value(raw))?;
    let parsed = raw
        .filter(|raw| !raw.trim().is_empty())
        .map(|raw| CityName::parse(&raw));
    skip_optional(outcome, line, "city", parsed.transpose())
}

fn remember(seen: &mut HashSet<String>, qid: &str) -> Result<(), DirectoryError> {
    check_size("wikidata QIDs", seen.len().saturating_add(1), 20_000)?;
    seen.try_reserve(1)
        .map_err(|_| DirectoryError::Allocation {
            resource: "wikidata QIDs",
        })?;
    let mut owned = String::new();
    owned
        .try_reserve_exact(qid.len())
        .map_err(|_| DirectoryError::Allocation {
            resource: "wikidata QID",
        })?;
    owned.push_str(qid);
    seen.insert(owned);
    Ok(())
}

fn qid_from_url(url: &str) -> Option<&str> {
    url.strip_prefix("http://www.wikidata.org/entity/")
        .or_else(|| url.strip_prefix("http://wikidata.org/entity/"))
}

fn valid_qid(qid: &str) -> bool {
    qid.strip_prefix('Q')
        .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|ch| ch.is_ascii_digit()))
}
