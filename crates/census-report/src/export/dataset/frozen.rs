use super::{DatasetLineage, ExportDataset};
use crate::report::{io_error, ReportError, ReportResult};
use census_store::clock::{Clock, SystemClock};
use census_store::{Store, StoreSnapshot, Table};
use codec::FrozenCodecError;
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

mod io;
pub(super) mod job;

mod archive;
mod budget;
mod capture;
mod chunks;
mod codec;
#[cfg(test)]
mod fixtures;
mod freeze;
mod inventory;
mod legacy;
mod line;
#[cfg(test)]
mod lineage_tests;
mod manifest;
mod materialize;
mod objects;
mod reader;
mod recover;
mod references;
mod root;
mod stream;
mod token;
mod verification_error;
mod verify;

#[cfg(test)]
mod coverage_tests;
#[cfg(test)]
mod version_preservation_tests;

#[cfg(test)]
mod freeze_recovery_tests;
#[cfg(test)]
mod freeze_v2_tests;
#[cfg(test)]
mod historical_policy_tests;
#[cfg(test)]
mod preflight_tests;

pub use freeze::CancelOutcome;
pub use root::FrozenRoot;

const SCHEMA_REVISION: u32 = 2;
const POLICY_REVISION: u32 = 13;
const INPUT_TABLES: [Table; 15] = [
    Table::Schools,
    Table::Teams,
    Table::Coaches,
    Table::Athletes,
    Table::Meets,
    Table::Events,
    Table::Performances,
    Table::ReviewCases,
    Table::SourceAccess,
    Table::IdentityVerdicts,
    Table::AthleteIdentityDecisions,
    Table::SourceObservations,
    Table::SourceMeets,
    Table::Coverage,
    Table::CoachLanes,
];

#[derive(Serialize)]
struct Frozen<'a> {
    lineage: &'a DatasetLineage,
    data: &'a ExportDataset,
}

pub(super) fn freeze_v2(store: &Store, job: &str) -> ReportResult<FrozenRoot> {
    freeze::freeze(store, job)
}

pub(super) fn cancel_freeze_v2(store: &Store, job: &str) -> ReportResult<CancelOutcome> {
    freeze::cancel(store, job)
}

pub(super) fn to_export_dataset(root: &FrozenRoot, store: &Store) -> ReportResult<ExportDataset> {
    let manifest = root.manifest();
    if manifest.lineage.store_identity != store_identity(store)? {
        return Err(invalid("a frozen root belongs to a different census store"));
    }
    let objects_dir = objects::directory(root.directory());
    verify::verify_root(&objects_dir, root.manifest_digest(), manifest)?;
    let dataset = ExportDataset::from_loaded(
        materialize::read_all(&objects_dir, manifest)?,
        manifest.lineage.clone(),
    )?;
    if dataset.lineage.input_digest != manifest.lineage.input_digest
        || dataset.lineage.input_generation != manifest.lineage.input_generation
    {
        return Err(invalid(
            "a frozen root does not reproduce the dataset input digest",
        ));
    }
    Ok(dataset)
}

pub(super) fn lineage(store: &Store, snapshot: &StoreSnapshot<'_>) -> ReportResult<DatasetLineage> {
    snapshot.ensure_origin(store)?;
    let store_identity = String::from(snapshot.store_identity()?);
    let bound = store.run_manifest()?;
    if let Some(manifest) = &bound {
        if manifest.store_identity != store_identity {
            return Err(invalid(
                "the store's run manifest belongs to another store root",
            ));
        }
    }
    Ok(DatasetLineage {
        store_root: store.root().display().to_string(),
        generated_on: SystemClock.today(),
        store_identity,
        export_job: None,
        input_generation: String::new(),
        input_digest: String::new(),
        source_digest: snapshot.tables_digest(&INPUT_TABLES)?,
        snapshot_sequence: snapshot.sequence(),
        schema_revision: SCHEMA_REVISION,
        policy_revision: POLICY_REVISION,
        run: bound.as_ref().map(|manifest| manifest.run),
        cohort: bound.as_ref().map(|manifest| manifest.cohort),
    })
}

pub(super) fn bind(dataset: &mut ExportDataset) -> ReportResult<()> {
    dataset.lineage.input_digest = digest(dataset)?;
    dataset.lineage.input_generation = generation(&dataset.lineage)?;
    Ok(())
}

pub(super) fn ensure_current(dataset: &ExportDataset, store: &Store) -> ReportResult<()> {
    job::ensure_store(dataset, store)?;
    ensure_snapshot(dataset, &store.snapshot())
}

pub(super) fn ensure_snapshot(
    dataset: &ExportDataset,
    snapshot: &StoreSnapshot<'_>,
) -> ReportResult<()> {
    if snapshot.tables_digest(&INPUT_TABLES)? != dataset.lineage.source_digest {
        return Err(invalid(
            "stale or foreign export input cannot publish changed source evidence",
        ));
    }
    Ok(())
}

pub(super) fn archive_digest(dataset: &ExportDataset) -> ReportResult<String> {
    digest(&Frozen {
        lineage: &dataset.lineage,
        data: dataset,
    })
}

pub(super) fn save(dataset: &ExportDataset, path: &Path) -> ReportResult<()> {
    if digest(dataset)? != dataset.lineage.input_digest
        || generation(&dataset.lineage)? != dataset.lineage.input_generation
    {
        return Err(invalid(
            "export dataset changed after its input generation was captured",
        ));
    }
    io::write_json(
        path,
        &Frozen {
            lineage: &dataset.lineage,
            data: dataset,
        },
    )
}

pub(super) fn reopen(path: &Path) -> ReportResult<ExportDataset> {
    archive::reopen(path, |_| Ok(()))
}

fn generation(lineage: &DatasetLineage) -> ReportResult<String> {
    digest(&(
        &lineage.store_identity,
        &lineage.source_digest,
        &lineage.input_digest,
        &lineage.generated_on,
        &lineage.export_job,
        lineage.schema_revision,
        lineage.policy_revision,
    ))
}

fn digest(value: &impl Serialize) -> ReportResult<String> {
    census_domain::model::serialized_digest(value)
        .map_err(|error| invalid(&format!("encoding frozen export input: {error}")))
}

pub(super) fn store_identity(store: &Store) -> ReportResult<String> {
    Ok(store.store_identity()?.into())
}

fn hex_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn capture_lock(lock_path: &Path) -> ReportResult<std::fs::File> {
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)
        .map_err(|source| io_error(lock_path, source))?;
    lock.try_lock().map_err(|error| {
        let source = match error {
            std::fs::TryLockError::WouldBlock => std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "export input is already being captured",
            ),
            std::fs::TryLockError::Error(source) => source,
        };
        io_error(lock_path, source)
    })?;
    Ok(lock)
}

fn invalid(detail: &str) -> ReportError {
    ReportError::Invariant {
        detail: detail.to_string(),
    }
}

fn codec_error(error: FrozenCodecError) -> ReportError {
    ReportError::ArtifactCorrupt {
        path: PathBuf::new(),
        detail: format!("frozen codec: {error}"),
    }
}

pub(super) fn map_values<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    Ok(BTreeMap::<String, T>::deserialize(deserializer)?
        .into_values()
        .collect())
}
