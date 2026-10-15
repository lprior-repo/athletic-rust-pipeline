use super::reviewed_fixture::{binding_id, ready_for_merge, review, ReviewedFixture};
use super::test_fixture::read_inputs;
use super::Inputs;
use crate::public_identity_tests::support::{retained_state, TestResult};
use crate::{PublicIdentityError, ReviewedIdentityRequestV1, Store, StoreError, Table};
use census_domain::model::{ReviewState, SourceIdentity};
use census_domain::public_identity_history::{
    BindingPartition, BindingResolution, EstablishmentOrdinal, PublicAthleteId, PublicResolution,
    ResolvedSuccessor,
};
use sha2::{Digest, Sha256};

#[test]
fn reviewed_merge_keeps_public_and_binding_consumers_consistent_across_snapshot_and_reopen(
) -> TestResult {
    let ReviewedFixture {
        directory,
        store,
        writer,
        candidates,
        births,
        request,
        capture_bodies,
        pending_case,
    } = ready_for_merge()?;
    let [first_candidate, second_candidate, unrelated_candidate] = &candidates;
    let [first_birth, second_birth, unrelated_birth] = &births;
    let first_binding = binding_id(&store, 1)?;
    let second_binding = binding_id(&store, 2)?;
    let original_material = source_material(&store)?;
    let held = store.snapshot();
    let original = held
        .accepted_public_identity_snapshot()?
        .ok_or("original snapshot")?;
    let first = ResolvedSuccessor {
        public_id: first_birth.public_id().clone(),
        representative_candidate: first_candidate.clone(),
        established: EstablishmentOrdinal {
            sequence: 1,
            position: 0,
        },
    };
    let second = PublicResolution::Canonical(ResolvedSuccessor {
        public_id: second_birth.public_id().clone(),
        representative_candidate: second_candidate.clone(),
        established: EstablishmentOrdinal {
            sequence: 2,
            position: 0,
        },
    });
    let unrelated = PublicResolution::Canonical(ResolvedSuccessor {
        public_id: unrelated_birth.public_id().clone(),
        representative_candidate: unrelated_candidate.clone(),
        established: EstablishmentOrdinal {
            sequence: 3,
            position: 0,
        },
    });
    check!(eq; original.resolve_public(second_birth.public_id())?, second);
    let merged = writer.apply_reviewed_identity(
        &store,
        "reviewed-merge",
        &request,
        store.snapshot().revision(),
        unrelated_birth.head(),
    )?;
    check!(eq; merged.head().sequence, 4);
    check!(eq; merged.introduced_public_ids(), &[]);
    check!(eq; merged.replayed(), false);
    let redirect = PublicResolution::Redirect {
        successor: first.clone(),
        changed_at: 4,
    };
    let current = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("merged snapshot")?;
    check!(eq; current.resolve_public(first_birth.public_id())?, PublicResolution::Canonical(first.clone()));
    check!(eq; current.resolve_public(second_birth.public_id())?, redirect);
    check!(eq; current.resolve_candidate(second_candidate)?, redirect);
    check!(eq; current.resolve_binding(&second_binding)?, BindingResolution::Resolved {
        identity: redirect.clone(), established_at: 2, superseded_by: None,
    });
    check!(eq; current.resolve_binding(&first_binding)?, BindingResolution::Resolved {
        identity: PublicResolution::Canonical(first.clone()), established_at: 1, superseded_by: None,
    });
    check!(eq; current.resolve_public(unrelated_birth.public_id())?, unrelated);
    check!(eq; current.resolve_candidate(unrelated_candidate)?, unrelated);
    check!(eq; original.resolve_public(second_birth.public_id())?, second);
    check!(eq; original.resolve_candidate(second_candidate)?, second);
    check!(eq; held.accepted_public_identity_snapshot()?.ok_or("held snapshot")?.resolve_public(second_birth.public_id())?, second);
    check!(eq; original.head(), unrelated_birth.head());
    check!(eq; current.head(), merged.head());
    for birth in &births {
        check!(eq; current.correction_history(birth.public_id()), Some([].as_slice()));
    }
    check!(eq; current.affected_record_keys(4), None);
    check!(eq; current.resolve_binding(second_candidate.as_str())?, BindingResolution::Unknown);
    let unknown = PublicAthleteId::parse(&format!("ath_public_v1_{}", "0".repeat(64)))?;
    check!(eq; current.resolve_public(&unknown)?, PublicResolution::Unknown);
    check!(eq; current.correction_history(&unknown), None);
    let root = reviewed_root(&store, 4)?;
    let retained = read_inputs(&store, &root)?;
    check!(eq; retained.athletes.len(), 4);
    check!(eq; retained.observations.len(), 1);
    check!(eq; retained.cases.len(), 2);
    check!(eq; retained.verdicts.len(), 2);
    check!(eq; retained.cases.iter().find(|case| case.id == pending_case.id), Some(&pending_case));
    let raw_first = retained
        .athletes
        .iter()
        .find(|athlete| athlete.id.as_str() == first_candidate.as_str())
        .ok_or("raw first athlete")?;
    check!(eq; raw_first.observed_grades.len(), 0);
    verify_capture_bodies(&retained, &capture_bodies)?;
    check!(eq; source_material(&store)?, original_material);
    let before = retained_state(&store)?;
    drop(held);
    drop(writer);
    drop(store);
    let (store, writer) =
        Store::open_public_identity_qualification_v2(directory.path().join("identity"))?
            .into_parts();
    let reopened = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("reopened snapshot")?;
    check!(eq; reopened.head(), merged.head());
    check!(eq; reopened.resolve_public(second_birth.public_id())?, redirect);
    check!(eq; reopened.resolve_candidate(second_candidate)?, redirect);
    check!(eq; reopened.resolve_binding(&second_binding)?, current.resolve_binding(&second_binding)?);
    check!(eq; reopened.resolve_public(unrelated_birth.public_id())?, unrelated);
    check!(eq; original.resolve_public(second_birth.public_id())?, second);
    let repeated_birth = writer.establish_source_bound(
        &store,
        "reviewed-second-birth",
        second_candidate,
        store.snapshot().revision(),
        merged.head(),
    )?;
    check!(eq; repeated_birth.replayed(), true);
    check!(eq; repeated_birth.public_id(), second_birth.public_id());
    check!(eq; repeated_birth.head(), second_birth.head());
    check!(eq; source_material(&store)?, original_material);
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn retained_reviewed_merge_replays_when_current_review_is_superseded_after_reopen() -> TestResult {
    let ReviewedFixture {
        directory,
        store,
        writer,
        candidates,
        births,
        request,
        ..
    } = ready_for_merge()?;
    let [first_birth, second_birth, unrelated_birth] = &births;
    let merged = writer.apply_reviewed_identity(
        &store,
        "reviewed-retained",
        &request,
        store.snapshot().revision(),
        unrelated_birth.head(),
    )?;
    let original = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("original merge")?;
    let expected = original.resolve_public(second_birth.public_id())?;
    let ReviewedIdentityRequestV1::Merge { case_id, .. } = &request else {
        return Err("fixture did not propose merge".into());
    };
    let mut live = Inputs::capture(&store.snapshot())?;
    let case = live
        .cases
        .iter_mut()
        .find(|case| &case.id == case_id)
        .ok_or("merged case")?;
    case.state = ReviewState::Superseded;
    let verdict = live
        .verdicts
        .iter_mut()
        .find(|verdict| &verdict.case_id == case_id)
        .ok_or("merged verdict")?;
    verdict.accepted = false;
    verdict.observed_at = "2026-10-15".into();
    store.replace_many(Table::ReviewCases, &live.cases)?;
    store.replace_many(Table::IdentityVerdicts, &live.verdicts)?;
    let before = retained_state(&store)?;
    check_not_reviewed(writer.apply_reviewed_identity(
        &store,
        "new-superseded-review",
        &request,
        store.snapshot().revision(),
        merged.head(),
    ))?;
    check!(eq; retained_state(&store)?, before);
    drop(writer);
    drop(store);
    let (store, writer) =
        Store::open_public_identity_qualification_v2(directory.path().join("identity"))?
            .into_parts();
    let reopened = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("reopened merge")?;
    let [_, second_candidate, _] = &candidates;
    check!(eq; reopened.resolve_public(second_birth.public_id())?, expected);
    check!(eq; reopened.resolve_candidate(second_candidate)?, expected);
    check!(eq; reopened.resolve_public(first_birth.public_id())?, original.resolve_public(first_birth.public_id())?);
    let replayed = writer.apply_reviewed_identity(
        &store,
        "reviewed-retained",
        &request,
        store.snapshot().revision(),
        merged.head(),
    )?;
    check!(eq; replayed.replayed(), true);
    check!(eq; replayed.head(), merged.head());
    check!(eq; replayed.introduced_public_ids(), &[]);
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn relabeling_the_same_positive_evidence_as_different_person_cannot_split_public_ids() -> TestResult
{
    let ReviewedFixture {
        directory: _directory,
        store,
        writer,
        candidates,
        births,
        request,
        ..
    } = ready_for_merge()?;
    let [first_birth, second_birth, unrelated_birth] = &births;
    let [first_candidate, second_candidate, _] = &candidates;
    let merged = writer.apply_reviewed_identity(
        &store,
        "reviewed-no-relabel",
        &request,
        store.snapshot().revision(),
        unrelated_birth.head(),
    )?;
    let original = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("merged snapshot")?;
    let expected = original.resolve_public(second_birth.public_id())?;
    let members = vec![first_candidate.clone(), second_candidate.clone()];
    let (case, verdict) = review(&store, &members, "different_person")?;
    let mut inputs = Inputs::capture(&store.snapshot())?;
    inputs.cases.push(case.clone());
    inputs.verdicts.push(verdict);
    store.replace_many(Table::ReviewCases, &inputs.cases)?;
    store.replace_many(Table::IdentityVerdicts, &inputs.verdicts)?;
    let mut partitions = vec![
        BindingPartition {
            binding_ids: vec![binding_id(&store, 1)?],
            representative: first_candidate.clone(),
        },
        BindingPartition {
            binding_ids: vec![binding_id(&store, 2)?],
            representative: second_candidate.clone(),
        },
    ];
    partitions.sort_by(|left, right| left.binding_ids.cmp(&right.binding_ids));
    let split = ReviewedIdentityRequestV1::Split {
        case_id: case.id,
        canonical: first_birth.public_id().clone(),
        revoke_merges: vec![4],
        partitions,
        unresolved_bindings: Vec::new(),
        affected_record_keys: vec!["synthetic-performance:1".into()],
    };
    let before = retained_state(&store)?;
    check_not_reviewed(writer.apply_reviewed_identity(
        &store,
        "unsupported-split",
        &split,
        store.snapshot().revision(),
        merged.head(),
    ))?;
    let current = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("current snapshot")?;
    check!(eq; current.head(), merged.head());
    check!(eq; current.resolve_public(second_birth.public_id())?, expected);
    check!(eq; current.resolve_candidate(second_candidate)?, expected);
    check!(eq; current.correction_history(first_birth.public_id()), Some([].as_slice()));
    check!(eq; current.affected_record_keys(5), None);
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

fn check_not_reviewed<T>(result: crate::StoreResult<T>) -> TestResult {
    match result {
        Err(StoreError::PublicIdentity(PublicIdentityError::NotReviewed)) => Ok(()),
        Err(error) => Err(format!("unexpected reviewed refusal: {error:?}").into()),
        Ok(_) => Err("unsupported reviewed authority was accepted".into()),
    }
}

fn reviewed_root(store: &Store, sequence: u64) -> TestResult<String> {
    let bytes = store
        .journal
        .get(crate::public_identity::witness::key(sequence))?
        .ok_or("reviewed witness")?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    value
        .get("authority_context")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "reviewed retained root".into())
}

fn source_material(store: &Store) -> TestResult<Vec<Vec<u8>>> {
    let operations = [
        "reviewed-first-birth",
        "reviewed-second-birth",
        "reviewed-unrelated-birth",
    ];
    let mut bytes = Vec::new();
    for (position, operation) in operations.iter().enumerate() {
        let sequence = u64::try_from(position)?
            .checked_add(1)
            .ok_or("fixture sequence overflow")?;
        bytes.push(
            store
                .journal
                .get(crate::public_identity::witness::key(sequence))?
                .ok_or("birth witness")?
                .to_vec(),
        );
        bytes.push(
            store
                .receipts
                .get(crate::public_identity::witness::receipt_key(operation))?
                .ok_or("birth receipt")?
                .to_vec(),
        );
    }
    Ok(bytes)
}

fn verify_capture_bodies(inputs: &Inputs, bodies: &[Vec<u8>; 2]) -> TestResult {
    for body in bodies {
        let hash = format!("{:x}", Sha256::digest(body));
        let claim = inputs
            .athletes
            .iter()
            .flat_map(|athlete| &athlete.identity_attestations)
            .find(|claim| claim.capture_sha256 == hash)
            .ok_or("retained synthetic capture claim")?;
        let value: serde_json::Value = serde_json::from_slice(body)?;
        let subject: SourceIdentity = serde_json::from_value(
            value
                .get("published_subject")
                .cloned()
                .ok_or("published subject")?,
        )?;
        check!(eq; claim.subject, subject);
        check!(eq; value.get("synthetic_fixture"), Some(&serde_json::Value::Bool(true)));
        check!(eq; value.get("publisher").and_then(serde_json::Value::as_str), Some(claim.source_family.as_str()));
        check!(eq; value.get("producer").and_then(serde_json::Value::as_str), Some(claim.upstream_producer.as_str()));
        check!(eq; value.get("locator").and_then(serde_json::Value::as_str), Some(claim.subject_locator.as_str()));
    }
    Ok(())
}
