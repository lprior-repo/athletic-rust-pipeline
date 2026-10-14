use super::fixtures::{seeded_store, TestResult};
use super::inventory::StreamInventory;
use super::root;
use super::stream::StreamKind;
use super::{freeze_v2, objects, to_export_dataset};
use crate::report::ReportError;
use census_store::{InputState, Store};
use version_fixture::VersionCase;
use version_records::RetainedState;
use version_snapshot::{object_bytes, RetainedEvidence};

#[path = "coverage_tests/version_fixture.rs"]
mod version_fixture;
#[allow(clippy::duplicate_mod)]
#[path = "coverage_tests/version_records.rs"]
mod version_records;
#[allow(clippy::duplicate_mod)]
#[path = "coverage_tests/version_snapshot.rs"]
mod version_snapshot;

const STATES: [RetainedState; 5] = [
    RetainedState::Missing,
    RetainedState::Requested,
    RetainedState::Freezing,
    RetainedState::Frozen,
    RetainedState::Abandoned,
];
const CONTROLS: [RetainedState; 3] = [
    RetainedState::Requested,
    RetainedState::Freezing,
    RetainedState::Frozen,
];

fn preserved(case: VersionCase, state: RetainedState) -> TestResult {
    let seeded = seeded_store()?;
    let store = seeded.store;
    let job = format!("version-preservation:{case:?}:{state:?}");
    let current = freeze_v2(&store, &job)?;
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

fn refused(store: &Store, job: &str, case: VersionCase) -> TestResult {
    let expected = format!("unsupported {}", case.refusal_what());
    check!(matches!(
        freeze_v2(store, job),
        Err(ReportError::ArtifactCorrupt { ref detail, .. }) if detail.contains(&expected)
    ));
    Ok(())
}

#[test]
fn historical_schema_one_preserves_every_retained_state_across_refusal_and_reopen() -> TestResult {
    STATES
        .into_iter()
        .try_for_each(|state| preserved(VersionCase::HistoricalSchema, state))
}

#[test]
fn historical_policy_preserves_every_retained_state_across_refusal_and_reopen() -> TestResult {
    STATES
        .into_iter()
        .try_for_each(|state| preserved(VersionCase::HistoricalPolicy, state))
}

#[test]
fn policy_fifteen_schema_three_roots_are_preserved_in_every_state_and_after_reopen() -> TestResult {
    STATES
        .into_iter()
        .try_for_each(|state| preserved(VersionCase::PolicyFifteen, state))
}

#[test]
fn historical_token_protocol_preserves_every_retained_state_across_refusal_and_reopen() -> TestResult
{
    STATES
        .into_iter()
        .try_for_each(|state| preserved(VersionCase::HistoricalProtocol, state))
}

#[test]
fn future_manifest_codec_preserves_every_retained_state_across_refusal_and_reopen() -> TestResult {
    STATES
        .into_iter()
        .try_for_each(|state| preserved(VersionCase::ManifestCodec, state))
}

#[test]
fn advertised_future_inventory_preserves_every_retained_state_across_refusal_and_reopen(
) -> TestResult {
    STATES
        .into_iter()
        .try_for_each(|state| preserved(VersionCase::AdvertisedInventory, state))
}

#[test]
fn checksummed_unsupported_page_header_preserves_every_state_across_refusal_and_reopen(
) -> TestResult {
    STATES
        .into_iter()
        .try_for_each(|state| preserved(VersionCase::InventoryHeader, state))
}

#[test]
fn independently_versioned_chunk_descriptor_preserves_every_state_across_refusal_and_reopen(
) -> TestResult {
    STATES
        .into_iter()
        .try_for_each(|state| preserved(VersionCase::ChunkDescriptor, state))
}

fn retired_current_corruption(state: RetainedState) -> TestResult {
    let seeded = seeded_store()?;
    let store = seeded.store;
    let job = format!("version-preservation:current-corruption:{state:?}");
    let current = freeze_v2(&store, &job)?;
    let directory = current.directory();
    let retained = version_fixture::install(&store, &current, VersionCase::Supported, state)?;
    let reference = current
        .manifest()
        .stream(StreamKind::Schools)
        .ok_or("schools missing")?;
    let encoded = objects::read(
        &objects::directory(directory),
        &reference.inventory.digest,
        objects::PAGE_CEILING,
    )?;
    let page = StreamInventory::decode(&encoded)?;
    let victim = &page.chunks.first().ok_or("schools chunk missing")?.sha256;
    std::fs::remove_file(objects::directory(directory).join(victim))?;
    let remaining = object_bytes(directory)?;
    let original = store
        .get_input_record(current.attempt())?
        .ok_or("baseline input missing")?;
    check!(matches!(freeze_v2(&store, &job), Err(ref error) if error.to_string().contains(victim)));
    check!(eq; root::read_retained(directory)?, None);
    check!(eq; object_bytes(directory)?, remaining);
    check!(eq; store.get_input_record(current.attempt())?, Some(original));
    let record = store
        .get_input_record(&retained.attempt)?
        .ok_or("retired input missing")?;
    let expected = match state {
        RetainedState::Frozen => InputState::Frozen,
        RetainedState::Requested | RetainedState::Freezing => InputState::Abandoned,
        RetainedState::Missing | RetainedState::Abandoned => {
            return Err("unsupported control state".into());
        }
    };
    check!(eq; record.state(), expected);
    if state != RetainedState::Frozen {
        check!(eq; store.get_lease_record(&retained.attempt)?, None);
    }
    let fresh = freeze_v2(&store, &job)?;
    check!(ne; fresh.attempt(), retained.attempt.as_str());
    check!(ne; fresh.attempt(), current.attempt());
    check!(eq; to_export_dataset(&fresh, &store)?.schools.len(), 1);
    Ok(())
}

#[test]
fn supported_missing_object_still_retires_every_state_and_allows_a_fresh_capture() -> TestResult {
    CONTROLS
        .into_iter()
        .try_for_each(retired_current_corruption)
}

#[test]
fn previous_tie_policy_preserves_every_retained_state_across_refusal_and_reopen() -> TestResult {
    STATES
        .into_iter()
        .try_for_each(|state| preserved(VersionCase::PreviousPolicy, state))
}

#[test]
fn policy_nine_roots_are_preserved_across_all_record_states_and_reopen() -> TestResult {
    STATES
        .into_iter()
        .try_for_each(|state| preserved(VersionCase::PriorAffiliationPolicy, state))
}

#[test]
fn policy_ten_roots_are_preserved_across_all_record_states_and_reopen() -> TestResult {
    STATES
        .into_iter()
        .try_for_each(|state| preserved(VersionCase::PriorParentScopePolicy, state))
}

#[path = "version_preservation_tests/cancellation.rs"]
mod cancellation;

#[test]
fn policy_sixteen_schema_three_roots_are_preserved_in_every_state_and_after_reopen() -> TestResult {
    STATES
        .into_iter()
        .try_for_each(|state| preserved(VersionCase::PolicySixteen, state))
}

#[test]
fn policy_seventeen_is_preserved_in_every_retained_state() -> TestResult {
    STATES
        .into_iter()
        .try_for_each(|state| preserved(VersionCase::PolicySeventeen, state))
}
