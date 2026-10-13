mod merge;

use super::codec::FrozenCodecError;
use super::fixtures::{seeded_store, TestResult};
use super::manifest::FrozenManifest;
use super::root::{self, Retained};
use super::{freeze_v2, objects, verify};
use crate::report::ReportError;
use census_domain::model::{serialized_bytes, serialized_digest};
use census_store::{InputRecord, InputState, LeaseRecord, Store};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
struct RetainedEvidence {
    marker: Vec<u8>,
    objects: BTreeMap<OsString, Vec<u8>>,
    records: BTreeMap<String, Option<InputRecord>>,
    leases: BTreeMap<String, Option<LeaseRecord>>,
    sequence: u64,
}

impl RetainedEvidence {
    fn capture(store: &Store, directory: &Path, attempts: &[&str]) -> TestResult<Self> {
        Ok(Self {
            marker: std::fs::read(directory.join("root.json"))?,
            objects: object_bytes(&objects::directory(directory))?,
            records: attempts
                .iter()
                .map(|attempt| -> TestResult<_> {
                    Ok(((*attempt).to_string(), store.get_input_record(attempt)?))
                })
                .collect::<TestResult<_>>()?,
            leases: attempts
                .iter()
                .map(|attempt| -> TestResult<_> {
                    Ok(((*attempt).to_string(), store.get_lease_record(attempt)?))
                })
                .collect::<TestResult<_>>()?,
            sequence: store.snapshot().sequence(),
        })
    }

    fn unchanged(&self, store: &Store, directory: &Path) -> TestResult {
        let attempts: Vec<_> = self.records.keys().map(String::as_str).collect();
        check!(eq; Self::capture(store, directory, &attempts)?, *self);
        Ok(())
    }
}

fn object_bytes(directory: &Path) -> TestResult<BTreeMap<OsString, Vec<u8>>> {
    std::fs::read_dir(directory)?
        .map(|entry| -> TestResult<_> {
            let entry = entry?;
            Ok((entry.file_name(), std::fs::read(entry.path())?))
        })
        .collect()
}

fn historical_manifest(current: &FrozenManifest) -> TestResult<FrozenManifest> {
    check!(eq; current.lineage.policy_revision, 15);
    let mut historical = current.clone();
    historical.lineage.policy_revision = 7;
    historical.lineage.input_generation = super::generation(&historical.lineage)?;
    historical.input_token.semantic_policy = 7;
    historical
        .input_token
        .validate_against(&historical.lineage)?;
    check!(ne; historical.lineage.input_generation, current.lineage.input_generation);
    Ok(historical)
}

fn install_historical_root(
    store: &Store,
    directory: &Path,
    manifest: &FrozenManifest,
    state: Option<InputState>,
) -> TestResult<Retained> {
    let encoded = serialized_bytes(manifest)?;
    check!(matches!(
        FrozenManifest::decode(&encoded),
        Err(FrozenCodecError::UnsupportedVersion { what: "manifest lineage", ref found, .. })
            if found.contains("policy revision 7")
    ));
    let manifest_digest = objects::install(&objects::directory(directory), &encoded)?;
    check!(eq;
        objects::read(&objects::directory(directory), &manifest_digest, objects::MANIFEST_CEILING)?,
        encoded
    );
    let inventory =
        verify::inventory_of(&objects::directory(directory), &manifest_digest, manifest)?;
    let attempt = serialized_digest(&("historical-policy-fixture", &manifest_digest))?;
    let retained = Retained {
        attempt,
        manifest: manifest_digest,
    };
    root::write_retained(directory, &retained)?;
    if let Some(state) = state {
        install_historical_record(store, &retained.attempt, manifest, &inventory, state)?;
    }
    check!(eq; store.get_input_record(&retained.attempt)?.map(|record| record.state()), state);
    Ok(retained)
}

