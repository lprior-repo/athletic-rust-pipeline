#![cfg(all(unix, feature = "native-fault-injection"))]
#![forbid(unsafe_code)]

#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

#[path = "public_identity_crash/controller.rs"]
mod controller;
#[path = "public_identity_crash/fixture.rs"]
mod fixture;
#[path = "public_identity_crash/fixture_files.rs"]
mod fixture_files;
#[path = "public_identity_crash/graph.rs"]
mod graph;
#[path = "public_identity_crash/manual.rs"]
mod manual;
#[path = "public_identity_crash/membership.rs"]
mod membership;
#[path = "public_identity_crash/oracle.rs"]
mod oracle;
#[path = "public_identity_crash/physical.rs"]
mod physical;

use census_domain::public_identity_history::PublicResolution;
use census_store::{IdentityCommitOutcomeV1, Store};

use controller::Boundary;
use fixture::{Fixture, OPERATION};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[test]
fn public_identity_sigterm_before_syncdata_retains_no_partial_acceptance() -> TestResult {
    exercise(Boundary::BeforeCommit)
}

#[test]
fn public_identity_sigterm_after_syncdata_retains_one_replayable_acceptance() -> TestResult {
    exercise(Boundary::AfterCommit)
}

#[test]
#[ignore = "owned subprocess fixture; invoked by the two boundary controllers"]
fn public_identity_crash_child_fixture() -> TestResult {
    fixture::child()
}

fn exercise(boundary: Boundary) -> TestResult {
    let (directory, fixture) = Fixture::create()?;
    let (directory, reached) = controller::interrupt(directory, &fixture.root, boundary)?;
    let interrupted = physical::read(&fixture.root)?;
    if boundary == Boundary::BeforeCommit {
        oracle::staged_delta(&fixture, &interrupted)?;
    }
    let (outcome, snapshot_sequence) = recover_and_replay(&fixture, boundary)?;
    let recovered = physical::read(&fixture.root)?;
    let receipt_digest =
        oracle::committed_delta(&fixture, &recovered, &outcome, snapshot_sequence)?;
    oracle::retained_graph_unchanged(&fixture, &interrupted, &recovered)?;
    if boundary == Boundary::AfterCommit {
        check!(eq; recovered, interrupted);
        check!(eq; receipt_digest, reached.input_digest);
    }
    replay_without_mutation(&fixture, &outcome)?;
    drop(directory);
    Ok(())
}

fn recover_and_replay(
    fixture: &Fixture,
    boundary: Boundary,
) -> TestResult<(IdentityCommitOutcomeV1, u64)> {
    let (store, writer) = Store::open_public_identity_qualification_v3(&fixture.root)?.into_parts();
    check!(eq; writer.store_identity(), &fixture.identity);
    let accepted = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("reopened qualification has no accepted reader")?;
    match boundary {
        Boundary::BeforeCommit => {
            check!(eq; accepted.head(), &fixture.genesis);
            check!(eq; accepted.resolve_candidate(&fixture.candidate)?, PublicResolution::Unknown);
        }
        Boundary::AfterCommit => {
            check!(eq; accepted.head().sequence, 1);
            check!(matches!(
                accepted.resolve_candidate(&fixture.candidate)?,
                PublicResolution::Canonical(_)
            ));
        }
    }
    let snapshot = store.snapshot();
    let revision = snapshot.revision();
    let snapshot_sequence = fixture::read_precommit_sequence(&fixture.root)?;
    let first = writer.establish_source_bound(
        &store,
        OPERATION,
        &fixture.candidate,
        revision,
        &fixture.genesis,
    )?;
    check!(eq; first.replayed(), boundary == Boundary::AfterCommit);
    let repeated = writer.establish_source_bound(
        &store,
        OPERATION,
        &fixture.candidate,
        revision,
        &fixture.genesis,
    )?;
    check!(repeated.replayed());
    check!(eq; repeated.public_id(), first.public_id());
    check!(eq; repeated.head(), first.head());
    let accepted = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("accepted reader disappeared after replay")?;
    let PublicResolution::Canonical(resolved) = accepted.resolve_candidate(&fixture.candidate)?
    else {
        return Err("replayed identity is not canonical".into());
    };
    check!(eq; &resolved.public_id, first.public_id());
    check!(eq; accepted.head(), first.head());
    store.flush()?;
    Ok((first, snapshot_sequence))
}

fn replay_without_mutation(fixture: &Fixture, committed: &IdentityCommitOutcomeV1) -> TestResult {
    let before = physical::read(&fixture.root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&fixture.root)?.into_parts();
    let repeated = writer.establish_source_bound(
        &store,
        OPERATION,
        &fixture.candidate,
        store.snapshot().revision(),
        &fixture.genesis,
    )?;
    check!(repeated.replayed());
    check!(eq; repeated.public_id(), committed.public_id());
    check!(eq; repeated.head(), committed.head());
    store.flush()?;
    drop(writer);
    drop(store);
    check!(eq; physical::read(&fixture.root)?, before);
    Ok(())
}
