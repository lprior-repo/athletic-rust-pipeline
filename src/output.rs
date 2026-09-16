use crate::model::{ordered_source_headers, MatchRecord};
use anyhow::{Context, Result};
use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
};

const PR_EVENTS: &[&str] = &[
    "100m",
    "200m",
    "400m",
    "800m",
    "1600m",
    "3200m",
    "100h",
    "110h",
    "300h",
    "400h",
    "high_jump",
    "long_jump",
    "triple_jump",
    "pole_vault",
    "shot_put",
    "discus",
    "javelin",
];

pub fn write_all(out_dir: &Path, records: &[MatchRecord]) -> Result<()> {
    validate_records(records)?;
    write_jsonl(&out_dir.join("matches.jsonl"), records)?;
    write_csv(&out_dir.join("matches.csv"), records, |_| true)?;
    write_csv(&out_dir.join("unresolved.csv"), records, |record| {
        record.status != "MATCH"
    })?;
    Ok(())
}

fn validate_records(records: &[MatchRecord]) -> Result<()> {
    records
        .iter()
        .try_fold(HashSet::new(), |mut keys, record| {
            if record.source_key.trim().is_empty() {
                anyhow::bail!("output record has an empty source key");
            }
            if !keys.insert(record.source_key.clone()) {
                anyhow::bail!("output contains duplicate source key {}", record.source_key);
            }
            if record.prospect.source_key != record.source_key {
                anyhow::bail!(
                    "output record metadata mismatch for source key {}",
                    record.source_key
                );
            }
            Ok(keys)
        })?;
    Ok(())
}

pub fn read_jsonl(path: &Path) -> Result<Vec<MatchRecord>> {
    let reader = BufReader::new(File::open(path)?);
    let mut records = Vec::new();
    let mut keys = HashSet::new();
    for (line_number, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let record: MatchRecord = serde_json::from_str(&line).with_context(|| {
            format!(
                "invalid match JSON at line {}",
                line_number.saturating_add(1)
            )
        })?;
        if !keys.insert(record.source_key.clone()) {
            anyhow::bail!(
                "duplicate match source key {} at line {}",
                record.source_key,
                line_number.saturating_add(1)
            );
        }
        records.push(record);
    }
    validate_records(&records)?;
    Ok(records)
}

fn write_jsonl(path: &Path, records: &[MatchRecord]) -> Result<()> {
    let mut writer = BufWriter::new(File::create(path)?);
    for record in records {
        serde_json::to_writer(&mut writer, record)?;
        writer.write_all(b"\n")?;
    }
    writer.flush()?;
    Ok(())
}

