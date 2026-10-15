use std::sync::Barrier;

use crate::{IdentityCommitOutcomeV1, PublicIdentityError, PublicIdentityWriterV1};
use census_domain::model::{AppliedAthleteIdentity, AthleteCandidateId};
use census_domain::public_identity_history::PublicResolution;

use super::support::refused;
use super::*;

#[test]
fn public_identity_derived_only_revision_movement_refuses_without_acceptance() -> TestResult {
    let (directory, store, writer) = fixture()?;
    let athlete = subject("derived-only-revision", "901")?;
    store.append(Table::Athletes, &athlete)?;
    let revision = store.snapshot().revision();
    let head = writer.head(&store)?;
    let next = revision
        .derived_generation()
        .checked_add(1)
        .ok_or("fixture overflow")?;
    {
        let _guard = store.lock_appends();
        let mut batch = store.db.batch();
        crate::generation::write_derived_current(&mut batch, &store.meta, next);
        batch.insert(
            &store.meta,
            crate::generation::DERIVED_NEXT,
            next.checked_add(1)
                .ok_or("fixture overflow")?
                .to_string()
                .as_bytes(),
        );
        batch
            .durability(Some(fjall::PersistMode::SyncData))
            .commit()?;
        store.generations.publish_derived_current(next);
    }
    check!(eq; store.evidence_generation(), revision.evidence_generation());
    check!(eq; store.derived_generation(), next);
    check!(eq; writer.head(&store)?, head);
    let before = retained_state(&store)?;
    let error = refused(writer.establish_source_bound(
        &store,
        "derived-only-revision",
        &athlete.id.cast(),
        revision,
        &head,
    ))?;
    check!(matches!(error, StoreError::RevisionMoved {
        operation, expected_evidence, actual_evidence, expected_derived, actual_derived,
    } if operation == "derived-only-revision"
        && expected_evidence == revision.evidence_generation()
        && actual_evidence == expected_evidence
        && expected_derived == revision.derived_generation()
        && actual_derived == next));
    check!(eq; retained_state(&store)?, before);
    drop(writer);
    drop(store);
    let root = directory.path().join("identity");
    let (store, writer) = Store::open_public_identity_qualification_v2(&root)?.into_parts();
    check!(eq; writer.head(&store)?, head);
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn public_identity_writer_clones_racing_exact_payload_commit_once_and_replay_once() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let athlete = subject("exact-race", "902")?;
    store.append(Table::Athletes, &athlete)?;
    let evidence = store.evidence_generation();
    let [left, right] = race(
        &store,
        &writer,
        "exact-race",
        [athlete.id.cast(), athlete.id.cast()],
    )?;
    let left = left?;
    let right = right?;
    check!(ne; left.replayed(), right.replayed());
    check!(eq; left.public_id(), right.public_id());
    check!(eq; left.head(), right.head());
    single_acceptance(&store, &writer, evidence)?;
    let accepted = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("missing accepted reader")?;
    let PublicResolution::Canonical(resolved) = accepted.resolve_candidate(&athlete.id.cast())?
    else {
        return Err("raced candidate is not accepted".into());
    };
    check!(eq; &resolved.public_id, left.public_id());
    Ok(())
}

#[test]
fn public_identity_writer_clones_racing_changed_payload_refuse_one_operation_conflict() -> TestResult
{
    let (_directory, store, writer) = fixture()?;
    let first = subject("conflict-race-first", "903")?;
    let second = subject("conflict-race-second", "904")?;
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
    let evidence = store.evidence_generation();
    let results = race(
        &store,
        &writer,
        "conflict-race",
        [first.id.cast(), second.id.cast()],
    )?;
    let (committed, winner, loser) = match results {
        [Ok(committed), Err(StoreError::PublicIdentity(PublicIdentityError::OperationConflict))] => {
            (committed, first.id.cast(), second.id.cast())
        }
        [Err(StoreError::PublicIdentity(PublicIdentityError::OperationConflict)), Ok(committed)] => {
            (committed, second.id.cast(), first.id.cast())
        }
        other => {
            return Err(format!("expected one commit and one payload conflict: {other:?}").into())
        }
    };
    check!(!committed.replayed());
    single_acceptance(&store, &writer, evidence)?;
    let accepted = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("missing accepted reader")?;
    let PublicResolution::Canonical(resolved) = accepted.resolve_candidate(&winner)? else {
        return Err("race winner is not accepted".into());
    };
    check!(eq; &resolved.public_id, committed.public_id());
    check!(eq; accepted.resolve_candidate(&loser)?, PublicResolution::Unknown);
    let before = retained_state(&store)?;
    let repeated = writer.establish_source_bound(
        &store,
        "conflict-race",
        &winner,
        store.snapshot().revision(),
        committed.head(),
    )?;
    check!(repeated.replayed());
    check!(eq; repeated.public_id(), committed.public_id());
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

fn race(
    store: &Store,
    writer: &PublicIdentityWriterV1,
    operation: &str,
    candidates: [AthleteCandidateId; 2],
) -> TestResult<[StoreResult<IdentityCommitOutcomeV1>; 2]> {
    let revision = store.snapshot().revision();
    let head = writer.head(store)?;
    let barrier = Barrier::new(2);
    let results = std::thread::scope(|scope| {
        let handles = candidates.map(|candidate| {
            let writer = writer.clone();
            let barrier = &barrier;
            let head = &head;
            scope.spawn(move || {
                barrier.wait();
                writer.establish_source_bound(store, operation, &candidate, revision, head)
            })
        });
        handles.map(|handle| handle.join())
    });
    let [left, right] = results;
    Ok([
        left.map_err(|_| "first identity race worker panicked")?,
        right.map_err(|_| "second identity race worker panicked")?,
    ])
}

fn single_acceptance(
    store: &Store,
    writer: &PublicIdentityWriterV1,
    previous_evidence: u64,
) -> TestResult {
    check!(eq; writer.head(store)?.sequence, 1);
    check!(eq; store.evidence_generation(), previous_evidence.checked_add(1).ok_or("fixture overflow")?);
    check!(eq; store.count(Table::AthleteIdentityDecisions)?, 1);
    check!(eq; store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?.len(), 1);
    for keyspace in [&store.journal, &store.receipts] {
        let keys = keyspace
            .prefix("public_identity")
            .take(2)
            .map(|guard| guard.key())
            .collect::<Result<Vec<_>, _>>()?;
        check!(eq; keys.len(), 1);
    }
    Ok(())
}
