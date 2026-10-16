use super::*;
use crate::public_identity::context::reviewed_fixture::review;
use crate::public_identity_tests::support::{fixture, retained_state, subject, TestResult};
use crate::{PublicIdentityWriterV1, Store, Table};
use census_domain::model::{AppliedAthleteIdentity, CanonicalAthlete, SchoolId};
use census_domain::public_identity_history::PublicResolution;

struct Fixture {
    _directory: tempfile::TempDir,
    store: Store,
    writer: PublicIdentityWriterV1,
    request: SourceCandidateMembershipRequestV1,
}

fn ready() -> TestResult<Fixture> {
    let (directory, store, writer) = fixture()?;
    writer.enroll_manual_context_admission_v1(&store)?;
    let first = subject("membership", "7401")?;
    store.append(Table::Athletes, &first)?;
    let birth = writer.establish_source_bound(
        &store,
        "membership-birth",
        &first.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let mut second = CanonicalAthlete::new(
        &SchoolId::mint("sch", &["membership-second-school"]),
        &first.canonical_name,
        first.grad_year,
        first.gender,
        first.source.clone().ok_or("primary")?,
    )?;
    second.evidence = first.evidence.clone();
    store.append(Table::Athletes, &second)?;
    let (case, verdict) = review(&store, &[first.id.cast(), second.id.cast()], "same_person")?;
    store.replace_many(Table::ReviewCases, std::slice::from_ref(&case))?;
    store.replace_many(Table::IdentityVerdicts, &[verdict])?;
    let key = crate::public_identity::witness::key(1);
    let raw = store.journal.get(key)?.ok_or("birth witness")?;
    let witness: crate::public_identity::witness::Witness = serde_json::from_slice(&raw)?;
    let request = SourceCandidateMembershipRequestV1 {
        case_id: case.id,
        owner: birth.public_id().clone(),
        anchor_binding_id: format!("source_binding_v1_{}", witness.request_digest),
        candidate: second.id.cast(),
    };
    Ok(Fixture {
        _directory: directory,
        store,
        writer,
        request,
    })
}

#[test]
fn genuine_membership_receipt_and_frozen_replay_preserve_anchor() -> TestResult {
    let Fixture {
        _directory,
        store,
        writer,
        request,
    } = ready()?;
    let before = store.snapshot().freeze_public_identity_v1()?;
    let snapshot = store.snapshot();
    let head = writer.head(&store)?;
    let attached = writer.extend_source_candidate(
        &store,
        "membership-attach",
        &request,
        snapshot.revision(),
        &head,
    )?;
    check!(eq; attached.public_id(), &request.owner);
    check!(attached.introduced_public_ids().is_empty());
    check!(!attached.replayed());
    check!(eq; attached.head().sequence, 2);
    let after = store.snapshot().freeze_public_identity_v1()?;
    check!(eq; before.replay()?.resolve_candidate(&request.candidate)?, PublicResolution::Unknown);
    let resolution = after.replay()?.resolve_candidate(&request.candidate)?;
    check!(
        matches!(&resolution, PublicResolution::Canonical(owner) if owner.public_id == request.owner)
    );
    check!(eq; store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?.len(), 2);
    let state = retained_state(&store)?;
    let repeated = writer.extend_source_candidate(
        &store,
        "membership-attach",
        &request,
        snapshot.revision(),
        &head,
    )?;
    check!(repeated.replayed());
    check!(eq; repeated.head(), attached.head());
    check!(eq; retained_state(&store)?, state);
    let mut changed = request.clone();
    changed.case_id.push_str("-changed");
    check!(writer
        .extend_source_candidate(
            &store,
            "membership-attach",
            &changed,
            snapshot.revision(),
            &head
        )
        .is_err());
    check!(eq; retained_state(&store)?, state);
    drop(snapshot);
    drop(writer);
    drop(store);
    std::fs::remove_dir_all(_directory.path().join("identity"))?;
    check!(eq; after.replay()?.resolve_candidate(&request.candidate)?, resolution);
    check!(eq; before.replay()?.resolve_candidate(&request.candidate)?, PublicResolution::Unknown);
    Ok(())
}

#[test]
fn membership_refuses_stale_foreign_and_wrong_anchor_atomically() -> TestResult {
    let fixture = ready()?;
    let Fixture {
        store,
        writer,
        request,
        ..
    } = &fixture;
    let snapshot = store.snapshot();
    let head = writer.head(store)?;
    let mut wrong = request.clone();
    wrong.anchor_binding_id.push_str("-foreign");
    let before = retained_state(store)?;
    check!(writer
        .extend_source_candidate(store, "wrong-anchor", &wrong, snapshot.revision(), &head)
        .is_err());
    check!(eq; retained_state(store)?, before);
    let other = ready()?;
    check!(writer
        .extend_source_candidate(
            store,
            "foreign-revision",
            request,
            other.store.snapshot().revision(),
            &head
        )
        .is_err());
    let mut wrong_head = head.clone();
    wrong_head.digest.push('0');
    check!(writer
        .extend_source_candidate(
            store,
            "wrong-head",
            request,
            snapshot.revision(),
            &wrong_head
        )
        .is_err());
    check!(eq; retained_state(store)?, before);
    store.append(Table::Athletes, &subject("membership-unrelated", "7402")?)?;
    let before = retained_state(store)?;
    check!(writer
        .extend_source_candidate(store, "stale", request, snapshot.revision(), &head)
        .is_err());
    check!(eq; retained_state(store)?, before);
    Ok(())
}

#[test]
fn membership_witness_tamper_and_legacy_manual_version_refuse() -> TestResult {
    let fixture = ready()?;
    let Fixture {
        store,
        writer,
        request,
        ..
    } = &fixture;
    writer.extend_source_candidate(
        store,
        "attach-tamper",
        request,
        store.snapshot().revision(),
        &writer.head(store)?,
    )?;
    let key = crate::public_identity::witness::key(2);
    let raw = store.journal.get(&key)?.ok_or("membership witness")?;
    let original: serde_json::Value = serde_json::from_slice(&raw)?;
    for field in [
        "request_digest",
        "decision_digest",
        "record_digest",
        "authority_context",
    ] {
        let mut value = original.clone();
        value[field] = serde_json::Value::String("tampered".into());
        let mut batch = store.db.batch();
        batch.insert(&store.journal, &key, serde_json::to_vec(&value)?);
        batch.commit()?;
        check!(writer.head(store).is_err());
    }
    let mut batch = store.db.batch();
    batch.insert(&store.journal, &key, raw.as_ref());
    batch.commit()?;
    check!(eq; writer.head(store)?.sequence, 2);
    let mut legacy = original;
    legacy["version"] = serde_json::Value::from(4);
    check!(crate::public_identity::operation::HeldWitness::decode(
        "manual4",
        &serde_json::to_vec(&legacy)?
    )
    .is_err());
    let oversized = vec![b' '; 65_537];
    check!(
        crate::public_identity::operation::HeldWitness::decode("oversized", &oversized).is_err()
    );
    Ok(())
}

mod withholding;

mod admission;
