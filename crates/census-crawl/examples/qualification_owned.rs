use census_crawl::milesplit::{parse_owned_meet, OwnedMeetVerdict};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

fn main() -> Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("pass actual capture path")?;
    let output = std::env::args_os()
        .nth(2)
        .ok_or("pass fresh output JSON path")?;
    let body = read_capture(path)?;
    let digest = format!("{:x}", Sha256::digest(&body));
    if digest != "8db9804ebc2d36b6f7ec1e2a289b2b8ab28610e0063f0ee9f54245eb128071ed" {
        return Err("actual captured body SHA differs".into());
    }
    let summary = provider_summary(&body)?;
    let verdict = parse_owned_meet(&body, 725218);
    let mut rejection_counts = BTreeMap::<String, usize>::new();
    let OwnedMeetVerdict::Parsed(page) = &verdict else {
        return Err(format!("actual capture parse failed: {verdict:?}").into());
    };
    for rejected in &page.rejected {
        increment(
            rejection_counts
                .entry(format!("{:?}", rejected.kind))
                .or_default(),
        )?;
    }
    let owned_target = page
        .rows
        .iter()
        .filter(|row| row.source_athlete.id == "14222592")
        .collect::<Vec<_>>();
    let evidence = serde_json::json!({
        "raw_sha256": digest, "raw_bytes": body.len(), "published_rows": summary.published_rows,
        "resultsets": summary.resultsets, "null_name_rows": summary.null_names,
        "owned_rows": page.rows.len(), "rejected_rows": page.rejected.len(),
        "completeness": page.completeness, "ownership_complete": page.ownership_complete(),
        "rejection_counts": rejection_counts, "target_provider_rows": summary.target,
        "target_owned_rows": owned_target, "rejected": page.rejected,
    });
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output)?;
    std::io::Write::write_all(&mut file, &serde_json::to_vec_pretty(&evidence)?)?;
    println!(
        "raw_bytes={} published={} owned={} rejected={} null_names={} ownership_complete={}",
        body.len(),
        summary.published_rows,
        page.rows.len(),
        page.rejected.len(),
        summary.null_names,
        page.ownership_complete()
    );
    println!(
        "resultsets={:?} rejection_counts={rejection_counts:?}",
        summary.resultsets
    );
    println!("target={}", serde_json::to_string(&owned_target)?);
    Ok(())
}

fn read_capture(path: impl AsRef<std::path::Path>) -> Result<Vec<u8>> {
    let file = std::fs::File::open(path)?;
    if file.metadata()?.len() != 311_763 {
        return Err("actual capture must contain exactly 311763 bytes".into());
    }
    let mut body = Vec::new();
    body.try_reserve_exact(311_763)?;
    file.take(311_764).read_to_end(&mut body)?;
    if body.len() != 311_763 {
        return Err("actual capture changed during its bounded read".into());
    }
    Ok(body)
}

struct ProviderSummary {
    published_rows: usize,
    resultsets: BTreeMap<String, usize>,
    null_names: usize,
    target: Vec<Value>,
}

fn provider_summary(body: &[u8]) -> Result<ProviderSummary> {
    let document: Value = serde_json::from_slice(body)?;
    let data = document
        .get("data")
        .and_then(Value::as_array)
        .ok_or("missing data array")?;
    let mut summary = ProviderSummary {
        published_rows: data.len(),
        resultsets: BTreeMap::new(),
        null_names: 0,
        target: Vec::new(),
    };
    for (index, row) in data.iter().enumerate() {
        let resultset = row
            .get("meetResultsId")
            .ok_or("missing result set")?
            .to_string();
        increment(summary.resultsets.entry(resultset).or_default())?;
        if row.get("firstName") == Some(&Value::Null) || row.get("lastName") == Some(&Value::Null) {
            increment(&mut summary.null_names)?;
        }
        if row.get("athleteId").and_then(Value::as_str) == Some("14222592") {
            summary
                .target
                .push(serde_json::json!({"locator": format!("data[{index}]"), "provider": row}));
        }
    }
    Ok(summary)
}

fn increment(count: &mut usize) -> Result<()> {
    *count = count.checked_add(1).ok_or("row counter overflow")?;
    Ok(())
}
