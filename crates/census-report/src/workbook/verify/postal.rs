use crate::export::ExportDataset;
use crate::report::{ReportError, ReportResult};
use census_domain::model::{CanonicalAthlete, CanonicalSchool, SchoolPostalAddress};
use census_domain::school_directory::AddressKind;
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::workbook) fn athlete_address_index(
    dataset: &ExportDataset,
    athletes: &[CanonicalAthlete],
) -> ReportResult<BTreeMap<String, String>> {
    let groups = address_affiliations(dataset, athletes);
    groups
        .into_iter()
        .map(|(canonical, schools)| {
            address_line(schools.into_iter().filter_map(|id| dataset.schools.get(id)))
                .map(|line| (canonical.to_owned(), line))
        })
        .collect()
}

fn address_affiliations<'a>(
    dataset: &'a ExportDataset,
    athletes: &'a [CanonicalAthlete],
) -> BTreeMap<&'a str, BTreeSet<&'a census_domain::model::SchoolId>> {
    let selected = athletes
        .iter()
        .map(|row| row.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut groups = BTreeMap::<&str, BTreeSet<&census_domain::model::SchoolId>>::new();
    dataset.athletes.iter().chain(athletes).for_each(|row| {
        let canonical = dataset
            .canonical_aliases
            .get(row.id.as_str())
            .map_or(row.id.as_str(), String::as_str);
        if selected.contains(canonical) {
            groups.entry(canonical).or_default().insert(&row.school);
        }
    });
    groups
}

fn address_line<'a>(
    schools: impl IntoIterator<Item = &'a CanonicalSchool>,
) -> ReportResult<String> {
    let Some((_, claim)) = ordered_claims(schools)?.into_iter().flatten().next() else {
        return Ok(String::new());
    };
    let address = claim.address();
    let mut line = street_city(address);
    append_state_zip(&mut line, address);
    Ok(line)
}

fn street_city(address: &census_domain::school_directory::PostalAddress) -> String {
    [
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
    })
}

fn append_state_zip(line: &mut String, address: &census_domain::school_directory::PostalAddress) {
    let state = address.state().map(|state| state.code());
    let zip = address.zip().map(|value| value.to_string());
    let head = separator(line);
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
}

fn separator(line: &str) -> &'static str {
    if line.is_empty() {
        ""
    } else {
        ", "
    }
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
