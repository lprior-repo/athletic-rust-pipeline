use census_domain::model::{CanonicalAthlete, CanonicalSchool};
use census_domain::school_directory::PostalAddress;
use std::collections::BTreeMap;

use super::{ordered_claims, ExportDataset, ReportResult};

pub(super) fn index(
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
    let Some((_, claim)) = ordered_claims(schools)?.into_iter().flatten().next() else {
        return Ok(String::new());
    };
    let address = claim.address();
    let mut line = street_city(address);
    append_state_zip(&mut line, address);
    Ok(line)
}

fn street_city(address: &PostalAddress) -> String {
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

fn append_state_zip(line: &mut String, address: &PostalAddress) {
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
