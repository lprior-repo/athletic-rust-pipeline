use crate::replay::{ensure_rows, unmapped, Capture};
use anyhow::{bail, Context, Result};
use census_crawl::pa_piaa::parse::{parse_details, parse_directory};
use serde_json::Value;

pub(super) fn replay(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    if let Some(letter) = file
        .strip_prefix("directory_alpha_")
        .and_then(|rest| rest.strip_suffix(".html"))
    {
        let rows = parse_directory(body).with_context(|| format!("parsing {file}"))?;
        ensure_rows(file, rows.len(), "member schools")?;
        let Some(first) = rows.first() else {
            bail!("{file} yielded no first member school");
        };
        return Ok(format!(
            "directory_letter letter={letter} schools={} first={:?} id={}",
            rows.len(),
            first.name,
            first.school_id
        ));
    }
    if let Some(id) = file
        .strip_prefix("details_")
        .and_then(|rest| rest.strip_suffix(".html"))
    {
        let page = parse_details(body).with_context(|| format!("parsing {file}"))?;
        if page.school_name.trim().is_empty() {
            bail!("{file} yielded no school name: the body did not parse");
        }
        return Ok(format!(
            "details id={id} school={:?} contacts={}",
            page.school_name,
            page.contacts.len()
        ));
    }
    if file == "PROVENANCE.json" {
        return provenance(capture, body);
    }
    if let Some(name) = file
        .strip_prefix("golden_")
        .and_then(|rest| rest.strip_suffix(".json"))
    {
        return golden(capture, name, body);
    }
    unmapped("pa_piaa", file)
}

fn provenance(capture: &Capture<'_>, body: &str) -> Result<String> {
    let record: Value = serde_json::from_str(body).context("decoding PROVENANCE.json")?;
    let fixtures = record
        .get("fixtures")
        .and_then(Value::as_array)
        .context("PROVENANCE.json carries no `fixtures` array")?;
    let mut verified = 0usize;
    for fixture in fixtures {
        let file = fixture
            .get("file")
            .and_then(Value::as_str)
            .context("a provenance entry carries no `file`")?;
        let Some(captured) = capture.corpus.get(file) else {
            bail!("PROVENANCE.json names {file}, which the fixture directory does not hold");
        };
        let Ok(actual_bytes) = u64::try_from(captured.len()) else {
            bail!("{file} is larger than a u64 byte count");
        };
        if let Some(expected) = fixture.get("bytes").and_then(Value::as_u64) {
            if expected != actual_bytes {
                bail!("{file}: PROVENANCE.json records {expected} bytes, the capture holds {actual_bytes}");
            }
        }
        if let Some(expected) = fixture.get("sha256").and_then(Value::as_str) {
            let actual = digest(captured);
            if !expected.eq_ignore_ascii_case(&actual) {
                bail!("{file}: PROVENANCE.json records sha256 {expected}, the capture hashes to {actual}");
            }
        }
        verified = verified.saturating_add(1);
    }
    if verified == 0 {
        bail!("PROVENANCE.json lists no fixture: there is nothing to verify");
    }
    Ok(format!(
        "provenance {verified} capture(s) match their recorded bytes and digest"
    ))
}

fn golden(capture: &Capture<'_>, name: &str, body: &str) -> Result<String> {
    let record: Value =
        serde_json::from_str(body).with_context(|| format!("decoding golden_{name}.json"))?;
    let Some(letter) = name.strip_prefix("directory_alpha_") else {
        return Ok(format!(
            "golden_{name} is a prototype record; the adapter's golden test compares it"
        ));
    };
    let capture_file = format!("directory_alpha_{letter}.html");
    let captured = capture
        .corpus
        .get(&capture_file)
        .with_context(|| format!("golden_{name}.json has no {capture_file} capture beside it"))?;
    let rows = parse_directory(captured).with_context(|| format!("parsing {capture_file}"))?;
    let mut parsed: Vec<&str> = rows.iter().map(|row| row.school_id.as_str()).collect();
    parsed.sort_unstable();
    let schools = record
        .get("schools")
        .and_then(Value::as_array)
        .with_context(|| format!("golden_{name}.json carries no `schools` array"))?;
    let mut recorded: Vec<&str> = schools
        .iter()
        .filter_map(|school| school.get("association_id").and_then(Value::as_str))
        .collect();
    recorded.sort_unstable();
    if parsed != recorded {
        bail!("golden_{name}.json and {capture_file} disagree on the school ids: {} recorded, {} parsed", recorded.len(), parsed.len());
    }
    Ok(format!(
        "golden_{name} ids match {capture_file} ({} school(s))",
        parsed.len()
    ))
}

fn digest(body: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(body.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
