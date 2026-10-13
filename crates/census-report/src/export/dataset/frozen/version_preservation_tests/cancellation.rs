use super::{
    root, seeded_store, version_fixture, RetainedEvidence, RetainedState, Store, TestResult,
    VersionCase, STATES,
};
use crate::export::dataset::frozen::cancel_freeze_v2;
use crate::report::ReportError;

const UNSUPPORTED: [VersionCase; 14] = [
    VersionCase::HistoricalSchema,
    VersionCase::HistoricalPolicy,
    VersionCase::PolicyThirteen,
    VersionCase::PolicyFourteen,
    VersionCase::PolicyFifteen,
    VersionCase::CodecTwo,
    VersionCase::PreviousPolicy,
    VersionCase::PriorAffiliationPolicy,
    VersionCase::PriorParentScopePolicy,
    VersionCase::HistoricalProtocol,
    VersionCase::ManifestCodec,
    VersionCase::AdvertisedInventory,
    VersionCase::InventoryHeader,
    VersionCase::ChunkDescriptor,
];

fn refused(store: &Store, job: &str, case: VersionCase) -> TestResult {
    let expected = format!("unsupported {}", case.refusal_what());
    check!(matches!(
        cancel_freeze_v2(store, job),
        Err(ReportError::ArtifactCorrupt { ref detail, .. }) if detail.contains(&expected)
    ));
    Ok(())
}

fn preserved(case: VersionCase, state: RetainedState) -> TestResult {
    let seeded = seeded_store()?;
    let store = seeded.store;
    let job = format!("cancel-version-preservation:{case:?}:{state:?}");
    let current = super::freeze_v2(&store, &job)?;
    let directory = current.directory().to_path_buf();
    let retained = version_fixture::install(&store, &current, case, state)?;
    let before =
        RetainedEvidence::capture(&store, &directory, &[current.attempt(), &retained.attempt])?;
    (0..2).try_for_each(|_| -> TestResult {
        refused(&store, &job, case)?;
        before.unchanged(&store, &directory)
    })?;
    drop(store);
    let reopened = Store::open(seeded.directory.path())?;
    before.unchanged(&reopened, &directory)?;
    (0..2).try_for_each(|_| -> TestResult {
        refused(&reopened, &job, case)?;
        before.unchanged(&reopened, &directory)
    })?;
    check!(eq; root::read_retained(&directory)?, Some(retained));
    Ok(())
}

#[test]
fn cancellation_preserves_unsupported_roots_in_every_state_and_after_reopen() -> TestResult {
    UNSUPPORTED.into_iter().try_for_each(|case| {
        STATES
            .into_iter()
            .try_for_each(|state| preserved(case, state))
    })
}
