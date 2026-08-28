use crate::model::MatchRecord;
use anyhow::{Context, Result};
use std::{
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
    write_jsonl(&out_dir.join("matches.jsonl"), records)?;
    write_csv(&out_dir.join("matches.csv"), records)?;
    let unresolved: Vec<MatchRecord> = records
        .iter()
        .filter(|record| matches!(record.status.as_str(), "REVIEW" | "NO_MATCH"))
        .cloned()
        .collect();
    write_csv(&out_dir.join("unresolved.csv"), &unresolved)?;
    Ok(())
}

pub fn read_jsonl(path: &Path) -> Result<Vec<MatchRecord>> {
    let reader = BufReader::new(File::open(path)?);
    let mut records = Vec::new();
    for (line_number, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        records.push(serde_json::from_str(&line).with_context(|| {
            format!(
                "invalid match JSON at line {}",
                line_number.saturating_add(1)
            )
        })?);
    }
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

fn write_csv(path: &Path, records: &[MatchRecord]) -> Result<()> {
    let mut writer = csv::Writer::from_path(path)?;
    let mut headers = vec![
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
    ];
    headers.extend(PR_EVENTS.iter().map(|event| format!("{event} PR")));
    headers.extend([
        "All Marks JSON".to_owned(),
        "Candidates JSON".to_owned(),
        "Notes".to_owned(),
        "Hint Count".to_owned(),
        "AI Logic".to_owned(),
    ]);
    writer.write_record(&headers)?;

    for record in records {
        let mut row = vec![
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
        ];
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
