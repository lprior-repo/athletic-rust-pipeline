use super::super::*;
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use base64::{engine::general_purpose::STANDARD, Engine};
use census_domain::model::Evidence;

pub(super) fn seed_fresh(fetcher: &Fetcher, url: &str, body: &[u8]) -> TestResult<String> {
    let digest = content_digest(body);
    let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", url, ""));
    write_cache(
        &body_path,
        &meta_path,
        body,
        &CacheMeta {
            representation: crate::net::RepresentationHeaders::default(),
            url: url.into(),
            method: "GET".into(),
            status: 200,
            content_digest: digest.clone(),
            bytes: body.len(),
            fetched_at: "2026-10-02T00:00:00Z".into(),
            ..CacheMeta::default()
        },
    )?;
    Ok(digest)
}

pub(super) fn assert_archived(store: &Store, body: &[u8], digest: &str) -> TestResult {
    let rows = store.journal_payloads(crate::milesplit::OWNED_CAPTURE_PHASE)?;
    let mut chunks: Vec<_> = rows
        .iter()
        .filter(|row| row["capture"]["content_digest"] == digest && row.get("raw_base64").is_some())
        .collect();
    chunks.sort_by_key(|row| row["chunk_index"].as_u64());
    let retained =
        chunks
            .into_iter()
            .try_fold(Vec::new(), |mut bytes, row| -> TestResult<Vec<u8>> {
                bytes.extend(STANDARD.decode(row["raw_base64"].as_str().ok_or("retained bytes")?)?);
                Ok(bytes)
            })?;
    check!(eq; retained, body);
    check!(eq; content_digest(&retained), digest);
    Ok(())
}

pub(super) fn assert_owned_year(store: &Store, result: u64, year: i16) -> TestResult {
    let rows = store.journal_payloads(crate::milesplit::OWNED_MEET_PHASE)?;
    check!(rows.iter().any(|row| {
        row["result_id"] == result
            && row["meet_id"] == 725218
            && row["result_set_id"] == 1266814
            && row["source_athlete"]["id"] == "14222592"
            && row["grad_year"] == year
            && row["cohort"] == "published"
    }));
    Ok(())
}

pub(super) fn evidence_notes(evidence: &[Evidence]) -> Vec<serde_json::Value> {
    evidence
        .iter()
        .filter_map(|row| row.note.as_deref())
        .filter_map(|note| serde_json::from_str(note).ok())
        .collect()
}

pub(super) fn assert_canonical_year(
    store: &Store,
    digest: &str,
    result: u64,
    year: i16,
) -> TestResult {
    let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    let year = census_domain::model::GradYear::new(year).ok_or("valid published year")?;
    check!(
        athletes.iter().any(|athlete| {
            athlete
                .source
                .as_ref()
                .is_some_and(|owner| owner.id == "14222592")
                && athlete
                    .published_graduations
                    .iter()
                    .any(|claim| claim.grad_year == year)
                && evidence_notes(&athlete.evidence).iter().any(|note| {
                    note["sha256"] == digest
                        && note["result_id"] == result
                        && note["source_athlete"]["id"] == "14222592"
                        && note["published_grad_year"] == year.get()
                })
        }),
        "new capture must reach canonical published graduation evidence"
    );
    let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
    check!(
        performances.iter().any(|performance| {
            performance.source_key == format!("milesplit_result:{result}")
                && evidence_notes(&performance.evidence).iter().any(|note| {
                    note["sha256"] == digest
                        && note["result_id"] == result
                        && note["source_athlete"]["id"] == "14222592"
                        && note["published_grad_year"] == year.get()
                })
        }),
        "new capture must reach source-owned canonical result evidence"
    );
    Ok(())
}

pub(super) fn projection_receipts(store: &Store) -> TestResult<Vec<serde_json::Value>> {
    Ok(store.journal_payloads(crate::milesplit::RESULT_SET_PHASE)?)
}

pub(super) fn assert_separate_projection(
    store: &Store,
    before_keys: &HashSet<String>,
    before_payloads: &[serde_json::Value],
    digest: &str,
) -> TestResult {
    let keys = store.journal_keys(crate::milesplit::RESULT_SET_PHASE)?;
    check!(
        before_keys.is_subset(&keys),
        "old projection identities remain retained"
    );
    let payloads = projection_receipts(store)?;
    check!(
        before_payloads.iter().all(|prior| payloads.contains(prior)),
        "old dispositions remain immutable"
    );
    let newly_identified: Vec<_> = keys
        .difference(before_keys)
        .map(|key| {
            Ok(store
                .journal_payload(crate::milesplit::RESULT_SET_PHASE, key)?
                .ok_or("identified payload")?)
        })
        .collect::<TestResult<_>>()?;
    check!(
        newly_identified.iter().any(|row| {
            row["disposition"] == "projection_applied"
                && row["meet"] == "725218"
                && row["rsid"] == "1266814"
                && row["capture"]["content_digest"] == digest
        }),
        "changed evidence needs its own explicitly applied projection identity"
    );
    Ok(())
}
