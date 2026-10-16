use super::TestResult;
use crate::export::qualification::participation::ParticipationReplay;
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

pub(super) fn high_school_results(
    directory: &Path,
    expected: &super::readback::Expected,
) -> TestResult {
    let archive: Value =
        serde_json::from_slice(&std::fs::read(directory.join("frozen-input.json"))?)?;
    let digest = archive
        .pointer("/selected_proofs/participation_digest")
        .and_then(Value::as_str)
        .ok_or("anchored participation descriptor")?;
    let replay = ParticipationReplay::load(&directory.join("frozen-input.json.proofs"), digest)?;
    let independent = replay
        .independent()
        .ok_or("v4 requires the independent replay bundle")?;
    let encoded = serde_json::to_value(independent)?;
    check!(eq; encoded.get("entries"), Some(&serde_json::to_value(&expected.packets)?));
    for reference in expected.packets.keys() {
        let reference =
            census_crawl::participation_evidence::ParticipationEvidenceRefV1::parse(reference)?;
        let proof = independent
            .read(&reference)?
            .ok_or("retained exact-day source packet")?;
        check!(eq; proof.subject().name(), "Synthetic Participant");
        check!(eq; proof.subject().school(), "Detroit Catholic Central");
        check!(
            matches!(proof.ownership(), census_crawl::participation_evidence::SubjectOwnershipV1::ProviderLink(person)
            if person.id == "14222592")
        );
    }
    let replay: ReplayJournal = serde_json::from_slice(&serde_json::to_vec(replay.retained())?)?;
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
    for case in &super::fixture::CASES {
        let super::fixture::ResultCase {
            id,
            meet,
            result_set,
            mark,
            ..
        } = case;
        let reference = OwnedAcquisitionRef::parse(manifest_key(
            entries,
            &format!("acquisition/{meet}/"),
            None,
        )?)?;
        let budget = CaptureReadBudget::new(32 * 1024 * 1024)?;
        let owned = read_retained_owned_result_from(&frozen, &reference, "data[0]", budget)?;
        check!(eq; owned.row().result_id, *id);
        assert_owned(owned.acquisition().body(), *id, meet, result_set, mark)?;
        let url = format!("https://mi.milesplit.com/meets/{meet}/results/{result_set}/raw");
        let raw = RawMetadataArchiveRef::parse(manifest_key(entries, "metadata/", Some(&url))?)?;
        let reference = ResultSetRef::parse(&url).ok_or("retained raw result-set URL")?;
        let source = read_retained_result_metadata_from(&frozen, &reference, &raw, &owned, budget)?;
        check!(eq; source.division(), RetainedResultDivision::HighSchool);
        check!(eq; source.result().row().result_id, *id);
        check!(eq; source.capture().url, url);
        check!(eq; source.capture().body.as_slice(), super::fixture::raw_body(case).as_bytes());
        check!(found.insert(*id));
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
