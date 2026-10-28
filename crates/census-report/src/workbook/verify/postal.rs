use crate::export::postal::{append_state_zip, street_city};
use crate::export::ExportDataset;
use crate::report::{ReportError, ReportResult};
use census_domain::model::{CanonicalAthlete, CanonicalSchool, SchoolPostalAddress};
use census_domain::school_directory::AddressKind;
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::workbook) fn athlete_address_index(
    dataset: &ExportDataset,
    athletes: &[CanonicalAthlete],
) -> ReportResult<BTreeMap<String, String>> {
    athletes
        .iter()
        .map(|athlete| {
            address_line(dataset.schools.get(&athlete.school))
                .map(|line| (athlete.id.as_str().to_owned(), line))
        })
        .collect()
}

fn address_line<'a>(
    schools: impl IntoIterator<Item = &'a CanonicalSchool>,
) -> ReportResult<String> {
    let tuples = ordered_claims(schools)?
        .into_iter()
        .flatten()
        .map(|(school, claim)| {
            validate_owner(school, claim)?;
            Ok((&school.id, claim.address()))
        })
        .collect::<ReportResult<BTreeSet<_>>>()?;
    let mut expected = String::new();
    let mut units = 0usize;
    for (_, address) in tuples {
        let mut line = street_city(address);
        append_state_zip(&mut line, address);
        append_address(&mut expected, &mut units, &line)?;
    }
    Ok(expected)
}

fn append_address(cell: &mut String, units: &mut usize, line: &str) -> ReportResult<()> {
    let separator = usize::from(!cell.is_empty());
    let next = units
        .checked_add(separator)
        .and_then(|count| count.checked_add(line.encode_utf16().count()))
        .filter(|count| *count <= 32_767)
        .ok_or_else(|| ReportError::Invariant {
            detail: "frozen school address exceeds UTF-16 cell budget".into(),
        })?;
    let bytes = line
        .len()
        .checked_add(separator)
        .ok_or_else(|| ReportError::Invariant {
            detail: "frozen school address byte length overflow".into(),
        })?;
    cell.try_reserve(bytes)
        .map_err(|error| ReportError::Invariant {
            detail: format!("allocating frozen school address: {error}"),
        })?;
    if separator != 0 {
        cell.push('\n');
    }
    cell.push_str(line);
    *units = next;
    Ok(())
}

pub(in crate::workbook) fn fields<'a>(
    schools: impl IntoIterator<Item = &'a CanonicalSchool>,
) -> ReportResult<[String; 13]> {
    let (columns, count) = ordered_claims(schools)?.into_iter().flatten().try_fold(
        (std::array::from_fn(|_| String::new()), 0usize),
        |(mut columns, count), (school, claim)| {
            validate_owner(school, claim)?;
            append_claim(&mut columns, school, claim, count)?;
            Ok::<_, ReportError>((columns, count.saturating_add(1)))
        },
    )?;
    let expected_separators = count.saturating_sub(1);
    if columns.iter().any(|column| {
        column.bytes().filter(|byte| *byte == b'\n').count() != expected_separators
            || (count == 0 && !column.is_empty())
    }) {
        return Err(ReportError::Invariant {
            detail: "frozen postal columns have inconsistent claim positions".into(),
        });
    }
    Ok(columns)
}

fn validate_owner(school: &CanonicalSchool, claim: &SchoolPostalAddress) -> ReportResult<()> {
    claim
        .belongs_to(school)
        .map_err(|error| ReportError::Invariant {
            detail: format!(
                "frozen school {} has unsupported postal ownership: {error}",
                school.id
            ),
        })
}

fn append_claim(
    columns: &mut [String; 13],
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
        address.line1().map_or("", |value| value.as_str()),
        address.line2().map_or("", |value| value.as_str()),
        address.city().map_or("", |value| value.as_str()),
        address.state().map_or("", |value| value.code()),
        zip.as_str(),
    ];
    let values = components
        .into_iter()
        .chain(claim_values(claim, &namespace, &source));
    let result = columns
        .iter_mut()
        .zip(values)
        .try_for_each(|(column, value)| append_column(column, value, count));
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

fn append_column(column: &mut String, value: &str, count: usize) -> ReportResult<()> {
    let required = value.len().saturating_add(usize::from(count != 0));
    if column.len().saturating_add(required) > 32_767 {
        return Err(ReportError::Invariant {
            detail: "frozen postal cell budget exceeded".into(),
        });
    }
    column
        .try_reserve(required)
        .map_err(|error| ReportError::Invariant {
            detail: format!("allocating frozen postal expectation: {error}"),
        })?;
    if count != 0 {
        column.push('\n');
    }
    column.push_str(value);
    Ok(())
}

fn ordered_claims<'a>(
    schools: impl IntoIterator<Item = &'a CanonicalSchool>,
) -> ReportResult<[Option<(&'a CanonicalSchool, &'a SchoolPostalAddress)>; 128]> {
    let mut positions = [None; 128];
    let mut count = 0usize;
    for school in schools {
        for claim in &school.postal_addresses {
            let slot = positions
                .get_mut(count)
                .ok_or_else(|| ReportError::Invariant {
                    detail: "frozen postal claim budget exceeds 128".into(),
                })?;
            *slot = Some((school, claim));
            count = count.saturating_add(1);
        }
    }
    positions.sort_unstable_by_key(|position| position.map(|(school, claim)| (&school.id, claim)));
    Ok(positions)
}
