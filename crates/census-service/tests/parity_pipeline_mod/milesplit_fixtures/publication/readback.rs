use anyhow::{ensure, Context, Result};
use calamine::{open_workbook, Reader, Xlsx};
use std::collections::BTreeSet;
use std::path::Path;

#[path = "../../../common/csv_readback.rs"]
mod csv_readback;
use csv_readback::{csv_rows, field, Row};

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
            assert_capture_time(field(fact, "source_url")?, field(fact, "observed_on")?)?;
            urls.insert(field(fact, "source_url")?.to_string());
            if field(fact, "source_url")? == owned_url() {
                assert_capture_fact(fact)?;
            } else {
                assert_raw_metadata_fact(fact)?;
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

fn raw_capture(url: &str) -> Result<(&'static str, &'static str, &'static str)> {
    crate::milesplit_fixtures::replay::RAW_CAPTURES
        .into_iter()
        .find(|(_, rsid, _)| url == format!("{}/{rsid}/raw", crate::milesplit_fixtures::TROY_URL))
        .context("unknown raw fixture capture URL")
}

fn assert_capture_time(url: &str, observed_on: &str) -> Result<()> {
    let expected = if url == owned_url() {
        crate::milesplit_fixtures::replay::ACQUIRED_AT
    } else {
        raw_capture(url)?.2
    };
    ensure!(
        observed_on == expected,
        "capture-owned time mismatch for {url}: expected {expected}, got {observed_on}"
    );
    Ok(())
}

fn assert_raw_metadata_fact(row: &Row) -> Result<()> {
    use sha2::{Digest, Sha256};
    let url = field(row, "source_url")?;
    let (file, rsid, captured_at) = raw_capture(url)?;
    let body = crate::common::fixture("milesplit", file)?;
    let fact: serde_json::Value = serde_json::from_str(field(row, "retained_fact")?)?;
    let note: serde_json::Value =
        serde_json::from_str(fact["note"].as_str().context("raw metadata note")?)?;
    ensure!(fact["observed_on"] == captured_at);
    ensure!(note["role"] == "raw result file meet/season metadata only");
    ensure!(note["result_set_id"] == rsid);
    ensure!(note["meet_date"] == "2026-03-27");
    let capture = &note["raw_metadata_capture"];
    ensure!(capture["url"] == url);
    ensure!(capture["fetched_at"] == captured_at);
    ensure!(capture["content_digest"] == format!("{:x}", Sha256::digest(body.as_bytes())));
    ensure!(capture["bytes"] == body.len());
    ensure!(capture["method"] == "GET" && capture["status"] == 200);
    let owned = &note["owned_capture"];
    ensure!(owned["url"] == owned_url());
    ensure!(owned["fetched_at"] == crate::milesplit_fixtures::replay::ACQUIRED_AT);
    ensure!(
        owned["content_digest"]
            == format!(
                "{:x}",
                Sha256::digest(crate::milesplit_fixtures::replay::OWNED)
            )
    );
    Ok(())
}

#[test]
fn publication_capture_clock_oracle_rejects_cross_source_and_meet_dates() -> Result<()> {
    let api = owned_url();
    let raw = format!("{}/1266814/raw", crate::milesplit_fixtures::TROY_URL);
    let other_raw = format!("{}/1266815/raw", crate::milesplit_fixtures::TROY_URL);
    assert_capture_time(&api, "2026-10-01T23:44:16Z")?;
    assert_capture_time(&raw, "2026-09-28T10:11:24Z")?;
    assert_capture_time(&other_raw, "2026-09-28T10:11:13Z")?;
    for (url, wrong) in [
        (&api, "2026-09-28T10:11:24Z"),
        (&raw, "2026-10-01T23:44:16Z"),
        (&raw, "2026-09-28T10:11:13Z"),
        (&other_raw, "2026-09-28T10:11:24Z"),
        (&raw, "2026-03-27"),
        (&api, "2026-03-27"),
    ] {
        let error = assert_capture_time(url, wrong)
            .err()
            .context("wrong source clock accepted")?;
        ensure!(error.to_string().contains("capture-owned time mismatch"));
    }
    let error = assert_capture_time("https://example.test/unowned", "2026-09-28T10:11:24Z")
        .err()
        .context("unknown source accepted")?;
    ensure!(error
        .to_string()
        .contains("unknown raw fixture capture URL"));
    Ok(())
}
