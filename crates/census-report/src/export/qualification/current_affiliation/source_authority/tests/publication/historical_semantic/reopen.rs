use super::super::{omission, retention::files};
use super::*;
use census_store::{RosterSourceIssuanceStoreError, StoreError};

#[test]
fn controlled_rekeyed_semantic_object_public_reopen_preserves_original_review_and_publication(
) -> TestResult {
    let (fixture, expected) = setup::prepare(Case::Resolved)?;
    let objects = tempfile::tempdir()?;
    let original = tempfile::tempdir()?;
    let candidate = tempfile::tempdir()?;
    let output = tempfile::tempdir()?;
    let dataset = capture(&fixture, objects.path())?;
    witness(&dataset, &expected)?;
    let frozen = original.path().join("authentic.json");
    dataset.save_frozen(&frozen)?;
    let digest = dataset.archive_digest()?;
    let authentic = files::inventory(original.path())?;
    let root = output.path().join("publication");
    let workbook = publish(&fixture, &dataset, &root)?;
    artifacts::readback(&workbook, &dataset, &expected)?;
    let pointer = std::fs::read_link(root.join("current"))?;
    let generation = workbook.parent().ok_or("valid generation")?;
    let retained = files::inventory(generation)?;
    let verified = published::verify_published(&workbook)?.generation_digest;
    let hostile = candidate.path().join("attacker-transport.json");
    dataset.save_frozen(&hostile)?;
    let transport =
        omission::rewrite_review(&hostile, rekey_semantic, "review closure object absent")?;
    check!(ne; transport, digest);
    refuse(ExportDataset::reopen_frozen_with_digest(
        &hostile, &transport,
    ))?;
    let admitted = candidate.path().join("admitted.json");
    refuse(omission::admit_save_publish(
        &fixture, &hostile, &transport, &admitted, &root,
    ))?;
    check!(!admitted.exists());
    check!(eq; std::fs::read_link(root.join("current"))?, pointer);
    check!(eq; files::inventory(generation)?, retained);
    check!(eq; files::inventory(original.path())?, authentic);
    check!(eq; published::verify_published(&workbook)?.generation_digest, verified);
    let restored = ExportDataset::reopen_frozen_with_digest(&frozen, &digest)?;
    witness(&restored, &expected)?;
    check!(eq; publish(&fixture, &restored, &root)?, workbook);
    artifacts::readback(&workbook, &restored, &expected)?;
    check!(eq; files::inventory(generation)?, retained);
    check!(eq; published::verify_published(&workbook)?.generation_digest, verified);
    Ok(())
}

fn refuse<T>(result: crate::report::ReportResult<T>) -> TestResult {
    check!(matches!(
        result,
        Err(crate::report::ReportError::Store(
            StoreError::RosterSourceIssuance(RosterSourceIssuanceStoreError::Corrupt(
                "review closure object absent"
            ))
        ))
    ));
    Ok(())
}

fn rekey_semantic(original: &[u8]) -> TestResult<Vec<u8>> {
    check!(original.len() <= 4 * 1024 * 1024);
    let mut archive: omission::wire::Archive = serde_json::from_slice(original)?;
    check!(eq; serde_json::to_vec(&archive)?, original);
    check!(archive.records.len() <= 4096);
    let mut original_value = serde_json::to_value(&archive)?;
    original_value
        .as_object_mut()
        .ok_or("archive object")?
        .remove("records");
    let reviews = archive
        .records
        .iter()
        .filter(|(key, _)| key.starts_with(b"roster_source_issuance/v1/reviews/"))
        .collect::<Vec<_>>();
    check!(eq; reviews.len(), 1);
    let review: Value = serde_json::from_slice(&reviews.first().ok_or("original review")?.1)?;
    let proposal_bytes: Vec<u8> = serde_json::from_value(
        review
            .pointer("/basis/proposal")
            .ok_or("original proposal")?
            .clone(),
    )?;
    let proposal: Value = serde_json::from_slice(&proposal_bytes)?;
    let original_digest = proposal
        .pointer("/capture/interpretation/digest")
        .and_then(Value::as_str)
        .ok_or("original interpretation reference")?;
    let old_key = format!("roster_source_issuance/v1/objects/{original_digest}").into_bytes();
    let index = archive
        .records
        .iter()
        .position(|(key, _)| key == &old_key)
        .ok_or("original semantic object")?;
    let (key, bytes) = archive.records.get(index).ok_or("semantic record")?;
    check!(eq; files::digest(bytes), original_digest);
    let unaffected = archive
        .records
        .iter()
        .filter(|(candidate, _)| candidate != key)
        .cloned()
        .collect::<Vec<_>>();
    let changed = wire::mutate(bytes, Mutation::OmitMember)?;
    let changed_digest = files::digest(&changed);
    check!(ne; changed_digest, original_digest);
    let new_key = format!("roster_source_issuance/v1/objects/{changed_digest}").into_bytes();
    check!(!archive.records.iter().any(|(key, _)| key == &new_key));
    archive.records[index] = (new_key.clone(), changed);
    archive.records.sort_by(|left, right| left.0.cmp(&right.0));
    check!(!archive.records.iter().any(|(key, _)| key == &old_key));
    check!(eq; archive.records.iter().filter(|(key, _)| key != &new_key).cloned().collect::<Vec<_>>(), unaffected);
    let mut changed_value = serde_json::to_value(&archive)?;
    changed_value
        .as_object_mut()
        .ok_or("changed archive object")?
        .remove("records");
    check!(eq; changed_value, original_value);
    Ok(serde_json::to_vec(&archive)?)
}
