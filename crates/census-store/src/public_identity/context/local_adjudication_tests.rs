use super::reviewed_fixture::{binding_id, ready_for_manual as ready_for_merge, ReviewedFixture};
use crate::public_identity::{local_adjudication, PublicIdentityError};
use crate::public_identity_tests::support::TestResult;
use crate::StoreError;
use census_domain::public_identity_history::local_adjudication::{
    ClaimTreatmentV1, LocalIdentitySplitV1, RetainedClaimDispositionV1, SupersededMergeV1,
};
use census_domain::public_identity_history::{BindingPartition, PublicResolution};
use sha2::{Digest, Sha256};

fn request(fixture: &ReviewedFixture) -> TestResult<LocalIdentitySplitV1> {
    let canonical = fixture.births.first().ok_or("first birth")?.public_id();
    let snapshot = fixture
        .store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("accepted snapshot")?;
    let prior = snapshot.history_records().iter().rev().find(|row| matches!(&row.request,
        census_domain::public_identity_history::HistoryRequest::Merge { canonical: owner, .. } if owner == canonical)).ok_or("prior reviewed merge")?;
    let inventory = fixture.writer.inspect_local_adjudication(
        &fixture.store,
        "manual-request-inventory",
        canonical,
    )?;
    let dispositions = inventory
        .claims()
        .iter()
        .map(|claim| RetainedClaimDispositionV1 {
            claim: claim.clone(),
            treatment: if claim.claim_path.starts_with("/source_links/")
                || claim.claim_path.starts_with("/identity_attestations/")
            {
                ClaimTreatmentV1::RejectIdentityCorroboration
            } else {
                ClaimTreatmentV1::RetainAsContext
            },
            rationale:
                "Synthetic local review: published link combined two separately retained athletes"
                    .into(),
        })
        .collect();
    let partitions = fixture
        .candidates
        .iter()
        .zip(&fixture.births)
        .take(2)
        .map(|(candidate, birth)| {
            Ok(BindingPartition {
                representative: candidate.clone(),
                binding_ids: vec![binding_id(&fixture.store, birth.head().sequence)?],
            })
        })
        .collect::<TestResult<Vec<_>>>()?;
    Ok(LocalIdentitySplitV1 {
        case_id: fixture.request.case_id().into(), canonical: canonical.clone(),
        supersedes: vec![SupersededMergeV1 { sequence: prior.sequence, digest: prior.digest.clone() }],
        partitions, unresolved_bindings: Vec::new(),
        affected_record_keys: inventory.affected_record_keys().to_vec(), dispositions,
        rationale: "Synthetic reviewed correction retains original source facts and reverses their identity interpretation".into(),
    })
}

fn merged() -> TestResult<ReviewedFixture> {
    let fixture = ready_for_merge()?;
    results::install(&fixture)?;
    fixture.writer.apply_reviewed_identity(
        &fixture.store,
        "synthetic-manual-prior-merge",
        &fixture.request,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    Ok(fixture)
}

fn approval(
    prepared: &local_adjudication::PreparedLocalAdjudicationV1<'_>,
) -> TestResult<(Vec<u8>, String)> {
    Ok((prepared.approval_bytes()?, prepared.approval_digest()?))
}

#[test]
fn genuine_merge_can_prepare_and_consume_explicit_manual_split_without_erasing_claims() -> TestResult
{
    let fixture = merged()?;
    let before = results::retained_facts(&fixture.store)?;
    let request = request(&fixture)?;
    let head = fixture.writer.head(&fixture.store)?;
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "synthetic-manual-split",
        request,
        fixture.store.snapshot().revision(),
        &head,
    )?;
    let (bytes, digest) = approval(&prepared)?;
    let old = fixture.births.first().ok_or("birth")?.public_id();
    let outcome =
        fixture
            .writer
            .apply_local_adjudication(&fixture.store, prepared, bytes, digest)?;
    let accepted = fixture
        .store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("snapshot")?;
    let PublicResolution::Ambiguous { successors, .. } = accepted.resolve_public(old)? else {
        return Err("manual split must make old birth ambiguous".into());
    };
    check!(eq; successors.iter().map(|value| value.public_id.clone()).collect::<Vec<_>>(), outcome.introduced_public_ids());
    check!(eq; outcome.introduced_public_ids().len(), 2);
    check!(eq; outcome.head().sequence, head.sequence.checked_add(1).ok_or("head overflow")?);
    check!(eq; fixture.writer.head(&fixture.store)?, *outcome.head());
    check!(eq; results::retained_facts(&fixture.store)?, before);
    Ok(())
}

