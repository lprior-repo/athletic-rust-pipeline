use super::super::{DatasetLineage, ExportDataset, Loaded};
use super::{generation, hex_digest, invalid, io, ReportResult, SCHEMA_REVISION};
use crate::report::file_digest::{hash_reader, FileDigestError};
use crate::report::io_error;
use serde::{de::DeserializeOwned, Deserialize};
use std::io::{BufReader, Read, Seek};
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    lineage: DatasetLineage,
    #[serde(rename = "data")]
    _data: serde::de::IgnoredAny,
    #[serde(rename = "selected_proofs", default)]
    _selected_proofs: Option<serde::de::IgnoredAny>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Archived {
    lineage: DatasetLineage,
    data: Loaded,
    #[serde(default)]
    selected_proofs: Option<super::super::selected_proofs::SelectedProofsV1>,
}

pub(super) fn reopen(
    path: &Path,
    expected_archive: Option<&str>,
    eligible: impl FnOnce(&DatasetLineage) -> ReportResult<()>,
) -> ReportResult<ExportDataset> {
    let mut reader = io::open_json(path)?;
    verify_commitment(path, &mut reader, expected_archive)?;
    let Header {
        lineage,
        _data: _,
        _selected_proofs: _,
    } = decode_archive(path, &mut reader)?;
    validate(&lineage)?;
    eligible(&lineage)?;
    reader.rewind().map_err(|source| io_error(path, source))?;
    let archived: Archived = decode_archive(path, &mut reader)?;
    verify_commitment(path, &mut reader, expected_archive)?;
    if archived.lineage != lineage {
        return Err(invalid("frozen export lineage changed during preflight"));
    }
    let mut dataset = ExportDataset::from_loaded(archived.data, archived.lineage)?;
    dataset.selected_proofs = archived.selected_proofs;
    if dataset.lineage != lineage {
        return Err(invalid(
            "frozen export input digest does not match its retained data",
        ));
    }
    if let Some(mut proofs) = dataset.selected_proofs.take() {
        if expected_archive.is_none() {
            return Err(invalid(
                "selected proofs require an external frozen archive commitment",
            ));
        }
        let profile = dataset.selected_authority()?.profile().clone();
        dataset.selected_input_origin = selected_replay_origin(&profile)?;
        let fingerprints = dataset.selected_authority()?.fingerprints().clone();
        let directory = super::proof_directory(path)?;
        proofs.bind_directory(&directory)?;
        let hydrated = proofs.hydrate(&mut dataset, &profile, &fingerprints, &directory)?;
        dataset.attach_selected_proofs(proofs, hydrated)?;
    }
    Ok(dataset)
}

fn decode_archive<T: DeserializeOwned>(
    path: &Path,
    reader: &mut BufReader<std::fs::File>,
) -> ReportResult<T> {
    let limit = crate::export::MAX_FROZEN_INPUT_BYTES
        .checked_add(1)
        .ok_or_else(|| invalid("frozen export input byte counter exhausted"))?;
    let mut bounded = reader.take(limit);
    let decoded = io::decode_json(path, &mut bounded);
    if bounded.limit() == 0 {
        return Err(invalid("frozen export input exceeds its byte budget"));
    }
    decoded
}

fn verify_commitment(
    path: &Path,
    reader: &mut BufReader<std::fs::File>,
    expected: Option<&str>,
) -> ReportResult<()> {
    let Some(expected) = expected else {
        return Ok(());
    };
    reader.rewind().map_err(|source| io_error(path, source))?;
    let bytes = archive_length(path, reader)?;
    let digest =
        hash_reader(reader, crate::export::MAX_FROZEN_INPUT_BYTES).map_err(
            |error| match error {
                FileDigestError::Io(source) => io_error(path, source),
                FileDigestError::CountConversion(source) => invalid(&source.to_string()),
                FileDigestError::InvalidReadLength => invalid("invalid file read length"),
                FileDigestError::CounterExhausted => {
                    invalid("frozen export input byte counter exhausted")
                }
                FileDigestError::BudgetExceeded => {
                    invalid("frozen export input exceeds its byte budget")
                }
            },
        )?;
    if digest.bytes != bytes || archive_length(path, reader)? != bytes {
        return Err(invalid("frozen archive changed while hashing"));
    }
    if digest.sha256 != expected {
        return Err(invalid(
            "frozen archive differs from its external commitment",
        ));
    }
    reader.rewind().map_err(|source| io_error(path, source))
}

fn archive_length(path: &Path, reader: &BufReader<std::fs::File>) -> ReportResult<u64> {
    let metadata = reader
        .get_ref()
        .metadata()
        .map_err(|source| io_error(path, source))?;
    if !metadata.file_type().is_file() || metadata.len() > crate::export::MAX_FROZEN_INPUT_BYTES {
        return Err(invalid(&format!(
            "invalid or oversized frozen input {}",
            path.display()
        )));
    }
    Ok(metadata.len())
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

#[cfg(test)]
mod commitment_tests;

#[cfg(test)]
mod origin_tests;

fn selected_replay_origin(
    profile: &census_domain::model::QueryProfileV1,
) -> ReportResult<super::super::SelectedInputOrigin> {
    use census_domain::participation_candidates::participation_policy_v2;
    use census_domain::profile_execution::validate_selected_qualification_profile_v1;
    if profile.input().policies.participation
        == participation_policy_v2().map_err(|error| invalid(&error.to_string()))?
    {
        return Ok(super::super::SelectedInputOrigin::HistoricalReplay);
    }
    let _supported = validate_selected_qualification_profile_v1(profile)
        .map_err(|error| invalid(&error.to_string()))?;
    Ok(super::super::SelectedInputOrigin::Current)
}
