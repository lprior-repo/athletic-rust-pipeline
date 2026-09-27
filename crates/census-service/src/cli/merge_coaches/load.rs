use super::{url_find, Row};
use census_domain::model::{CONTACT_COLUMNS, CONTACT_PROOF_COLUMN};
use std::path::Path;

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
        verified_proof_digest: next(),
    }
}

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
        row.verified_proof_digest.clone(),
    ]
}

pub(super) fn dedupe_key(row: &Row) -> (String, String, String, String) {
    (
        row.school.trim().to_lowercase(),
        row.state.clone(),
        row.sport.trim().to_lowercase(),
        row.role.trim().to_lowercase(),
    )
}

fn validate_header(record: &[String]) -> Result<(), String> {
    let proof_index = CONTACT_COLUMNS.len();
    let expected_len = proof_index.saturating_add(1);
    if record.len() < expected_len {
        return Err(format!("expected at least {expected_len} columns"));
    }
    let normalized: Vec<String> = record
        .iter()
        .take(proof_index)
        .map(|h| h.trim().trim_start_matches('\u{feff}').to_string())
        .collect();
    let expected: Vec<String> = CONTACT_COLUMNS.iter().map(|s| s.to_string()).collect();
    if normalized != expected {
        return Err(format!("header mismatch: {normalized:?}"));
    }
    let proof = record.get(proof_index).map(|header| header.trim());
    if proof != Some(CONTACT_PROOF_COLUMN) {
        return Err(format!(
            "expected column {expected_len} to be {CONTACT_PROOF_COLUMN}"
        ));
    }
    Ok(())
}

fn is_empty_record(record: &[String]) -> bool {
    record.iter().all(|f| f.trim().is_empty())
}

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
        let line_no = idx.saturating_add(1);
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
        if row.verified_proof_digest.is_empty() {
            return (
                Vec::new(),
                Some(format!("missing proof digest at line {line_no}")),
            );
        }
        records.push((line_no, row));
    }
    (records, None)
}

pub(super) fn normalize(row: &mut Row) {
    if let Some(m) = url_find().and_then(|re| re.captures(&row.source_url)) {
        if let Some(whole) = m.get(0) {
            row.source_url = whole.as_str().to_string();
        }
    }
}
