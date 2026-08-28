use crate::model::MatchRecord;
use anyhow::{Context, Result};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
};

pub fn load_latest(path: &Path) -> Result<HashMap<String, MatchRecord>> {
    let mut records = HashMap::new();
    if !path.exists() {
        return Ok(records);
    }
    let reader = BufReader::new(File::open(path)?);
    for (line_number, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let record: MatchRecord = serde_json::from_str(&line).with_context(|| {
            format!(
                "invalid checkpoint JSON at line {}",
                line_number.saturating_add(1)
            )
        })?;
        records.insert(record.source_key.clone(), record);
    }
    Ok(records)
}

pub fn append(path: &Path, record: &MatchRecord) -> Result<()> {
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(&mut writer, record)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}
