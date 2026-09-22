//! Row parsing, CSV loading, and in-place normalization.

use super::{url_find, Row, HEADER, PERSONAL_MAIL};
use std::path::Path;

/// Build a Row from parsed CSV fields, padding with blanks as needed.
pub(super) fn from_fields(fields: Vec<String>) -> Row {
    let mut rest = fields
        .into_iter()
        .chain(std::iter::repeat_with(String::new));
    let mut next = move || rest.next().unwrap_or_default();
    Row {
        school: next(),
        city: next(),
        state: next().trim().to_uppercase(),
        sport: next(),
        role: next(),
        coach_name: next(),
        public_professional_email: next(),
        ad_name: next(),
        ad_email: next(),
        source_url: next(),
        last_observed: next(),
    }
}

/// Return the row as a flat vec of field strings, in HEADER order.
pub(super) fn to_fields(row: &Row) -> Vec<String> {
    vec![
        row.school.clone(),
        row.city.clone(),
        row.state.clone(),
        row.sport.clone(),
        row.role.clone(),
        row.coach_name.clone(),
        row.public_professional_email.clone(),
        row.ad_name.clone(),
        row.ad_email.clone(),
        row.source_url.clone(),
        row.last_observed.clone(),
    ]
}

/// Dedupe key: (lowercased school, state, lowercased sport, lowercased role).
pub(super) fn dedupe_key(row: &Row) -> (String, String, String, String) {
    (
        row.school.trim().to_lowercase(),
        row.state.clone(),
        row.sport.trim().to_lowercase(),
        row.role.trim().to_lowercase(),
    )
}

/// Validate the header row against the expected HEADER.
fn validate_header(record: &[String]) -> Result<(), String> {
    let normalized: Vec<String> = record
        .iter()
        .map(|h| h.trim().trim_start_matches('\u{feff}').to_string())
        .collect();
    let expected: Vec<String> = HEADER.iter().map(|s| s.to_string()).collect();
    if normalized != expected {
        Err(format!("header mismatch: {:?}", normalized))
    } else {
        Ok(())
    }
}

/// Check if a record is completely empty.
fn is_empty_record(record: &[String]) -> bool {
    record.iter().all(|f| f.trim().is_empty())
}

/// Load rows from a fragment CSV file.
pub(super) fn load_rows(path: &Path, expected_state: &str) -> (Vec<(usize, Row)>, Option<String>) {
    let mut content = String::new();
    if let Err(e) = std::fs::read_to_string(path).map(|s| content = s) {
        return (Vec::new(), Some(format!("read error: {e}")));
    }
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(content.as_bytes());
    let mut records: Vec<(usize, Row)> = Vec::new();
    let mut header_found = false;
    for (idx, result) in reader.records().enumerate() {
        let line_no = idx + 1;
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                return (
                    Vec::new(),
                    Some(format!("parse error at line {line_no}: {e}")),
                )
            }
        };
        if !header_found {
            let fields: Vec<String> = record.iter().map(|f| f.to_string()).collect();
            if let Err(e) = validate_header(&fields) {
                return (Vec::new(), Some(e));
            }
            header_found = true;
            continue;
        }
        let fields: Vec<String> = record.iter().map(|f| f.to_string()).collect();
        if is_empty_record(&fields) {
            continue;
        }
        let mut row = Row::from_fields(fields);
        let state_field = row.state.trim().to_uppercase();
        row.state = if state_field.is_empty() {
            expected_state.to_string()
        } else {
            state_field
        };
        records.push((line_no, row));
    }
    (records, None)
}

/// Normalize a row in place: trim source_url, blank personal-mail addresses.
pub(super) fn normalize(row: &mut Row) {
    if let Some(m) = url_find().and_then(|re| re.captures(&row.source_url)) {
        if let Some(whole) = m.get(0) {
            row.source_url = whole.as_str().to_string();
        }
    }
    for field in [&mut row.public_professional_email, &mut row.ad_email] {
        let value = field.trim().to_string();
        if value.is_empty() || !value.contains('@') {
            continue;
        }
        let domain = value.rsplit_once('@').map(|(_, d)| d.trim().to_lowercase());
        if let Some(domain) = domain {
            if PERSONAL_MAIL.contains(&domain.as_str()) {
                *field = String::new();
            }
        }
    }
}
