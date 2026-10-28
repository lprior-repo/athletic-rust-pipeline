use census_domain::model::{CanonicalAthlete, CanonicalSchool};
use std::collections::BTreeMap;

use super::{append_state_zip, defect, ordered_claims, street_city, ExportDataset, ReportResult};

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

#[cfg(test)]
mod tests;
