use super::{DatasetLineage, ExportDataset, Loaded};
use crate::report::{io_error, ReportError, ReportResult};
use census_store::clock::{Clock, SystemClock};
use census_store::{Store, StoreSnapshot, Table};
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

mod io;
pub(super) mod job;

const SCHEMA_REVISION: u32 = 1;
const POLICY_REVISION: u32 = 6;
const INPUT_TABLES: [Table; 13] = [
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
];

#[derive(Serialize)]
struct Frozen<'a> {
    lineage: &'a DatasetLineage,
    data: &'a ExportDataset,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Archived {
    lineage: DatasetLineage,
    data: Loaded,
}

pub(super) fn lineage(store: &Store, snapshot: &StoreSnapshot<'_>) -> ReportResult<DatasetLineage> {
    let store_identity = store_identity(store)?;
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
    let archived: Archived = io::read_json(path)?;
    let expected = archived.lineage.clone();
    if expected.schema_revision != SCHEMA_REVISION
        || expected.policy_revision != POLICY_REVISION
        || !hex_digest(&expected.store_identity)
        || !hex_digest(&expected.source_digest)
        || census_domain::model::SchoolYear::from_date(&expected.generated_on).is_none()
    {
        return Err(invalid("unsupported or malformed frozen export lineage"));
    }
    let dataset = ExportDataset::from_loaded(archived.data, archived.lineage)?;
    if dataset.lineage != expected {
        return Err(invalid(
            "frozen export input digest does not match its retained data",
        ));
    }
    Ok(dataset)
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
    let path = store.root().join(".export-lineage.lock");
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|source| io_error(&path, source))?;
    lock.lock().map_err(|source| io_error(&path, source))?;
    let retained = store.journal_payloads("export-lineage-v1")?;
    let identity = match retained.as_slice() {
        [] => {
            let created = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|error| invalid(&format!("creating export store identity: {error}")))?;
            let identity = digest(&(
                store.root().display().to_string(),
                created.as_secs(),
                created.subsec_nanos(),
            ))?;
            store.journal_done("export-lineage-v1", "store-identity", &identity)?;
            identity
        }
        [value] => value
            .as_str()
            .ok_or_else(|| invalid("malformed durable export store identity"))?
            .to_string(),
        _ => return Err(invalid("multiple durable export store identities")),
    };
    if !hex_digest(&identity) {
        return Err(invalid("malformed persisted export store identity"));
    }
    Ok(identity)
}

fn hex_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn invalid(detail: &str) -> ReportError {
    ReportError::Invariant {
        detail: detail.to_string(),
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
