use super::super::DetailRow;
use crate::domain::{evidence::ProfileEvidence, facts::Location as FactLocation};
use anyhow::{bail, Result};
use std::collections::HashMap;
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

pub(super) fn verify_identity(row: &DetailRow, profile: &ProfileEvidence) -> Result<()> {
    let first = source_field(row, "Person First")?;
    let last = source_field(row, "Person Last")?;
    let school = normalized_school(&source_field(row, "Schools Name")?);
    let city = source_field_optional(row, "Address Mailing / Permanent City");
    let region = source_field_optional(row, "Address Mailing / Permanent Region");
    if normalized(&format!("{first} {last}")).is_empty()
        || matches!(
            school.as_str(),
            "" | "unknown" | "not provided" | "none" | "null" | "n a" | "other" | "high school"
        )
    {
        bail!("accepted source lacks meaningful name or school identity");
    }
    if city.is_none() && region.is_none() {
        bail!("accepted source has no location context");
    }
    if normalized(&format!("{first} {last}")) != normalized(profile.name.value.as_str()) {
        bail!("retained profile name contradicts source identity");
    }
    let city = city.as_deref().map(normalized);
    let region = region.as_deref().map(normalized);
    let mut corroboration = HashMap::new();
    if !profile
        .teams
        .iter()
        .filter(|team| normalized_school(team.name.value.as_str()) == school)
        .any(|team| {
            let Some(observation) = team.location.as_ref() else {
                return false;
            };
            let fields = corroboration
                .entry(team.team_id)
                .or_insert((city.is_none(), region.is_none()));
            let observed = location_matches(&observation.value, city.as_deref(), region.as_deref());
            fields.0 |= observed.0;
            fields.1 |= observed.1;
            fields.0 && fields.1
        })
    {
        bail!("retained profile lacks source-matching school and location evidence");
    }
    Ok(())
}

pub(super) fn source_field(row: &DetailRow, name: &str) -> Result<String> {
    source_field_optional(row, name)
        .ok_or_else(|| anyhow::anyhow!("accepted source field is empty: {name}"))
}

pub(super) fn source_field_optional(row: &DetailRow, name: &str) -> Option<String> {
    row.source
        .fields
        .get(name)
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

pub(super) fn location_matches(
    location: &FactLocation,
    city: Option<&str>,
    region: Option<&str>,
) -> (bool, bool) {
    match location {
        FactLocation::Missing => (false, false),
        FactLocation::RegionOnly(value) => (
            false,
            region.is_some_and(|actual| normalized(value.as_str()) == actual),
        ),
        FactLocation::CityOnly(value) => (
            city.is_some_and(|actual| normalized(value.as_str()) == actual),
            false,
        ),
        FactLocation::CityRegion {
            city: observed_city,
            region: observed_region,
        } => (
            city.is_some_and(|actual| normalized(observed_city.as_str()) == actual),
            region.is_some_and(|actual| normalized(observed_region.as_str()) == actual),
        ),
    }
}

pub(super) fn normalized(raw: &str) -> String {
    raw.nfkd()
        .filter(|character| !is_combining_mark(*character))
        .fold(String::new(), |mut output, character| {
            if character.is_alphanumeric() {
                output.extend(character.to_lowercase());
            } else if !output.ends_with(' ') {
                output.push(' ');
            }
            output
        })
        .trim()
        .to_owned()
}

pub(super) fn normalized_school(raw: &str) -> String {
    let mut value = normalized(raw);
    let suffix = " high school";
    if value.ends_with(suffix) {
        value.truncate(value.len().saturating_sub(suffix.len()));
    }
    value
}
