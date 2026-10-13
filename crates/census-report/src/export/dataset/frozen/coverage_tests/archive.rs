use super::super::codec::sha256_hex;
use super::super::fixtures::TestResult;
use super::evidence::{self, EvidenceShape};
use crate::export::{DatasetLineage, ExportDataset};
use crate::report::ReportError;
use census_domain::model::{serialized_bytes, CoverageRow};
use census_store::Store;
use serde_json::Value;
use std::path::Path;

#[derive(Clone, Copy)]
enum MissingEvidence {
    All,
    Rows,
    CoachLanes,
    NullAll,
    NullRows,
    NullCoachLanes,
}

fn remove_required(value: &mut Value, field: MissingEvidence) -> TestResult {
    let data = value
        .get_mut("data")
        .and_then(Value::as_object_mut)
        .ok_or("archive data object missing")?;
    match field {
        MissingEvidence::All => {
            data.remove("coverage_evidence")
                .ok_or("coverage evidence missing")?;
        }
        MissingEvidence::NullAll => {
            *data
                .get_mut("coverage_evidence")
                .ok_or("coverage evidence missing")? = Value::Null;
        }
        field => {
            let evidence = data
                .get_mut("coverage_evidence")
                .and_then(Value::as_object_mut)
                .ok_or("coverage evidence object missing")?;
            match field {
                MissingEvidence::Rows => {
                    evidence.remove("rows").ok_or("coverage rows missing")?;
                }
                MissingEvidence::CoachLanes => {
                    evidence
                        .remove("coach_lanes")
                        .ok_or("coach lanes missing")?;
                }
                MissingEvidence::NullRows => {
                    *evidence.get_mut("rows").ok_or("coverage rows missing")? = Value::Null;
                }
                MissingEvidence::NullCoachLanes => {
                    *evidence
                        .get_mut("coach_lanes")
                        .ok_or("coach lanes missing")? = Value::Null;
                }
                MissingEvidence::All | MissingEvidence::NullAll => {
                    return Err("nested evidence mutation was selected incorrectly".into());
                }
            }
        }
    }
    Ok(())
}

fn missing_is_refused(path: &Path, current: &Value, missing: MissingEvidence) -> TestResult {
    let mut malformed = current.clone();
    remove_required(&mut malformed, missing)?;
    let retained = serde_json::to_vec(&malformed)?;
    std::fs::write(path, &retained)?;
    (0..2).try_for_each(|_| -> TestResult {
        check!(matches!(
            ExportDataset::reopen_frozen(path),
            Err(ReportError::Decode { .. })
        ));
        check!(eq; std::fs::read(path)?, retained);
        Ok(())
    })
}

#[test]
fn schema_two_archives_require_both_coverage_arrays_and_reject_null() -> TestResult {
    let seeded = evidence::seed(EvidenceShape::Both)?;
    let path = seeded.directory.path().join("required-coverage.json");
    ExportDataset::load(&seeded.store)?.save_frozen(&path)?;
    let current: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    [
        MissingEvidence::All,
        MissingEvidence::Rows,
        MissingEvidence::CoachLanes,
        MissingEvidence::NullAll,
        MissingEvidence::NullRows,
        MissingEvidence::NullCoachLanes,
    ]
    .into_iter()
    .try_for_each(|missing| missing_is_refused(&path, &current, missing))
}

fn old_schema_archive(dataset: &ExportDataset, store: &Store) -> TestResult<Vec<u8>> {
    check!(dataset.relays.is_empty());
    let suffix = format!(
        ",\"coverage_evidence\":{},\"relays\":[]}}",
        String::from_utf8(serialized_bytes(&dataset.coverage_evidence)?)?
    );
    let encoded = serialized_bytes(dataset)?;
    let old_prefix = encoded
        .strip_suffix(suffix.as_bytes())
        .ok_or("required nested evidence suffix missing")?;
    let old_data: Vec<u8> = old_prefix
        .iter()
        .copied()
        .chain(std::iter::once(b'}'))
        .collect();
    let mut lineage = dataset.lineage.clone();
    lineage.schema_revision = 1;
    lineage.policy_revision = 8;
    let historical_tables = super::super::INPUT_TABLES
        .get(..13)
        .ok_or("historical table contract missing")?;
    lineage.source_digest = store.snapshot().tables_digest(historical_tables)?;
    lineage.input_digest = sha256_hex(&old_data);
    lineage.input_generation = super::super::generation(&lineage)?;
    check!(ne; lineage.input_digest, dataset.lineage.input_digest);
    check!(ne; lineage.input_generation, dataset.lineage.input_generation);
    let archive = format!(
        "{{\"lineage\":{},\"data\":{}}}",
        String::from_utf8(serialized_bytes(&lineage)?)?,
        String::from_utf8(old_data)?
    );
    Ok(archive.into_bytes())
}

fn unsupported_schema(path: &Path, retained: &[u8]) -> TestResult {
    check!(matches!(ExportDataset::reopen_frozen(path),
        Err(ReportError::Invariant { ref detail }) if detail == "unsupported or malformed frozen export lineage"));
    check!(eq; std::fs::read(path)?, retained);
    Ok(())
}

#[test]
fn valid_schema_one_archive_without_coverage_is_preserved_and_refused_after_reopen() -> TestResult {
    let seeded = evidence::seed(EvidenceShape::Both)?;
    let store = seeded.store;
    let dataset = ExportDataset::load(&store)?;
    let retained = old_schema_archive(&dataset, &store)?;
    let value: Value = serde_json::from_slice(&retained)?;
    let data = value.get("data").ok_or("historical data missing")?;
    check!(data.get("coverage_evidence").is_none());
    check!(data.get("relays").is_none());
    let lineage: DatasetLineage = serde_json::from_value(
        value
            .get("lineage")
            .ok_or("historical lineage missing")?
            .clone(),
    )?;
    check!(eq; lineage.schema_revision, 1);
    check!(eq; lineage.policy_revision, 8);
    check!(eq; super::super::generation(&lineage)?, lineage.input_generation);
    let path = seeded.directory.path().join("schema-one.json");
    std::fs::write(&path, &retained)?;
    (0..2).try_for_each(|_| unsupported_schema(&path, &retained))?;
    drop(dataset);
    drop(store);
    let reopened = Store::open(seeded.directory.path())?;
    unsupported_schema(&path, &retained)?;
    check!(eq; reopened.snapshot().scan::<CoverageRow>(census_store::Table::Coverage)?.len(), 1);
    Ok(())
}