fn install_historical_record(
    store: &Store,
    attempt: &str,
    manifest: &FrozenManifest,
    inventory: &[String],
    state: InputState,
) -> TestResult {
    let bytes = root::payload_bytes_of(manifest)?;
    let record = InputRecord::new(attempt, bytes, inventory)?;
    store.put_input_record(&record)?;
    let (epoch, claimed) = store.input_lease_acquire(
        attempt,
        record.revision(),
        "historical-policy-fixture",
        "2999-12-31",
    )?;
    match state {
        InputState::Requested => {}
        InputState::Freezing => {
            store.input_freeze(attempt, claimed.revision, epoch, inventory)?;
        }
        InputState::Frozen => {
            let freezing = store.input_freeze(attempt, claimed.revision, epoch, inventory)?;
            store.input_frozen(attempt, freezing.revision, epoch, bytes, inventory)?;
            store.lease_release(attempt, epoch)?;
        }
        InputState::Abandoned => {
            store.input_abandon(attempt, claimed.revision, epoch)?;
        }
    }
    let retained = store
        .get_input_record(attempt)?
        .ok_or("historical input missing")?;
    check!(eq; retained.state(), state);
    verify::check_record(manifest, &retained, inventory)?;
    check!(eq; store.get_lease_record(attempt)?.is_some(), state != InputState::Frozen);
    Ok(())
}

fn refused(store: &Store, job: &str) -> TestResult {
    check!(matches!(
        freeze_v2(store, job),
        Err(ReportError::ArtifactCorrupt { ref detail, .. })
            if detail.contains("unsupported manifest lineage")
                && detail.contains("policy revision 7")
    ));
    Ok(())
}

#[test]
fn freeze_reuse_preserves_a_checksummed_prior_policy_root_across_repeated_refusal_and_reopen(
) -> TestResult {
    let seeded = seeded_store()?;
    let store = seeded.store;
    let job = "freeze-recovery:prior-policy";
    let current = freeze_v2(&store, job)?;
    let directory = current.directory().to_path_buf();
    let historical = historical_manifest(current.manifest())?;
    let retained =
        install_historical_root(&store, &directory, &historical, Some(InputState::Frozen))?;
    let before =
        RetainedEvidence::capture(&store, &directory, &[current.attempt(), &retained.attempt])?;
    (0..2).try_for_each(|_| -> TestResult {
        refused(&store, job)?;
        before.unchanged(&store, &directory)
    })?;
    drop(store);
    let reopened = Store::open(seeded.directory.path())?;
    refused(&reopened, job)?;
    before.unchanged(&reopened, &directory)?;
    check!(eq; root::read_retained(&directory)?, Some(retained));
    Ok(())
}

#[test]
fn freeze_refusal_preserves_prior_policy_roots_in_missing_and_unfinished_record_states(
) -> TestResult {
    [
        None,
        Some(InputState::Requested),
        Some(InputState::Freezing),
        Some(InputState::Abandoned),
    ]
    .into_iter()
    .try_for_each(|state| -> TestResult {
        let seeded = seeded_store()?;
        let store = seeded.store;
        let job = format!("freeze-recovery:prior-policy:{state:?}");
        let current = freeze_v2(&store, &job)?;
        let directory = current.directory().to_path_buf();
        let historical = historical_manifest(current.manifest())?;
        let retained = install_historical_root(&store, &directory, &historical, state)?;
        let before = RetainedEvidence::capture(
            &store,
            &directory,
            &[current.attempt(), &retained.attempt],
        )?;
        check!(eq; before.records.get(&retained.attempt).ok_or("historical attempt was not captured")?.as_ref().map(InputRecord::state), state);
        if state.is_none() {
            check!(eq; before.leases.get(&retained.attempt), Some(&None));
        }
        (0..2).try_for_each(|_| -> TestResult {
            refused(&store, &job)?;
            before.unchanged(&store, &directory)
        })?;
        drop(store);
        let reopened = Store::open(seeded.directory.path())?;
        refused(&reopened, &job)?;
        before.unchanged(&reopened, &directory)?;
        check!(eq; root::read_retained(&directory)?, Some(retained));
        Ok(())
    })
}
