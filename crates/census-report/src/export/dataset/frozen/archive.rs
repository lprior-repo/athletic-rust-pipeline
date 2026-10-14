use super::super::{DatasetLineage, ExportDataset, Loaded};
use super::{generation, hex_digest, invalid, io, ReportResult, SCHEMA_REVISION};
use crate::report::io_error;
use serde::Deserialize;
use std::io::Seek;
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    lineage: DatasetLineage,
    #[serde(rename = "data")]
    _data: serde::de::IgnoredAny,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Archived {
    lineage: DatasetLineage,
    data: Loaded,
}

pub(super) fn reopen(
    path: &Path,
    eligible: impl FnOnce(&DatasetLineage) -> ReportResult<()>,
) -> ReportResult<ExportDataset> {
    let mut reader = io::open_json(path)?;
    let Header { lineage, _data: _ } = io::decode_json(path, &mut reader)?;
    validate(&lineage)?;
    eligible(&lineage)?;
    reader.rewind().map_err(|source| io_error(path, source))?;
    let archived: Archived = io::decode_json(path, &mut reader)?;
    if archived.lineage != lineage {
        return Err(invalid("frozen export lineage changed during preflight"));
    }
    let dataset = ExportDataset::from_loaded(archived.data, archived.lineage)?;
    if dataset.lineage != lineage {
        return Err(invalid(
            "frozen export input digest does not match its retained data",
        ));
    }
    Ok(dataset)
}

fn validate(lineage: &DatasetLineage) -> ReportResult<()> {
    if lineage.schema_revision != SCHEMA_REVISION
        || census_domain::model::EventSemantics::from_revision(lineage.policy_revision).is_none()
        || !hex_digest(&lineage.store_identity)
        || !hex_digest(&lineage.source_digest)
        || !hex_digest(&lineage.input_digest)
        || !hex_digest(&lineage.input_generation)
        || census_domain::model::SchoolYear::from_date(&lineage.generated_on).is_none()
        || generation(lineage)? != lineage.input_generation
    {
        return Err(invalid("unsupported or malformed frozen export lineage"));
    }
    Ok(())
}
