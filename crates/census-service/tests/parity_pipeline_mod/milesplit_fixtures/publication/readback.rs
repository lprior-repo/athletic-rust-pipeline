use anyhow::{ensure, Context, Result};
use calamine::{open_workbook, Reader, Xlsx};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

type Row = BTreeMap<String, String>;

pub fn assert_publication(path: &Path) -> Result<()> {
    let directory = path.parent().context("publication directory")?;
    let athletes = csv_rows(&directory.join("athletes.csv"))?;
    let selected: Vec<_> = athletes
        .iter()
        .filter(|row| {
            row.get("display_name")
                .is_some_and(|name| name == "Adelyn Spann")
        })
        .collect();
    ensure!(
        selected.len() == 1,
        "2027 source athlete absent or duplicated"
    );
    let athlete = selected.first().context("selected athlete")?;
    ensure!(field(athlete, "graduation_year")? == "2027");
    ensure!(
        !athletes.iter().any(|row| row
            .get("display_name")
            .is_some_and(|name| name == "Payton Ousley")),
        "2026 source athlete entered 2027 publication"
    );
    let bests = csv_rows(&directory.join("best_marks.csv"))?;
    let owned: Vec<_> = bests
        .iter()
        .filter(|row| row.get("athlete_id") == athlete.get("athlete_id"))
        .collect();
    ensure!(
        owned.len() == 2,
        "owned LJ/TJ did not reach normalized bests"
    );
    let raw: BTreeSet<_> = owned
        .iter()
        .map(|row| field(row, "raw_mark"))
        .collect::<Result<_>>()?;
    ensure!(raw == BTreeSet::from(["13-9", "30-7"]));
    let provenance = csv_rows(&directory.join("provenance.csv"))?;
    for row in owned {
        ensure!(field(row, "meet_date")? == "2026-03-27");
        ensure!(field(row, "capture_reference_status")? == "capture_link_not_retained");
        ensure!(field(row, "meet_name")? == "Troy Invitational #2");
        ensure!(field(row, "source_url")? == owned_url());
        let refs: Vec<String> = serde_json::from_str(field(row, "evidence_refs")?)?;
        ensure!(refs.len() == 2);
        let mut urls = BTreeSet::new();
        for id in refs {
            let fact = provenance
                .iter()
                .find(|fact| fact.get("evidence_id") == Some(&id))
                .context("referenced provenance missing")?;
            ensure!(field(fact, "entity_type")? == "performance");
            ensure!(field(fact, "entity_id")? == field(row, "performance_id")?);
            ensure!(field(fact, "observed_on")? == crate::milesplit_fixtures::replay::ACQUIRED_AT);
            urls.insert(field(fact, "source_url")?.to_string());
            if field(fact, "source_url")? == owned_url() {
                assert_capture_fact(fact)?;
            }
        }
        ensure!(
            urls == BTreeSet::from([
                owned_url(),
                format!("{}/1266814/raw", crate::milesplit_fixtures::TROY_URL)
            ])
        );
    }
    workbook_rows(path)
}

fn workbook_rows(path: &Path) -> Result<()> {
    let mut book: Xlsx<_> = open_workbook(path)?;
    let mut keys = BTreeSet::new();
    for name in book.sheet_names().to_vec() {
        let range = book.worksheet_range(&name)?;
        let mut rows = range.rows();
        let Some(header) = rows.next() else { continue };
        let headers: Vec<_> = header.iter().map(ToString::to_string).collect();
        if !headers.iter().any(|name| name == "Source ResultID") {
            continue;
        }
        for values in rows {
            let row: Row = headers
                .iter()
                .cloned()
                .zip(values.iter().map(ToString::to_string))
                .collect();
            let key = field(&row, "Source ResultID")?;
            if !["milesplit_result:201782263", "milesplit_result:201782277"].contains(&key) {
                continue;
            }
            ensure!(
                keys.insert(key.to_string()),
                "duplicate workbook result {key}"
            );
            ensure!(field(&row, "Athlete")? == "Adelyn Spann");
            ensure!(field(&row, "Graduation Year")? == "2027");
            ensure!(field(&row, "Date")? == "2026-03-27");
            ensure!(field(&row, "Source URL")? == owned_url());
        }
    }
    ensure!(
        keys == BTreeSet::from([
            "milesplit_result:201782263".into(),
            "milesplit_result:201782277".into()
        ]),
        "captured results missing from independent workbook readback"
    );
    Ok(())
}

fn csv_rows(path: &Path) -> Result<Vec<Row>> {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    reader
        .records()
        .map(|record| {
            let record = record?;
            ensure!(record.len() == headers.len(), "malformed normalized row");
            Ok(headers
                .iter()
                .zip(record.iter())
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect())
        })
        .collect()
}

fn field<'a>(row: &'a Row, name: &str) -> Result<&'a str> {
    row.get(name)
        .map(String::as_str)
        .with_context(|| format!("missing column {name}"))
}

fn owned_url() -> String {
    format!(
        "https://al.milesplit.com/api/v1/meets/725218/performances?isMeetPro=0&fields={}",
        census_crawl::milesplit::OWNED_FIELDS
    )
}

fn assert_capture_fact(row: &Row) -> Result<()> {
    use sha2::{Digest, Sha256};
    ensure!(field(row, "capture_status")? == "capture_link_not_retained");
    let fact: serde_json::Value = serde_json::from_str(field(row, "retained_fact")?)?;
    let note: serde_json::Value =
        serde_json::from_str(fact["note"].as_str().context("producer provenance note")?)?;
    ensure!(
        note["sha256"]
            == format!(
                "{:x}",
                Sha256::digest(crate::milesplit_fixtures::replay::OWNED)
            )
    );
    ensure!(note["acquired_at"] == crate::milesplit_fixtures::replay::ACQUIRED_AT);
    ensure!(note["meet_date"] == "2026-03-27");
    ensure!(note["published_grad_year"] == 2027);
    Ok(())
}
