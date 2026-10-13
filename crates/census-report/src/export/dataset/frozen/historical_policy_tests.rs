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
    records: BTreeMap<String, InputRecord>,
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
                    Ok((
                        (*attempt).to_string(),
                        store.get_input_record(attempt)?.ok_or("input missing")?,
                    ))
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
    check!(eq; current.lineage.policy_revision, 8);
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
    let bytes = root::payload_bytes_of(manifest)?;
    let attempt = serialized_digest(&("historical-policy-fixture", &manifest_digest))?;
    let record = InputRecord::new(&attempt, bytes, &inventory)?;
    store.put_input_record(&record)?;
    let (epoch, claimed) = store.input_lease_acquire(
        &attempt,
        record.revision(),
        "historical-policy-fixture",
        "2999-12-31",
    )?;
    let freezing = store.input_freeze(&attempt, claimed.revision, epoch, &inventory)?;
    store.input_frozen(&attempt, freezing.revision, epoch, bytes, &inventory)?;
    store.lease_release(&attempt, epoch)?;
    let frozen = store
        .get_input_record(&attempt)?
        .ok_or("historical input missing")?;
    check!(eq; frozen.state(), InputState::Frozen);
    verify::check_record(manifest, &frozen, &inventory)?;
    let retained = Retained {
        attempt,
        manifest: manifest_digest,
    };
    root::write_retained(directory, &retained)?;
    Ok(retained)
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
    let retained = install_historical_root(&store, &directory, &historical)?;
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