#[test]
fn omitted_retained_claim_cannot_prepare_a_manual_correction() -> TestResult {
    let fixture = merged()?;
    let mut request = request(&fixture)?;
    request.dispositions.pop().ok_or("claim")?;
    stage_inventory(&fixture, "synthetic-manual-omission")?;
    let before = crate::public_identity_tests::support::retained_state(&fixture.store)?;
    for _ in 0..2 {
        let result = fixture.writer.prepare_local_adjudication(
            &fixture.store,
            "synthetic-manual-omission",
            request.clone(),
            fixture.store.snapshot().revision(),
            &fixture.writer.head(&fixture.store)?,
        );
        check!(matches!(
            result,
            Err(StoreError::PublicIdentity(PublicIdentityError::History(
                census_domain::public_identity_history::HistoryError::Invalid(
                    "manual adjudication inventory"
                )
            )))
        ));
        check!(eq; crate::public_identity_tests::support::retained_state(&fixture.store)?, before);
    }
    Ok(())
}

#[test]
fn changed_approval_bytes_never_reach_the_atomic_commit_hook() -> TestResult {
    let fixture = merged()?;
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "synthetic-manual-corrupt",
        request(&fixture)?,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let (mut bytes, digest) = approval(&prepared)?;
    bytes.push(b' ');
    let before = crate::public_identity_tests::support::retained_state(&fixture.store)?;
    let result = fixture
        .writer
        .apply_local_adjudication(&fixture.store, prepared, bytes, digest);
    check!(matches!(
        result,
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "manual graph approval binding"
        )))
    ));
    check!(eq; crate::public_identity_tests::support::retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn changed_store_revision_after_approval_cannot_reach_commit() -> TestResult {
    let fixture = merged()?;
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "synthetic-manual-stale",
        request(&fixture)?,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let (bytes, digest) = approval(&prepared)?;
    let extra = crate::public_identity_tests::support::subject("manual-new-evidence", "9991")?;
    fixture
        .store
        .append_many(crate::Table::Athletes, &[extra])?;
    let before = crate::public_identity_tests::support::retained_state(&fixture.store)?;
    let result = fixture
        .writer
        .apply_local_adjudication(&fixture.store, prepared, bytes, digest);
    check!(matches!(result, Err(StoreError::RevisionMoved { .. })));
    check!(eq; crate::public_identity_tests::support::retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn forged_retained_claim_value_cannot_prepare_even_when_its_digest_is_recomputed() -> TestResult {
    let fixture = merged()?;
    let mut request = request(&fixture)?;
    let claim = &mut request.dispositions.first_mut().ok_or("claim")?.claim;
    claim.retained_value = serde_json::json!("invented fact");
    claim.claim_digest = census_domain::model::serialized_digest(&claim.retained_value)?;
    stage_inventory(&fixture, "synthetic-manual-forged")?;
    let before = crate::public_identity_tests::support::retained_state(&fixture.store)?;
    for _ in 0..2 {
        let result = fixture.writer.prepare_local_adjudication(
            &fixture.store,
            "synthetic-manual-forged",
            request.clone(),
            fixture.store.snapshot().revision(),
            &fixture.writer.head(&fixture.store)?,
        );
        check!(matches!(
            result,
            Err(StoreError::PublicIdentity(PublicIdentityError::History(
                census_domain::public_identity_history::HistoryError::EvidenceMismatch
            )))
        ));
        check!(eq; crate::public_identity_tests::support::retained_state(&fixture.store)?, before);
    }
    Ok(())
}

mod affected;
mod affected_replay;
mod durable;
mod five_family;
mod frozen_refusals;
mod results;
mod retained_validation;

mod admission;

fn stage_inventory(
    fixture: &ReviewedFixture,
    operation: &str,
) -> TestResult<crate::LocalAdjudicationInventoryV1> {
    let store = &fixture.store;
    let writer = &fixture.writer;
    let canonical = fixture.births.first().ok_or("birth")?.public_id();
    let raw_operation =
        crate::public_identity::graph_operation::raw_operation(writer.store_identity(), operation)?;
    let key = format!(
        "manual-context-admission-v1\0reservation\0{}",
        crate::journal::object_digest(raw_operation.as_str().as_bytes())
    );
    if store.journal.get(&key)?.is_some() {
        let before = crate::public_identity_tests::support::retained_state(store)?;
        let inventory = writer.inspect_local_adjudication(store, operation, canonical)?;
        check!(eq; crate::public_identity_tests::support::retained_state(store)?, before);
        return Ok(inventory);
    }
    let before = crate::public_identity_tests::support::retained_state(store)?;
    let inventory = writer.inspect_local_adjudication(store, operation, canonical)?;
    let after = crate::public_identity_tests::support::retained_state(store)?;
    let raw_operation =
        crate::public_identity::graph_operation::raw_operation(writer.store_identity(), operation)?;
    let reservation_key = format!(
        "manual-context-admission-v1\0reservation\0{}",
        crate::journal::object_digest(raw_operation.as_str().as_bytes())
    );
    let reservation = store
        .journal
        .get(&reservation_key)?
        .ok_or("graph reservation")?;
    let reservation: serde_json::Value = serde_json::from_slice(&reservation)?;
    check!(eq; reservation.get("version").and_then(serde_json::Value::as_u64), Some(3));
    let allowed = staged_graph_keys(&reservation, &reservation_key)?;
    check!(eq; before.len(), after.len());
    for ((before_name, before_rows), (after_name, after_rows)) in before.iter().zip(&after) {
        check!(eq; before_name, after_name);
        if before_name != crate::JOURNAL {
            check!(eq; before_rows, after_rows);
            continue;
        }
        let header_key = b"manual-context-admission-v1\0header";
        for (key, value) in before_rows {
            let (_, current) = after_rows
                .iter()
                .find(|(held, _)| held == key)
                .ok_or("inspection removed a retained row")?;
            if key.as_slice() != header_key {
                check!(eq; current, value);
            }
        }
        let added: Vec<_> = after_rows
            .iter()
            .filter(|(key, _)| !before_rows.iter().any(|(prior, _)| prior == key))
            .collect();
        check!(added.iter().all(|(key, _)| allowed.contains(key)));
        check!(added
            .iter()
            .any(|(key, _)| key == reservation_key.as_bytes()));
        let bytes = added
            .iter()
            .try_fold(0_u64, |total, (key, value)| -> TestResult<u64> {
                let row = key
                    .len()
                    .checked_add(value.len())
                    .ok_or("row charge overflow")?;
                Ok(total
                    .checked_add(u64::try_from(row)?)
                    .ok_or("charge overflow")?)
            })?;
        let roots = u64::try_from(
            added
                .iter()
                .filter(|(key, _)| key.starts_with(b"j2\0manifest\0"))
                .count(),
        )?;
        let header = |rows: &[(Vec<u8>, Vec<u8>)]| -> TestResult<serde_json::Value> {
            let (_, raw) = rows
                .iter()
                .find(|(key, _)| key.as_slice() == header_key)
                .ok_or("admission header")?;
            Ok(serde_json::from_slice(raw)?)
        };
        let mut expected_header = header(before_rows)?;
        for (field, increment) in [("bytes", bytes), ("roots", roots), ("reservations", 1)] {
            let count = expected_header
                .get(field)
                .and_then(serde_json::Value::as_u64)
                .ok_or("admission counter")?;
            *expected_header.get_mut(field).ok_or("admission counter")? =
                serde_json::Value::from(count.checked_add(increment).ok_or("counter overflow")?);
        }
        check!(eq; header(after_rows)?, expected_header);
    }
    let sequence = store.snapshot().sequence();
    let repeated = writer.inspect_local_adjudication(store, operation, canonical)?;
    check!(eq; repeated.graph(), inventory.graph());
    check!(eq; repeated.claims(), inventory.claims());
    check!(eq; repeated.affected_record_keys(), inventory.affected_record_keys());
    check!(eq; crate::public_identity_tests::support::retained_state(store)?, after);
    check!(eq; store.snapshot().sequence(), sequence);
    Ok(inventory)
}

fn staged_graph_keys(
    reservation: &serde_json::Value,
    reservation_key: &str,
) -> TestResult<std::collections::BTreeSet<Vec<u8>>> {
    let descriptors = reservation
        .get("descriptors")
        .and_then(serde_json::Value::as_array)
        .ok_or("graph descriptors")?;
    let mut keys = std::collections::BTreeSet::from([reservation_key.as_bytes().to_vec()]);
    for descriptor in descriptors {
        let physical = descriptor.get("physical").ok_or("physical descriptor")?;
        let root = physical
            .get("root")
            .and_then(serde_json::Value::as_str)
            .ok_or("physical root")?;
        keys.insert(crate::journal::object_manifest_key(root)?);
        for (field, chunk) in [("chunks", true), ("pages", false)] {
            let rows = physical
                .get(field)
                .and_then(serde_json::Value::as_array)
                .ok_or("physical rows")?;
            for ordinal in 0..rows.len() {
                keys.insert(if chunk {
                    crate::journal::object_chunk_key(root, ordinal)?
                } else {
                    crate::journal::object_index_key(root, ordinal)?
                });
            }
        }
    }
    Ok(keys)
}

fn committed_state(store: &crate::Store) -> TestResult<serde_json::Value> {
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("accepted snapshot")?;
    Ok(serde_json::json!({
        "identity": store.store_identity()?,
        "head": accepted.head(),
        "history": accepted.history_records(),
        "tables": snapshot.tables_digest(&crate::Table::ALL)?,
        "counts": snapshot.appended_counts()?,
        "evidence": snapshot.evidence_generation(),
        "derived": snapshot.derived_generation(),
    }))
}

fn admission_counts(store: &crate::Store) -> TestResult<(u64, u64, u64)> {
    let raw = store
        .journal
        .get("manual-context-admission-v1\0header")?
        .ok_or("admission header")?;
    let header: serde_json::Value = serde_json::from_slice(&raw)?;
    Ok((
        header
            .get("bytes")
            .and_then(serde_json::Value::as_u64)
            .ok_or("bytes")?,
        header
            .get("roots")
            .and_then(serde_json::Value::as_u64)
            .ok_or("roots")?,
        header
            .get("reservations")
            .and_then(serde_json::Value::as_u64)
            .ok_or("reservations")?,
    ))
}
