use super::{
    OwnedCompleteness, OwnedMeetPage, OwnedMeetVerdict, OwnedRejection, OwnedRejectionKind,
    MAX_OWNED_BODY_BYTES, MAX_OWNED_ROWS,
};
use serde_json::Value;
use std::collections::HashSet;

pub fn parse_owned_meet(body: &[u8], meet_id: u64) -> OwnedMeetVerdict {
    match parse_page(body, meet_id) {
        Ok(page) => OwnedMeetVerdict::Parsed(page),
        Err(detail) => OwnedMeetVerdict::Malformed { detail },
    }
}

fn parse_page(body: &[u8], meet_id: u64) -> Result<OwnedMeetPage, String> {
    if meet_id == 0 || body.len() > MAX_OWNED_BODY_BYTES {
        return Err("invalid meet ID or oversized structured response".to_string());
    }
    let mut document: Value = serde_json::from_slice(body).map_err(|error| error.to_string())?;
    check_envelope(&document, meet_id)?;
    let data = document
        .as_object_mut()
        .and_then(|object| object.remove("data"))
        .and_then(|data| match data {
            Value::Array(rows) => Some(rows),
            _ => None,
        })
        .ok_or_else(|| "expected a JSON object with a data array".to_string())?;
    if data.len() > MAX_OWNED_ROWS {
        return Err(format!("structured data exceeds {MAX_OWNED_ROWS} rows"));
    }
    parse_rows(data, meet_id)
}

fn parse_rows(data: Vec<Value>, meet_id: u64) -> Result<OwnedMeetPage, String> {
    let mut page = OwnedMeetPage {
        rows: Vec::new(),
        rejected: Vec::new(),
        published_rows: data.len(),
        completeness: OwnedCompleteness::Unknown,
    };
    page.rows
        .try_reserve(data.len())
        .map_err(|error| error.to_string())?;
    page.rejected
        .try_reserve(data.len())
        .map_err(|error| error.to_string())?;
    let mut seen = HashSet::new();
    seen.try_reserve(data.len())
        .map_err(|error| error.to_string())?;
    data.into_iter().enumerate().for_each(|(index, value)| {
        let locator = format!("data[{index}]");
        match super::row::parse_row(value, meet_id, locator.clone()) {
            Ok(row) if seen.insert(row.result_id) => page.rows.push(row),
            Ok(row) => page.rejected.push(OwnedRejection {
                locator,
                kind: OwnedRejectionKind::DuplicateResult,
                detail: format!("repeated source result ID {}", row.result_id),
            }),
            Err((kind, detail)) => page.rejected.push(OwnedRejection {
                locator,
                kind,
                detail,
            }),
        }
    });
    Ok(page)
}

fn check_envelope(document: &Value, meet_id: u64) -> Result<(), String> {
    if let Some(status) = document.pointer("/_meta/status_code") {
        if status.as_u64() != Some(200) {
            return Err(format!("provider envelope status is {status}"));
        }
    }
    let Some(published) = document.pointer("/_embedded/meet/id") else {
        return Ok(());
    };
    let id = published
        .as_u64()
        .or_else(|| published.as_str().and_then(|id| id.parse::<u64>().ok()));
    if id != Some(meet_id) {
        return Err(format!(
            "embedded meet ID {published} differs from requested meet {meet_id}"
        ));
    }
    Ok(())
}
