use super::{controller, physical};
use census_domain::public_identity_history::PublicResolution;
use census_store::{Store, Table};
use controller::Boundary;
#[path = "membership_evidence.rs"]
mod evidence;
#[path = "membership_fixture.rs"]
mod fixture;
#[path = "membership_oracle.rs"]
mod oracle;
use fixture::{Fixture, Mode};
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[test]
fn membership_before_commit_has_no_partial_identity() -> TestResult {
    exercise(Mode::Attach, Boundary::BeforeCommit)
}
#[test]
fn membership_after_commit_replays_exact_receipt() -> TestResult {
    exercise(Mode::Attach, Boundary::AfterCommit)
}
#[test]
fn withholding_before_commit_retains_original_membership() -> TestResult {
    exercise(Mode::Withhold, Boundary::BeforeCommit)
}
#[test]
fn withholding_after_commit_retains_anchor_and_exact_receipt() -> TestResult {
    exercise(Mode::Withhold, Boundary::AfterCommit)
}
#[test]
#[ignore = "owned subprocess fixture invoked by boundary controllers"]
fn membership_crash_child_fixture() -> TestResult {
    fixture::child()
}

fn exercise(mode: Mode, boundary: Boundary) -> TestResult {
    let (directory, fixture) = Fixture::create(mode)?;
    let (directory, reached) = controller::interrupt_case(
        directory,
        &fixture.root,
        boundary,
        controller::CrashCase {
            child_name: fixture::CHILD_NAME,
            operation: fixture.plan.operation(),
            sequence: fixture.plan.prior_head().sequence + 1,
        },
    )?;
    let interrupted = physical::read(&fixture.root)?;
    if boundary == Boundary::BeforeCommit {
        oracle::staged(&fixture, &interrupted)?;
    }
    let (frozen, head, snapshot_sequence) = recover(&fixture, mode, boundary)?;
    let after = physical::read(&fixture.root)?;
    if boundary == Boundary::AfterCommit {
        check!(eq; after, interrupted);
    }
    let receipt_digest = oracle::committed(&fixture, &after, mode, &head, snapshot_sequence)?;
    if boundary == Boundary::AfterCommit {
        check!(eq; receipt_digest, reached.input_digest);
    }
    replay_without_mutation(&fixture, &head)?;
    drop(directory);
    let expected = match mode {
        Mode::Attach => fixture.owner,
        Mode::Withhold => PublicResolution::Unresolved,
    };
    check!(eq; frozen.replay()?.resolve_candidate(&fixture.plan.attachment.candidate)?, expected);
    Ok(())
}

fn recover(
    fixture: &Fixture,
    mode: Mode,
    boundary: Boundary,
) -> TestResult<(
    census_store::FrozenPublicIdentityBundleV1,
    census_domain::public_identity_history::RegistryHead,
    u64,
)> {
    let (store, writer) = Store::open_public_identity_qualification_v2(&fixture.root)?.into_parts();
    let snapshot = store.snapshot();
    let snapshot_sequence = match boundary {
        Boundary::BeforeCommit => snapshot.sequence(),
        Boundary::AfterCommit => super::fixture::read_precommit_sequence(&fixture.root)?,
    };
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("accepted snapshot")?;
    check!(eq; accepted.head().sequence, fixture.plan.prior_head().sequence + u64::from(boundary == Boundary::AfterCommit));
    let expected_before = match (mode, boundary) {
        (Mode::Attach, Boundary::BeforeCommit) => PublicResolution::Unknown,
        (Mode::Withhold, Boundary::AfterCommit) => PublicResolution::Unresolved,
        _ => fixture.owner.clone(),
    };
    check!(eq; accepted.resolve_candidate(&fixture.plan.attachment.candidate)?, expected_before);
    drop(accepted);
    drop(snapshot);
    let (head, replayed) = fixture.plan.apply(&store, &writer)?;
    check!(eq; replayed, boundary == Boundary::AfterCommit);
    let (again, replayed) = fixture.plan.apply(&store, &writer)?;
    check!(replayed);
    check!(eq; head, again);
    check!(eq; store.scan::<census_domain::model::AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?.len(), 2);
    let frozen = store.snapshot().freeze_public_identity_v1()?;
    let accepted = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("committed snapshot")?;
    check!(eq; accepted.resolve_public(&fixture.plan.attachment.owner)?, fixture.owner);
    check!(eq; serde_json::to_vec(accepted.history_records().first().ok_or("birth")?)?, fixture.source_record);
    store.flush()?;
    drop(accepted);
    drop(writer);
    drop(store);
    Ok((frozen, head, snapshot_sequence))
}

fn replay_without_mutation(
    fixture: &Fixture,
    head: &census_domain::public_identity_history::RegistryHead,
) -> TestResult {
    let before = physical::read(&fixture.root)?;
    let (store, writer) = Store::open_public_identity_qualification_v2(&fixture.root)?.into_parts();
    let sequence = store.snapshot().sequence();
    let (again, replayed) = fixture.plan.apply(&store, &writer)?;
    check!(replayed);
    check!(eq; &again, head);
    check!(eq; store.snapshot().sequence(), sequence);
    store.flush()?;
    drop(writer);
    drop(store);
    check!(eq; physical::read(&fixture.root)?, before);
    Ok(())
}
