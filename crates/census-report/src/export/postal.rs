use census_domain::model::{CanonicalAthlete, CanonicalSchool, SchoolPostalAddress};
use census_domain::school_directory::AddressKind;
use std::collections::BTreeMap;

use super::ExportDataset;
use crate::report::{ReportError, ReportResult};

mod address;

pub const POSTAL_HEADERS: [&str; 13] = [
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
    "Postal Address Kind",
];

pub const POSTAL_CSV_HEADERS: [&str; 13] = [
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
    "postal_address_kind",
];

pub const ATHLETE_ADDRESS_HEADER: &str = "School Address";

pub const ATHLETE_ADDRESS_CSV_HEADER: &str = "school_address";

pub fn athlete_address_index(
    dataset: &ExportDataset,
    athletes: &[CanonicalAthlete],
) -> ReportResult<BTreeMap<String, String>> {
    address::index(dataset, athletes)
}

pub fn postal_fields<'a>(
    schools: impl IntoIterator<Item = &'a CanonicalSchool>,
) -> ReportResult<[String; 13]> {
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
        .and_then(|(fields, count)| checked_fields(fields, count))
}

fn checked_fields(fields: [String; 13], count: usize) -> ReportResult<[String; 13]> {
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
            insert_claim(&mut claims, &mut used, school, claim)
        })?;
    claims.sort_unstable_by(|left, right| {
        left.map(|(school, claim)| (&school.id, claim))
            .cmp(&right.map(|(school, claim)| (&school.id, claim)))
    });
    Ok(claims)
}

type Claims<'a> = [Option<(&'a CanonicalSchool, &'a SchoolPostalAddress)>; 128];

fn insert_claim<'a>(
    claims: &mut Claims<'a>,
    used: &mut usize,
    school: &'a CanonicalSchool,
    claim: &'a SchoolPostalAddress,
) -> ReportResult<()> {
    validate_claim(school, claim, *used)?;
    let slot = claims
        .get_mut(*used)
        .ok_or_else(|| defect("postal claim count exceeds publication budget 128"))?;
    *slot = Some((school, claim));
    *used = used.saturating_add(1);
    Ok(())
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
    fields: &mut [String; 13],
    school: &CanonicalSchool,
    claim: &SchoolPostalAddress,
    count: usize,
) -> ReportResult<()> {
    let address = claim.address();
    let namespace = claim.owner().namespace.to_string();
    let source = claim.source_label().label();
    let zip = address.zip().map_or_else(String::new, ToString::to_string);
    let components = [
        school.id.as_str(),
        address.line1().map_or("", |line| line.as_str()),
        address.line2().map_or("", |line| line.as_str()),
        address.city().map_or("", |city| city.as_str()),
        address.state().map_or("", |state| state.code()),
        zip.as_str(),
    ];
    let values = components
        .into_iter()
        .chain(claim_values(claim, &namespace, &source));
    let result = fields
        .iter_mut()
        .zip(values)
        .try_for_each(|(field, value)| append_field(field, value, count));
    result
}

fn claim_values<'a>(
    claim: &'a SchoolPostalAddress,
    namespace: &'a str,
    source: &'a str,
) -> [&'a str; 7] {
    [
        namespace,
        claim.owner().id.as_str(),
        source,
        claim
            .evidence()
            .source
            .url
            .as_deref()
            .map_or("", core::convert::identity),
        claim.evidence().observed_on.as_str(),
        claim.capture_sha256(),
        match claim.address().kind() {
            AddressKind::Physical => "physical",
            AddressKind::Mailing => "mailing",
            AddressKind::Unknown => "unknown",
        },
    ]
}

fn append_field(field: &mut String, value: &str, count: usize) -> ReportResult<()> {
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
}

fn defect(detail: &str) -> ReportError {
    ReportError::Invariant {
        detail: detail.to_owned(),
    }
}

#[cfg(test)]
pub(crate) mod tests;
