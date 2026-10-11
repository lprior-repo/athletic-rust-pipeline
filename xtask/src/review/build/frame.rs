use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub(super) fn frame_counts(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut header: Vec<String> = Vec::new();
    let mut website = usize::MAX;
    let mut zip = usize::MAX;
    let mut rows = 0u64;
    let mut website_claims = 0u64;
    let mut with_zip = 0u64;
    scan_csv(&text, |record, field, value| {
        if record == 0 {
            header.push(value.to_lowercase());
            return;
        }
        if website == usize::MAX {
            website = header
                .iter()
                .position(|name| name == "website")
                .map_or(usize::MAX, core::convert::identity);
            zip = header
                .iter()
                .position(|name| name == "zip")
                .map_or(usize::MAX, core::convert::identity);
        }
        if field == 0 {
            rows = rows.saturating_add(1);
        }
        if field == website && value.starts_with("http") {
            website_claims = website_claims.saturating_add(1);
        }
        if field == zip && !value.trim().is_empty() {
            with_zip = with_zip.saturating_add(1);
        }
    });
    Ok(json!({"rows": rows, "website_claims": website_claims, "with_zip": with_zip}))
}

pub(super) fn scan_csv(text: &str, mut on_field: impl FnMut(usize, usize, &str)) {
    let mut record = 0usize;
    let mut field = 0usize;
    let mut value = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                value.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => {
                on_field(record, field, &value);
                value.clear();
                field = field.saturating_add(1);
            }
            '\n' if !quoted => {
                on_field(record, field, &value);
                value.clear();
                field = 0;
                record = record.saturating_add(1);
            }
            '\r' if !quoted => {}
            _ => value.push(c),
        }
    }
    if field > 0 || !value.is_empty() {
        on_field(record, field, &value);
    }
}
