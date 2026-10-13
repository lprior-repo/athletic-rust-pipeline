use super::fixtures::{read_back, TestResult};
use super::inventory::StreamInventory;
use super::stream::StreamKind;
use super::{freeze_v2, objects, to_export_dataset, FrozenRoot, INPUT_TABLES};
use crate::export::ExportDataset;
use census_domain::model::{serialized_bytes, serialized_digest, CoachLaneObligation, CoverageRow};
use census_store::{Entity, Store, Table};

mod archive;
mod evidence;
mod retained;

use evidence::{interpretation, seed, EvidenceShape};

fn streamed<T: Entity>(root: &FrozenRoot, kind: StreamKind) -> TestResult<Vec<T>> {
    let reference = root
        .manifest()
        .stream(kind)
        .ok_or("missing evidence stream")?;
    let directory = objects::directory(root.directory());
    let encoded = objects::read(
        &directory,
        &reference.inventory.digest,
        objects::PAGE_CEILING,
    )?;
    let page = StreamInventory::decode(&encoded)?;
    let chunks = page
        .chunks()
        .iter()
        .map(|chunk| {
            Ok(objects::read(
                &directory,
                &chunk.sha256,
                objects::CHUNK_CEILING,
            )?)
        })
        .collect::<TestResult<Vec<_>>>()?;
    read_back(kind, &chunks, &page)
}

fn check_manifest(root: &FrozenRoot, store: &Store, dataset: &ExportDataset) -> TestResult {
    let manifest = root.manifest();
    check!(eq; manifest.lineage.schema_revision, 3);
    check!(eq; manifest.lineage.policy_revision, 15);
    check!(eq; manifest.streams.len(), 15);
    check!(eq; manifest.source_fingerprints.len(), 16);
    check!(eq; StreamKind::ALL.len(), 15);
    check!(eq; INPUT_TABLES.len(), 16);
    check!(eq; StreamKind::CoverageRows.index(), 12);
    check!(eq; StreamKind::CoachLanes.index(), 13);
    check!(eq; StreamKind::CoverageRows.wire(), "coverage_rows");
    check!(eq; StreamKind::CoachLanes.wire(), "coach_lanes");
    check!(eq; INPUT_TABLES.get(13), Some(&Table::Coverage));
    check!(eq; INPUT_TABLES.get(14), Some(&Table::CoachLanes));
    check!(eq; INPUT_TABLES.get(15), Some(&Table::RelayResults));
    check!(eq; StreamKind::Relays.index(), 14);
    [Table::Coverage, Table::CoachLanes]
        .into_iter()
        .try_for_each(|table| -> TestResult {
            let fingerprint = manifest
                .fingerprint(table)
                .ok_or("missing evidence fingerprint")?;
            check!(eq; fingerprint.digest, store.snapshot().tables_digest(&[table])?);
            Ok(())
        })?;
    check!(eq; serialized_digest(dataset)?, manifest.lineage.input_digest);
    check!(eq; manifest.legacy_logical_digest.as_deref(), Some(manifest.lineage.input_digest.as_str()));
    check!(eq; streamed::<CoverageRow>(root, StreamKind::CoverageRows)?, dataset.coverage_evidence.rows);
    check!(eq; streamed::<CoachLaneObligation>(root, StreamKind::CoachLanes)?, dataset.coverage_evidence.coach_lanes);
    Ok(())
}

fn round_trip(shape: EvidenceShape) -> TestResult {
    let seeded = seed(shape)?;
    let store = &seeded.store;
    let live = ExportDataset::load(store)?;
    evidence::check_shape(&live, shape)?;
    let root = freeze_v2(store, "frozen-coverage:round-trip")?;
    let chunked = to_export_dataset(&root, store)?;
    check_manifest(&root, store, &chunked)?;
    check!(eq; serialized_bytes(&chunked)?, serialized_bytes(&live)?);
    check!(eq; interpretation(&chunked)?, interpretation(&live)?);
    let archived = ExportDataset::for_job(store, "frozen-coverage:round-trip")?;
    let archive = evidence::job_archive(store, "frozen-coverage:round-trip")?;
    let reopened = ExportDataset::reopen_frozen(&archive)?;
    [archived, reopened]
        .iter()
        .try_for_each(|dataset| -> TestResult {
            check!(eq; dataset.lineage, chunked.lineage);
            check!(eq; serialized_bytes(dataset)?, serialized_bytes(&chunked)?);
            check!(eq; interpretation(dataset)?, interpretation(&chunked)?);
            evidence::check_shape(dataset, shape)
        })?;
    Ok(())
}

#[test]
fn frozen_coverage_nonempty_vectors_match_json_chunks_and_logical_digest() -> TestResult {
    round_trip(EvidenceShape::Both)
}

#[test]
fn frozen_coverage_empty_rows_keep_nonempty_coach_lanes() -> TestResult {
    round_trip(EvidenceShape::LanesOnly)
}

#[test]
fn frozen_coverage_empty_coach_lanes_keep_nonempty_rows() -> TestResult {
    round_trip(EvidenceShape::RowsOnly)
}

#[test]
fn frozen_coverage_explicit_empty_vectors_remain_unmeasured() -> TestResult {
    round_trip(EvidenceShape::Empty)
}
