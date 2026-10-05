use census_domain::model::{CanonicalAthlete, CanonicalSchool, SchoolPostalAddress};
use std::collections::{BTreeMap, BTreeSet};

use super::ExportDataset;
use crate::report::{ReportError, ReportResult};

pub const POSTAL_HEADERS: [&str; 12] = [
    "Postal School ID",
    "Postal Street",
    "Postal Second Line",
    "Postal City",
    "Postal State",
    "Postal ZIP",
    "Postal Owner Namespace",
    "Postal Owner ID",
    "Postal Source",
    "Postal Source URL",
    "Postal Observed Date",
    "Postal Capture SHA256",
];

pub const POSTAL_CSV_HEADERS: [&str; 12] = [
    "postal_school_id",
    "postal_street",
    "postal_second_line",
    "postal_city",
    "postal_state",
    "postal_zip",
    "postal_owner_namespace",
    "postal_owner_id",
    "postal_source",
    "postal_source_url",
    "postal_observed_date",
    "postal_capture_sha256",
];

pub const ATHLETE_ADDRESS_HEADER: &str = "School Address";

pub const ATHLETE_ADDRESS_CSV_HEADER: &str = "school_address";

pub fn athlete_address_index(
    dataset: &ExportDataset,
    athletes: &[CanonicalAthlete],
) -> ReportResult<BTreeMap<String, String>> {
    let selected = athletes
        .iter()
        .map(|athlete| athlete.id.as_str())
        .collect::<BTreeSet<_>>();
    let affiliations = dataset.athletes.iter().chain(athletes).fold(
        BTreeMap::<&str, BTreeSet<&census_domain::model::SchoolId>>::new(),
        |mut groups, member| {
            let subject = member.id.as_str();
            let canonical = dataset
                .canonical_aliases
                .get(subject)
                .map_or(subject, String::as_str);
            if selected.contains(canonical) {
                groups.entry(canonical).or_default().insert(&member.school);
            }
            groups
        },
    );
    affiliations
        .into_iter()
        .map(|(athlete, schools)| {
            address_line(schools.into_iter().filter_map(|id| dataset.schools.get(id)))
                .map(|line| (athlete.to_owned(), line))
        })
        .collect()
}

fn address_line<'a>(
    schools: impl IntoIterator<Item = &'a CanonicalSchool>,
) -> ReportResult<String> {
    let Some((_, claim)) = ordered_claims(schools)?.into_iter().flatten().next() else {
        return Ok(String::new());
    };
    let address = claim.address();
    let mut line = [
        address.line1().map(|value| value.as_str()),
        address.line2().map(|value| value.as_str()),
        address.city().map(|value| value.as_str()),
    ]
    .into_iter()
    .flatten()
    .filter(|part| !part.is_empty())
    .fold(String::new(), |mut line, part| {
        if !line.is_empty() {
            line.push_str(", ");
        }
        line.push_str(part);
        line
    });
    let state = address.state().map(|state| state.code());
    let zip = address.zip().map(|value| value.to_string());
    let head = if line.is_empty() { "" } else { ", " };
    match (state, zip.as_deref()) {
        (Some(state), Some(zip)) => {
            line.push_str(head);
            line.push_str(state);
            line.push(' ');
            line.push_str(zip);
        }
        (Some(state), None) => {
            line.push_str(head);
            line.push_str(state);
        }
        (None, Some(zip)) => {
            line.push_str(head);
            line.push_str(zip);
        }
        (None, None) => {}
    }
    Ok(line)
}

pub fn postal_fields<'a>(
    schools: impl IntoIterator<Item = &'a CanonicalSchool>,
) -> ReportResult<[String; 12]> {
    ordered_claims(schools)?
        .into_iter()
        .flatten()
        .try_fold(
            (std::array::from_fn(|_| String::new()), 0usize),
            |(mut fields, count), (school, claim)| {
                append_claim(&mut fields, school, claim, count)?;
                Ok((fields, count.saturating_add(1)))
            },
        )
        .and_then(|(fields, count)| {
            if fields.iter().any(|field| {
                if count == 0 {
                    !field.is_empty()
                } else {
                    field.split('\n').count() != count
                }
            }) {
                return Err(defect("postal columns have inconsistent claim positions"));
            }
            Ok(fields)
        })
}

fn ordered_claims<'a>(
    schools: impl IntoIterator<Item = &'a CanonicalSchool>,
) -> ReportResult<[Option<(&'a CanonicalSchool, &'a SchoolPostalAddress)>; 128]> {
    let mut claims = [None; 128];
    let mut used = 0usize;
    schools
        .into_iter()
        .flat_map(|school| {
            school
                .postal_addresses
                .iter()
                .map(move |claim| (school, claim))
        })
        .try_for_each(|(school, claim)| -> ReportResult<()> {
            validate_claim(school, claim, used)?;
            let slot = claims
                .get_mut(used)
                .ok_or_else(|| defect("postal claim count exceeds publication budget 128"))?;
            *slot = Some((school, claim));
            used = used.saturating_add(1);
            Ok(())
        })?;
    claims.sort_unstable_by(|left, right| {
        left.map(|(school, claim)| (&school.id, claim))
            .cmp(&right.map(|(school, claim)| (&school.id, claim)))
    });
    Ok(claims)
}

fn validate_claim(
    school: &CanonicalSchool,
    claim: &SchoolPostalAddress,
    count: usize,
) -> ReportResult<()> {
    claim.belongs_to(school).map_err(|error| {
        defect(&format!(
            "school {} postal claim owned by {}:{} requires review: {error}",
            school.id,
            claim.owner().namespace,
            claim.owner().id,
        ))
    })?;
    if count >= 128 {
        return Err(defect("postal claim count exceeds publication budget 128"));
    }
    Ok(())
}

fn append_claim(
    fields: &mut [String; 12],
    school: &CanonicalSchool,
    claim: &SchoolPostalAddress,
    count: usize,
) -> ReportResult<()> {
    let address = claim.address();
    let namespace = claim.owner().namespace.to_string();
    let source = claim.source_label().label();
    let zip = address.zip().map_or_else(String::new, ToString::to_string);
    let values = [
        school.id.as_str(),
        address.line1().map_or("", |line| line.as_str()),
        address.line2().map_or("", |line| line.as_str()),
        address.city().map_or("", |city| city.as_str()),
        address.state().map_or("", |state| state.code()),
        zip.as_str(),
        namespace.as_str(),
        claim.owner().id.as_str(),
        source.as_str(),
        claim
            .evidence()
            .source
            .url
            .as_deref()
            .map_or(Default::default(), core::convert::identity),
        claim.evidence().observed_on.as_str(),
        claim.capture_sha256(),
    ];
    fields
        .iter_mut()
        .zip(values)
        .try_for_each(|(field, value)| {
            let separator = usize::from(count > 0);
            let length = field
                .len()
                .checked_add(separator)
                .and_then(|length| length.checked_add(value.len()))
                .ok_or_else(|| defect("postal field length overflow"))?;
            if length > 32_767 {
                return Err(defect("postal field exceeds the publication cell budget"));
            }
            field
                .try_reserve(separator.saturating_add(value.len()))
                .map_err(|error| defect(&format!("allocating postal field: {error}")))?;
            if count > 0 {
                field.push('\n');
            }
            field.push_str(value);
            Ok(())
        })?;
    Ok(())
}

fn defect(detail: &str) -> ReportError {
    ReportError::Invariant {
        detail: detail.to_owned(),
    }
}

#[cfg(test)]
pub(crate) mod tests;
