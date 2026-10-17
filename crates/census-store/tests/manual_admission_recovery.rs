#![cfg(all(unix, feature = "native-fault-injection"))]
#![forbid(unsafe_code)]

#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

#[path = "manual_admission_recovery/controller.rs"]
mod controller;
#[path = "public_identity_crash/manual_evidence.rs"]
mod evidence;
#[path = "manual_admission_recovery/fixture.rs"]
mod fixture;
#[path = "public_identity_crash/fixture_files.rs"]
mod fixture_files;
#[path = "public_identity_crash/graph.rs"]
mod graph;
#[path = "manual_admission_recovery/membership.rs"]
mod membership;
#[path = "manual_admission_recovery/oracle.rs"]
mod oracle;
#[path = "public_identity_crash/physical.rs"]
mod physical;
#[path = "public_identity_crash/manual_results.rs"]
mod results;

use census_store::Store;
use controller::Boundary;
use fixture::{Fixture, Mode};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[test]
fn split_raw_graph_chunk_sigterm_preserves_mixed_admission_and_exact_retry() -> TestResult {
    exercise(Mode::Split, Boundary::RawGraphChunk)
}

#[test]
fn withholding_raw_graph_chunk_sigterm_preserves_mixed_admission_and_exact_retry() -> TestResult {
    exercise(Mode::Withhold, Boundary::RawGraphChunk)
}

#[test]
fn split_preparation_syncdata_sigterm_replays_exact_approval_without_recharging() -> TestResult {
    exercise(Mode::Split, Boundary::Preparation)
}

#[test]
fn withholding_preparation_syncdata_sigterm_replays_exact_approval_without_recharging() -> TestResult
{
    exercise(Mode::Withhold, Boundary::Preparation)
}

#[test]
#[ignore = "owned subprocess fixture invoked by graph chunk and preparation controllers"]
fn manual_admission_graph_recovery_child() -> TestResult {
    fixture::child()
}

fn exercise(mode: Mode, boundary: Boundary) -> TestResult {
    let (directory, fixture) = Fixture::create(mode)?;
    let (directory, reached) = controller::interrupt(directory, &fixture, boundary)?;
    let interrupted = physical::read(&fixture.root)?;
    let original = oracle::interrupted(&fixture, &interrupted, boundary, &reached)?;
    let admission = recover(&fixture, original.as_deref())?;
    let recovered = physical::read(&fixture.root)?;
    oracle::recovered(&fixture, &interrupted, &recovered, boundary, &admission)?;
    repeat_without_writes(&fixture, &admission)?;
    check!(eq; physical::read(&fixture.root)?, recovered);
    drop(directory);
    Ok(())
}

fn recover(fixture: &Fixture, original: Option<&[u8]>) -> TestResult<Vec<u8>> {
    let (store, writer) = Store::open_public_identity_qualification_v3(&fixture.root)?.into_parts();
    check!(eq; writer.store_identity(), &fixture.identity);
    check!(eq; writer.head(&store)?, fixture.head);
    check!(eq; store.evidence_generation(), fixture.evidence);
    check!(eq; store.derived_generation(), fixture.derived);
    let prepared =
        fixture
            .requests
            .prepare(&store, &writer, fixture.mode, fixture.mode.operation())?;
    if let Some(original) = original {
        check!(eq; prepared.as_slice(), original);
    }
    check!(eq; writer.head(&store)?, fixture.head);
    Ok(prepared)
}

fn repeat_without_writes(fixture: &Fixture, original: &[u8]) -> TestResult {
    let (store, writer) = Store::open_public_identity_qualification_v3(&fixture.root)?.into_parts();
    let sequence = store.snapshot().sequence();
    let repeated =
        fixture
            .requests
            .prepare(&store, &writer, fixture.mode, fixture.mode.operation())?;
    check!(eq; repeated.as_slice(), original);
    check!(eq; store.snapshot().sequence(), sequence);
    check!(matches!(fixture
        .requests
        .prepare(
            &store,
            &writer,
            fixture.mode.other(),
            fixture.mode.operation()
        ), Err(error) if matches!(error.downcast_ref::<census_store::StoreError>(), Some(census_store::StoreError::PublicIdentity(census_store::PublicIdentityError::OperationConflict)))));
    check!(eq; store.snapshot().sequence(), sequence);
    let mut changed = fixture.requests.clone();
    changed.change_rationale(fixture.mode);
    check!(
        matches!(changed.prepare(&store, &writer, fixture.mode, fixture.mode.operation()),
        Err(error) if matches!(error.downcast_ref::<census_store::StoreError>(), Some(census_store::StoreError::PublicIdentity(census_store::PublicIdentityError::OperationConflict))))
    );
    check!(eq; store.snapshot().sequence(), sequence);
    Ok(())
}
