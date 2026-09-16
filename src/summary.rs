use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;

use crate::model::MatchRecord;
use crate::xlsx;

pub fn summarize(records: &[MatchRecord]) {
    let counts = records
        .iter()
        .fold(BTreeMap::<&str, usize>::new(), |mut counts, record| {
            counts
                .entry(record.status.as_str())
                .and_modify(|count| *count = count.saturating_add(1))
                .or_insert(1);
            counts
        });
    eprintln!("wrote {} records", records.len());
    counts
        .into_iter()
        .for_each(|(status, count)| eprintln!("  {status}: {count}"));
}

/// Scan and print xlsx stats.
pub fn inspect(input: &Path) -> Result<()> {
    let result = xlsx::scan(input, xlsx::ScanMode::Sports(Vec::new()), None)?;
    println!("{}", serde_json::to_string_pretty(&result.stats)?);
    Ok(())
}
