use super::super::fixtures::TestResult;
use super::super::{freeze_v2, objects, to_export_dataset};
use super::evidence::{self, interpretation, EvidenceShape};
use crate::export::{ExportDataset, ExportJobRequest};
use crate::report::{coverage_report, CoverageReport, ReportError};
use census_domain::model::{
    serialized_bytes, CoachLaneResolution, CoachLaneStage, CoverageRow, CoverageScope,
};
use census_store::{InputRecord, LeaseRecord, Store, Table};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;

#[derive(Clone, Copy)]
enum Mutation {
    Coverage,
    CoachLanes,
}

impl Mutation {
    fn apply(self, store: &Store) -> TestResult {
        match self {
            Self::Coverage => {
                let row = CoverageRow::new(CoverageScope::Jurisdiction, "WI")
                    .with("registered", 12)
                    .with("applicable", 12)
                    .with("attempted", 9)
                    .with("captured", 8)
                    .with("parsed", 7)
                    .with("accepted", 6)
                    .with("withheld", 5);
                check!(eq; CoverageReport::publish_coverage_rows(store, &[row])?, 1);
            }
            Self::CoachLanes => {
                let blocked = evidence::lane()?.record(
                    CoachLaneStage::Blocked,
                    None,
                    CoachLaneResolution::unresolved("2026-05-02"),
                )?;
                store.replace(Table::CoachLanes, &blocked)?;
            }
        }
        Ok(())
    }

    fn check_changed(self, live: &ExportDataset) -> TestResult {
        let report = coverage_report(live, Some(2027))?;
        let wi = report
            .jurisdictions
            .iter()
            .find(|row| row.jurisdiction.code() == "WI")
            .ok_or("Wisconsin coverage missing")?;
        match self {
            Self::Coverage => {
                check!(eq; report.source_population.registered, Some(12));
                check!(eq; report.source_population.accepted, Some(6));
                check!(eq; serde_json::to_value(wi.state)?, serde_json::json!("pending"));
                check!(eq; live.coverage_evidence.coach_lanes, vec![evidence::lane()?]);
            }
            Self::CoachLanes => {
                check!(eq; report.source_population.registered, Some(7));
                check!(eq; report.source_population.accepted, Some(3));
                check!(eq; serde_json::to_value(wi.state)?, serde_json::json!("blocked"));
                check!(eq; report.contact.pending, 0);
                check!(eq; report.contact.blocked, 1);
                check!(eq; live.coverage_evidence.rows, vec![evidence::row()]);
            }
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
struct RetainedBytes {
    archive: Vec<u8>,
    marker: Vec<u8>,
    objects: BTreeMap<OsString, Vec<u8>>,
    record: InputRecord,
    lease: Option<LeaseRecord>,
    sequence: u64,
}

impl RetainedBytes {
    fn capture(store: &Store, archive: &Path, directory: &Path, attempt: &str) -> TestResult<Self> {
        Ok(Self {
            archive: std::fs::read(archive)?,
            marker: std::fs::read(directory.join("root.json"))?,
            objects: std::fs::read_dir(objects::directory(directory))?
                .map(|entry| -> TestResult<_> {
                    let entry = entry?;
                    Ok((entry.file_name(), std::fs::read(entry.path())?))
                })
                .collect::<TestResult<_>>()?,
            record: store
                .get_input_record(attempt)?
                .ok_or("retained input record missing")?,
            lease: store.get_lease_record(attempt)?,
            sequence: store.snapshot().sequence(),
        })
    }

    fn unchanged(
        &self,
        store: &Store,
        archive: &Path,
        directory: &Path,
        attempt: &str,
    ) -> TestResult {
        check!(eq; Self::capture(store, archive, directory, attempt)?, *self);
        Ok(())
    }
}

fn stale<T>(result: crate::report::ReportResult<T>) -> TestResult {
    check!(matches!(result, Err(ReportError::Invariant { .. })));
    Ok(())
}

fn retained_after_reopen(mutation: Mutation) -> TestResult {
    let seeded = evidence::seed(EvidenceShape::Both)?;
    let store = seeded.store;
    let job = "frozen-coverage:retained";
    let dataset = ExportDataset::for_job_request(&store, job, &ExportJobRequest::LegacyV2)?;
    evidence::check_shape(&dataset, EvidenceShape::Both)?;
    let root = freeze_v2(&store, job)?;
    let archive = evidence::job_archive(&store, job)?;
    let lineage = dataset.lineage.clone();
    let encoded = serialized_bytes(&dataset)?;
    let interpreted = interpretation(&dataset)?;
    mutation.apply(&store)?;
    let live = ExportDataset::load(&store)?;
    mutation.check_changed(&live)?;
    check!(ne; live.lineage.source_digest, lineage.source_digest);
    check!(ne; live.lineage.input_digest, lineage.input_digest);
    check!(ne; interpretation(&live)?, interpreted);
    stale(dataset.ensure_snapshot(&store.snapshot()))?;
    stale(ExportDataset::for_job_request(
        &store,
        job,
        &ExportJobRequest::LegacyV2,
    ))?;
    let retained = RetainedBytes::capture(&store, &archive, root.directory(), root.attempt())?;
    drop(live);
    drop(dataset);
    drop(store);
    let reopened_store = Store::open(seeded.directory.path())?;
    let reused = freeze_v2(&reopened_store, job)?;
    check!(eq; reused.attempt(), root.attempt());
    check!(eq; reused.manifest_digest(), root.manifest_digest());
    let chunked = to_export_dataset(&reused, &reopened_store)?;
    let archived = ExportDataset::reopen_frozen(&archive)?;
    [chunked, archived]
        .iter()
        .try_for_each(|dataset| -> TestResult {
            check!(eq; dataset.lineage, lineage);
            check!(eq; serialized_bytes(dataset)?, encoded);
            check!(eq; interpretation(dataset)?, interpreted);
            evidence::check_shape(dataset, EvidenceShape::Both)?;
            stale(dataset.ensure_snapshot(&reopened_store.snapshot()))
        })?;
    stale(ExportDataset::for_job_request(
        &reopened_store,
        job,
        &ExportJobRequest::LegacyV2,
    ))?;
    retained.unchanged(&reopened_store, &archive, root.directory(), root.attempt())
}

#[test]
fn frozen_coverage_only_change_is_fenced_and_cannot_hydrate_retained_inputs() -> TestResult {
    retained_after_reopen(Mutation::Coverage)
}

#[test]
fn frozen_coach_lane_only_change_is_fenced_and_cannot_hydrate_retained_inputs() -> TestResult {
    retained_after_reopen(Mutation::CoachLanes)
}
