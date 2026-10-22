use census_domain::model::{CanonicalAthlete, CanonicalSchool};
use census_domain::school_directory::PostalAddress;
use std::collections::BTreeMap;

use super::{defect, ordered_claims, ExportDataset, ReportResult};

const CELL_UTF16_LIMIT: usize = 32_767;

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
    let mut cell = String::new();
    let mut units = 0usize;
    let mut previous = None;
    for (school, claim) in ordered_claims(schools)?.into_iter().flatten() {
        let tuple = (&school.id, claim.address());
        if previous == Some(tuple) {
            continue;
        }
        let mut line = street_city(claim.address());
        append_state_zip(&mut line, claim.address());
        append_line(&mut cell, &mut units, &line)?;
        previous = Some(tuple);
    }
    Ok(cell)
}

fn append_line(cell: &mut String, units: &mut usize, line: &str) -> ReportResult<()> {
    let separator = usize::from(!cell.is_empty());
    let length = units
        .checked_add(separator)
        .and_then(|length| length.checked_add(line.encode_utf16().count()))
        .ok_or_else(|| defect("school address UTF-16 length overflow"))?;
    if length > CELL_UTF16_LIMIT {
        return Err(defect(
            "school address exceeds the publication UTF-16 cell budget",
        ));
    }
    let additional = separator
        .checked_add(line.len())
        .ok_or_else(|| defect("school address byte length overflow"))?;
    cell.try_reserve(additional)
        .map_err(|error| defect(&format!("allocating school address: {error}")))?;
    if separator != 0 {
        cell.push('\n');
    }
    cell.push_str(line);
    *units = length;
    Ok(())
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

#[cfg(test)]
mod tests;
