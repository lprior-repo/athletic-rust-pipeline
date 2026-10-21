mod adjustment_roundtrip;

use super::*;
use crate::export::selected_test_fixture::{coverage_time_fixture, CoverageTimeFixture};
use crate::export::ExportJobRequest;
use census_domain::model::{
    CoachLaneObligation, CoachLaneRefusal, CoachLaneResolution, CoachLaneStage, CoverageRow,
    CoverageScope, PendingCoachLane, ProfileSelectionV1, SchoolYear, SourceRef,
};
use census_domain::UsJurisdiction;
use census_store::Table;

pub(super) const JOB: &str = "publication:captured-coverage";
const OBSERVED_ON: &str = "2026-10-11";

#[derive(Clone, Copy)]
pub(super) enum Mutation {
    CoverageOnly,
    CoachLanesOnly,
    Both,
}

pub(super) struct Captured {
    pub(super) directory: tempfile::TempDir,
    pub(super) store: Store,
    pub(super) selection: ProfileSelectionV1,
    pub(super) dataset: ExportDataset,
    pub(super) workbook: PathBuf,
    pub(super) options: Options,
    pub(super) archive: PathBuf,
    pub(super) digest: String,
}

impl Captured {
    pub(super) async fn create() -> TestResult<Self> {
        let fixture =
            coverage_time_fixture(5000).map_err(|error| -> Box<dyn std::error::Error> { error })?;
        fixture.store.replace(Table::Coverage, &coverage_row(0))?;
        fixture.store.replace(Table::CoachLanes, &pending_lane()?)?;
        let request = ExportJobRequest::SelectedPublicationV1 {
            selection: fixture.selection.clone(),
        };
        let dataset = ExportDataset::for_job_request(&fixture.store, JOB, &request)?;
        let options = crate::workbook::options_for_selected_dataset(&dataset, None)?;
        let censuses = crate::workbook::Censuses::of(
            &dataset,
            &fixture.store.out_dir(),
            &options.jurisdictions,
        )?;
        let workbook = crate::workbook::build_from(&dataset, &fixture.store, &options, &censuses)?;
        let archive = fixture.directory.path().join("captured-frozen.json");
        dataset.save_frozen(&archive)?;
        let digest = dataset.archive_digest()?;
        let CoverageTimeFixture {
            directory,
            store,
            selection,
            ..
        } = fixture;
        Ok(Self {
            directory,
            store,
            selection,
            dataset,
            workbook,
            options,
            archive,
            digest,
        })
    }
}

fn coverage_row(extra: u64) -> CoverageRow {
    CoverageRow::new(CoverageScope::Jurisdiction, "CO")
        .with("registered", 7 + extra)
        .with("applicable", 7 + extra)
        .with("attempted", 6 + extra)
        .with("captured", 5 + extra)
        .with("parsed", 4 + extra)
        .with("accepted", 3 + extra)
        .with("withheld", 2 + extra)
}

fn pending_lane() -> TestResult<CoachLaneObligation> {
    Ok(CoachLaneObligation::pending(PendingCoachLane {
        jurisdiction: UsJurisdiction::Colorado,
        season: SchoolYear::new(2026).ok_or("fixture season")?,
        revision: 1,
        arm: "synthetic_coach_directory".into(),
        arm_basis: "fixed local publication coverage fixture".into(),
        observed_on: OBSERVED_ON.into(),
    })?)
}

pub(super) fn mutate(store: &Store, mutation: Mutation) -> TestResult {
    if matches!(mutation, Mutation::CoverageOnly | Mutation::Both) {
        store.replace(Table::Coverage, &coverage_row(10))?;
    }
    if matches!(mutation, Mutation::CoachLanesOnly | Mutation::Both) {
        let refusal = CoachLaneRefusal::new(
            SourceRef::id("synthetic_coach_directory"),
            "fixed local directory refusal",
            OBSERVED_ON,
        )?;
        let blocked = pending_lane()?.record(
            CoachLaneStage::Blocked,
            Some(refusal),
            CoachLaneResolution::unresolved(OBSERVED_ON),
        )?;
        store.replace(Table::CoachLanes, &blocked)?;
    }
    Ok(())
}
