use std::collections::HashMap;
use std::path::Path;
use anyhow::Result;
use crate::model::MatchRecord;
use crate::xlsx;
pub fn summarize(records: &[MatchRecord]) {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for r in records {
        counts
            .entry(r.status.as_str())
            .and_modify(|c| *c = c.saturating_add(1))
            .or_insert(1);
    }
    eprintln!("wrote {} records", records.len());
    for status in ["MATCH", "CLOSE_MATCH", "REVIEW", "NO_MATCH"] {
        eprintln!(
            "  {status}: {}",
            counts.get(status).copied().map_or(0, |c| c)
        );
    }
}

/// Scan and print xlsx stats.
pub fn inspect(input: &Path) -> Result<()> {
    let result = xlsx::scan(input, &[], None)?;
    println!("{}", serde_json::to_string_pretty(&result.stats)?);
    Ok(())
}
