use census_store::{
    AcceptedPublicIdentitySnapshot, FrozenPublicIdentityBundleV2, LocalAdjudicationCommitOutcomeV1,
    PublicIdentityError, Store, StoreError, Table,
};

use super::controller::{self, Boundary, CrashCase};
use super::{physical, TestResult};

#[path = "manual_evidence.rs"]
mod evidence;
#[path = "manual_fixture.rs"]
mod fixture;
#[path = "manual_oracle.rs"]
mod oracle;
#[path = "manual_resolution.rs"]
mod resolution;
#[path = "manual_results.rs"]
mod results;

use fixture::{Approval, Fixture, CHILD_NAME, OPERATION};

#[test]
fn approved_manual_split_sigterm_before_syncdata_retains_no_partial_correction() -> TestResult {
    exercise(Boundary::BeforeCommit)
}

#[test]
fn approved_manual_split_sigterm_after_syncdata_retains_one_replayable_correction() -> TestResult {
    exercise(Boundary::AfterCommit)
}

#[test]
#[ignore = "owned synthetic subprocess; invoked only by the manual boundary controllers"]
fn public_identity_manual_crash_child_fixture() -> TestResult {
    fixture::child()
}

fn exercise(boundary: Boundary) -> TestResult {
    let (directory, fixture) = Fixture::create()?;
    let (directory, reached) = controller::interrupt_case(
        directory,
        &fixture.root,
        boundary,
        CrashCase {
            child_name: CHILD_NAME,
            operation: OPERATION,
            sequence: 5,
        },
    )?;
    let interrupted = physical::read(&fixture.root)?;
    let child_approval = fixture::read_approval(&fixture.root)?;
    let prepared_journal = oracle::prepared_journal(&fixture, &interrupted, &child_approval)?;
    if boundary == Boundary::BeforeCommit {
        oracle::staged(&fixture, &interrupted, &prepared_journal)?;
    }
    let recovered = recover(&fixture, boundary, child_approval)?;
    let after = physical::read(&fixture.root)?;
    let receipt_digest = oracle::committed(
        &fixture,
        &after,
        &recovered.approval,
        &recovered.outcome,
        &prepared_journal,
    )?;
    if boundary == Boundary::AfterCommit {
        check!(eq; after, interrupted);
        check!(eq; receipt_digest, reached.input_digest);
    }
    replay_without_mutation(&fixture, &recovered)?;
    resolution::frozen(
        &fixture.plan,
        &recovered.frozen,
        &recovered.accepted,
        &recovered.outcome,
    )?;
    drop(directory);
    Ok(())
}

struct Recovered {
    approval: Approval,
    outcome: LocalAdjudicationCommitOutcomeV1,
    accepted: AcceptedPublicIdentitySnapshot,
    frozen: FrozenPublicIdentityBundleV2,
}

fn recover(
    fixture: &Fixture,
    boundary: Boundary,
    child_approval: Approval,
) -> TestResult<Recovered> {
    let (store, writer) = Store::open_public_identity_qualification_v3(&fixture.root)?.into_parts();
    check!(eq; writer.store_identity(), &fixture.plan.identity);
    let (_, child_digest) = child_approval.encoded()?;
    let replay = writer.replay_local_adjudication(&store, OPERATION, &child_digest)?;
    let (approval, outcome) = match boundary {
        Boundary::BeforeCommit => {
            check!(replay.is_none());
            resolution::merged(&fixture.plan, &snapshot(&store)?)?;
            resolution::merged(
                &fixture.plan,
                &store.snapshot().freeze_public_identity_v2()?.replay()?,
            )?;
            let prepared = fixture::prepare(&store, &writer, &fixture.plan)?;
            let approval = Approval::new(&prepared);
            let (bytes, digest) = approval.encoded()?;
            check!(eq; prepared.approval_bytes()?, bytes);
            check!(eq; prepared.approval_digest()?, digest);
            let outcome = writer.apply_local_adjudication(&store, prepared, bytes, digest)?;
            check!(!outcome.replayed());
            (approval, outcome)
        }
        Boundary::AfterCommit => {
            let outcome = replay.ok_or("durable manual receipt missing after reopen")?;
            check!(outcome.replayed());
            (child_approval, outcome)
        }
    };
    check!(eq; store.snapshot().scan::<census_domain::model::AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?.len(), 4);
    let accepted = snapshot(&store)?;
    resolution::split(&fixture.plan, &accepted, &outcome)?;
    let frozen = store.snapshot().freeze_public_identity_v2()?;
    store.flush()?;
    Ok(Recovered {
        approval,
        outcome,
        accepted,
        frozen,
    })
}

fn replay_without_mutation(fixture: &Fixture, recovered: &Recovered) -> TestResult {
    let before = physical::read(&fixture.root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&fixture.root)?.into_parts();
    let sequence = store.snapshot().sequence();
    let (_, digest) = recovered.approval.encoded()?;
    for _ in 0..2 {
        let repeated = writer
            .replay_local_adjudication(&store, OPERATION, &digest)?
            .ok_or("manual replay")?;
        check!(repeated.replayed());
        check!(eq; repeated.head(), recovered.outcome.head());
        check!(eq; repeated.introduced_public_ids(), recovered.outcome.introduced_public_ids());
        resolution::split(&fixture.plan, &snapshot(&store)?, &repeated)?;
    }
    let wrong_digest = "0".repeat(64);
    check!(ne; digest, wrong_digest);
    check!(matches!(
        writer.replay_local_adjudication(&store, OPERATION, &wrong_digest),
        Err(StoreError::PublicIdentity(
            PublicIdentityError::OperationConflict
        ))
    ));
    check!(eq; store.snapshot().sequence(), sequence);
    check!(eq; store.snapshot().scan::<census_domain::model::AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?.len(), 4);
    check!(eq; snapshot(&store)?.history_records(), recovered.accepted.history_records());
    store.flush()?;
    drop(writer);
    drop(store);
    check!(eq; physical::read(&fixture.root)?, before);
    Ok(())
}

fn snapshot(store: &Store) -> TestResult<AcceptedPublicIdentitySnapshot> {
    store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or_else(|| "manual accepted snapshot missing".into())
}
