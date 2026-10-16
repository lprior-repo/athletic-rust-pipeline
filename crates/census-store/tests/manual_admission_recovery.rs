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
#[path = "manual_admission_recovery/membership.rs"]
mod membership;
#[path = "manual_admission_recovery/oracle.rs"]
mod oracle;
#[path = "public_identity_crash/physical.rs"]
mod physical;
#[path = "public_identity_crash/manual_results.rs"]
mod results;

use census_store::Store;
use fixture::{Fixture, Mode};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[test]
fn split_partial_staging_sigterm_preserves_mixed_admission_and_exact_retry() -> TestResult {
    exercise(Mode::Split)
}

#[test]
fn withholding_partial_staging_sigterm_preserves_mixed_admission_and_exact_retry() -> TestResult {
    exercise(Mode::Withhold)
}

#[test]
#[ignore = "owned subprocess fixture invoked by the partial context staging controllers"]
fn manual_admission_partial_staging_child() -> TestResult {
    fixture::child()
}

fn exercise(mode: Mode) -> TestResult {
    let (directory, fixture) = Fixture::create(mode)?;
    let (directory, reached) = controller::interrupt(directory, &fixture)?;
    let interrupted = physical::read(&fixture.root)?;
    let admission = oracle::interrupted(&fixture, &interrupted, &reached.input_digest)?;
    recover(&fixture, &admission)?;
    let recovered = physical::read(&fixture.root)?;
    oracle::recovered(&fixture, &interrupted, &recovered)?;
    repeat_without_writes(&fixture, &admission)?;
    check!(eq; physical::read(&fixture.root)?, recovered);
    drop(directory);
    Ok(())
}

fn recover(fixture: &Fixture, original: &[u8]) -> TestResult {
    let (store, writer) = Store::open_public_identity_qualification_v2(&fixture.root)?.into_parts();
    check!(eq; writer.store_identity(), &fixture.identity);
    check!(eq; writer.head(&store)?, fixture.head);
    check!(eq; store.evidence_generation(), fixture.evidence);
    check!(eq; store.derived_generation(), fixture.derived);
    let prepared =
        fixture
            .requests
            .prepare(&store, &writer, fixture.mode, fixture.mode.operation())?;
    check!(eq; prepared.as_slice(), original);
    check!(eq; writer.head(&store)?, fixture.head);
    Ok(())
}

fn repeat_without_writes(fixture: &Fixture, original: &[u8]) -> TestResult {
    let (store, writer) = Store::open_public_identity_qualification_v2(&fixture.root)?.into_parts();
    let sequence = store.snapshot().sequence();
    let repeated =
        fixture
            .requests
            .prepare(&store, &writer, fixture.mode, fixture.mode.operation())?;
    check!(eq; repeated.as_slice(), original);
    check!(eq; store.snapshot().sequence(), sequence);
    check!(fixture
        .requests
        .prepare(
            &store,
            &writer,
            fixture.mode.other(),
            fixture.mode.operation()
        )
        .is_err());
    check!(eq; store.snapshot().sequence(), sequence);
    let mut changed = fixture.requests.clone();
    changed.change_rationale(fixture.mode);
    check!(changed
        .prepare(&store, &writer, fixture.mode, fixture.mode.operation())
        .is_err());
    check!(eq; store.snapshot().sequence(), sequence);
    Ok(())
}
