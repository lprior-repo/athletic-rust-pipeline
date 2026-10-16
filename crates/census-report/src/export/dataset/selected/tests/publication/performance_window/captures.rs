use super::TestResult;
use crate::export::qualification::participation::ParticipationReplayV1;
use census_crawl::milesplit::{
    read_retained_owned_result_from, read_retained_result_metadata_from, CaptureReadBudget,
    FrozenRetainedJournal, OwnedAcquisitionRef, RawMetadataArchiveRef, ResultSetRef,
    RetainedJournalBundle, RetainedResultDivision, OWNED_CAPTURE_PHASE,
};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Deserialize)]
struct ReplayJournal {
    store_identity: String,
    snapshot_sequence: u64,
    journal: RetainedJournalBundle,
}

pub(super) fn high_school_results(directory: &Path) -> TestResult {
    let archive: Value =
        serde_json::from_slice(&std::fs::read(directory.join("frozen-input.json"))?)?;
    let digest = archive
        .pointer("/selected_proofs/participation_digest")
        .and_then(Value::as_str)
        .ok_or("anchored participation descriptor")?;
    let replay = ParticipationReplayV1::load(&directory.join("frozen-input.json.proofs"), digest)?;
    let replay: ReplayJournal = serde_json::from_slice(&serde_json::to_vec(&replay)?)?;
    let bytes = replay.journal.encode()?;
    let frozen = FrozenRetainedJournal::from_descriptor(
        &format!("{:x}", Sha256::digest(&bytes)),
        &replay.store_identity,
        replay.snapshot_sequence,
        &bytes,
    )?;
    let journal: Value = serde_json::from_slice(&bytes)?;
    let entries = journal
        .get("entries")
        .and_then(Value::as_array)
        .ok_or("retained participation journal entries")?;
    let mut found = BTreeSet::new();
    for (id, meet, result_set, mark, date, name) in [
        (
            201782263,
            "725218",
            "1266814",
            "13-9",
            "2026-03-27",
            "Troy Invitational #2",
        ),
        (
            201782264,
            "725219",
            "1266815",
            "14-9",
            "2026-04-03",
            "Troy Invitational #3",
        ),
    ] {
        let reference = OwnedAcquisitionRef::parse(manifest_key(
            entries,
            &format!("acquisition/{meet}/"),
            None,
        )?)?;
        let budget = CaptureReadBudget::new(32 * 1024 * 1024)?;
        let owned = read_retained_owned_result_from(&frozen, &reference, "data[0]", budget)?;
        check!(eq; owned.row().result_id, id);
        assert_owned(owned.acquisition().body(), id, meet, result_set, mark)?;
        let url = format!("https://al.milesplit.com/meets/{meet}/results/{result_set}/raw");
        let raw = RawMetadataArchiveRef::parse(manifest_key(entries, "metadata/", Some(&url))?)?;
        let reference = ResultSetRef::parse(&url).ok_or("retained raw result-set URL")?;
        let source = read_retained_result_metadata_from(&frozen, &reference, &raw, &owned, budget)?;
        check!(eq; source.division(), RetainedResultDivision::HighSchool);
        check!(eq; source.result().row().result_id, id);
        check!(eq; source.capture().url, url);
        assert_raw(&source.capture().body, meet, result_set, date, name)?;
        check!(found.insert(id));
    }
    check!(eq; found, BTreeSet::from([201782263, 201782264]));
    Ok(())
}

fn manifest_key<'a>(entries: &'a [Value], prefix: &str, url: Option<&str>) -> TestResult<&'a str> {
    let keys: Vec<_> = entries
        .iter()
        .filter(|entry| entry.get("phase").and_then(Value::as_str) == Some(OWNED_CAPTURE_PHASE))
        .filter(|entry| {
            entry.pointer("/payload/presence").and_then(Value::as_str) == Some("present")
                && url.is_none_or(|url| {
                    entry
                        .pointer("/payload/value/capture/url")
                        .and_then(Value::as_str)
                        == Some(url)
                })
        })
        .filter_map(|entry| entry.get("key").and_then(Value::as_str))
        .filter(|key| key.starts_with(prefix) && key.ends_with("/manifest"))
        .collect();
    check!(eq; keys.len(), 1);
    Ok(*keys.first().ok_or("unique retained capture manifest")?)
}

fn assert_owned(body: &[u8], id: u64, meet: &str, result_set: &str, mark: &str) -> TestResult {
    let value: Value = serde_json::from_slice(body)?;
    let rows = value
        .get("data")
        .and_then(Value::as_array)
        .ok_or("owned rows")?;
    check!(eq; rows.len(), 1);
    let row = rows.first().ok_or("owned result row")?;
    check!(eq; row.get("id").and_then(Value::as_u64), Some(id));
    check!(eq; value.pointer("/_embedded/meet/id").and_then(Value::as_str), Some(meet));
    for (key, expected) in [
        ("athleteId", "14222592"),
        ("teamId", "38332"),
        ("meetId", meet),
        ("meetResultsId", result_set),
        ("divisionName", "High School"),
        ("mark", mark),
    ] {
        check!(eq; row.get(key).and_then(Value::as_str), Some(expected));
    }
    Ok(())
}

fn assert_raw(body: &[u8], meet: &str, result_set: &str, date: &str, name: &str) -> TestResult {
    let expected = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../census-crawl/tests/fixtures/milesplit/troy_725218_rs1266814_raw_projection.html"
    ))
    .replace("725218", meet)
    .replace("1266814", result_set)
    .replace("2026-03-27", date)
    .replace("Troy Invitational #2", name);
    check!(eq; body, expected.as_bytes());
    Ok(())
}