fn write_csv(
    path: &Path,
    records: &[MatchRecord],
    include: impl Fn(&MatchRecord) -> bool,
) -> Result<()> {
    let mut writer = csv::Writer::from_path(path)?;
    let source_headers = ordered_source_headers(records);
    let mut headers = source_headers.clone();
    headers.extend([
        "Source Key".to_owned(),
        "Sheet".to_owned(),
        "Excel Row".to_owned(),
        "Prospect Name".to_owned(),
        "Prospect School".to_owned(),
        "Prospect City".to_owned(),
        "Prospect State".to_owned(),
        "Prospect Sport".to_owned(),
        "Status".to_owned(),
        "Score".to_owned(),
        "Athletic Name".to_owned(),
        "Athletic School".to_owned(),
        "Athletic Location".to_owned(),
        "Athletic Profile".to_owned(),
        "Track Confirmed".to_owned(),
        "XC Confirmed".to_owned(),
    ]);
    headers.extend(PR_EVENTS.iter().map(|event| format!("{event} PR")));
    headers.extend([
        "All Marks JSON".to_owned(),
        "Candidates JSON".to_owned(),
        "Notes".to_owned(),
        "Hint Count".to_owned(),
        "AI Logic".to_owned(),
        "Deterministic Decision JSON".to_owned(),
        "Review Mode".to_owned(),
    ]);
    writer.write_record(&headers)?;

    for record in records.iter().filter(|record| include(record)) {
        let mut row = source_headers
            .iter()
            .map(|header| {
                record
                    .prospect
                    .source_fields
                    .get(header)
                    .map_or_else(String::new, Clone::clone)
            })
            .collect::<Vec<_>>();
        row.extend([
            record.source_key.clone(),
            record.prospect.sheet.clone(),
            record.prospect.excel_row.to_string(),
            record.prospect.full_name(),
            record.prospect.school.clone(),
            record.prospect.city.clone(),
            record.prospect.state.clone(),
            record.prospect.sport.clone(),
            record.status.clone(),
            format!("{:.4}", record.score),
            record.selected_name.clone(),
            record.selected_school.clone(),
            record.selected_location.clone(),
            record.selected_profile_url.clone(),
            yes_no(record.track_confirmed).to_owned(),
            yes_no(record.xc_confirmed).to_owned(),
        ]);
        row.extend(PR_EVENTS.iter().map(|event| {
            record
                .best_marks
                .get(*event)
                .map_or_else(String::new, |mark| mark.mark.clone())
        }));
        row.push(serde_json::to_string(&record.best_marks)?);
        row.push(serde_json::to_string(&record.candidates)?);
        row.push(record.notes.clone());
        row.push(record.hint_count.to_string());
        row.push(record.ai_logic.clone());
        row.push(serde_json::to_string(&record.deterministic_decision)?);
        row.push(record.model_decision.model_status.clone());
        writer.write_record(&row)?;
    }
    writer.flush()?;
    Ok(())
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "YES"
    } else {
        "NO"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Prospect;
    use std::collections::BTreeMap;
    use tempfile::tempdir;

    fn record_with_source_fields(status: &str) -> MatchRecord {
        MatchRecord {
            source_key: "Export:2".to_owned(),
            prospect: Prospect {
                source_key: "Export:2".to_owned(),
                sheet: "Export".to_owned(),
                excel_row: 2,
                first_name: "Ada".to_owned(),
                last_name: "Lovelace".to_owned(),
                source_fields: BTreeMap::from([
                    ("Person First".to_owned(), "Ada".to_owned()),
                    ("Person Last".to_owned(), "Lovelace".to_owned()),
                    ("Person Email".to_owned(), "ada@example.test".to_owned()),
                    (
                        "Address Mailing / Permanent Street Combined".to_owned(),
                        "1 Main".to_owned(),
                    ),
                ]),
                ..Default::default()
            },
            status: status.to_owned(),
            ai_logic: "full AI output".to_owned(),
            ..Default::default()
        }
    }

    #[test]
    fn write_all_rejects_duplicate_source_keys_before_writing() -> Result<()> {
        let directory = tempdir()?;
        let record = record_with_source_fields("MATCH");
        assert!(write_all(directory.path(), &[record.clone(), record]).is_err());
        assert!(!directory.path().join("matches.jsonl").exists());
        Ok(())
    }

    #[test]
    fn csv_carries_original_source_columns_before_match_columns() -> Result<()> {
        let directory = tempdir()?;
        write_all(directory.path(), &[record_with_source_fields("MATCH")])?;
        let text = std::fs::read_to_string(directory.path().join("matches.csv"))?;
        let header = text.lines().next().context("missing CSV header")?;
        assert!(header.starts_with(
            "Person First,Person Last,Person Email,Address Mailing / Permanent Street Combined"
        ));
        assert!(text.contains("Ada,Lovelace,ada@example.test,1 Main"));
        assert!(text.contains("full AI output"));
        Ok(())
    }

    #[test]
    fn unresolved_contains_close_match_and_source_address() -> Result<()> {
        let directory = tempdir()?;
        write_all(
            directory.path(),
            &[record_with_source_fields("CLOSE_MATCH")],
        )?;
        let text = std::fs::read_to_string(directory.path().join("unresolved.csv"))?;
        assert!(text.contains("CLOSE_MATCH"));
        assert!(text.contains("1 Main"));
        Ok(())
    }
}
