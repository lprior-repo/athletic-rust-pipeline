use super::{seal, SealEvidence, TestResult, Verification};
use census_domain::model::{
    AccessBlockKind, CanonicalSchool, CensusRun, CollectionSnapshot, CoverageRow, CoverageScope,
    Evidence, GradYear, ReviewCase, SourceAccessCondition, SourceRef,
};
use census_domain::UsJurisdiction;
use census_report::report::Scope;
use census_store::{Store, Table};
use std::path::PathBuf;

use crate::census::seal::{JournalCounts, SealRequest};

mod readback;
mod witness;

use readback::assert_readback;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Shape {
    EmptyOptional,
    EmptyReview,
}

pub(super) struct Fixture {
    pub(super) directory: tempfile::TempDir,
    pub(super) store: Store,
    pub(super) path: PathBuf,
    pub(super) run: CensusRun,
    shape: Shape,
    with_access: bool,
}

impl Fixture {
    pub(super) fn create(shape: Shape, with_access: bool) -> TestResult<Self> {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path().join("store"))?;
        let run = super::super::bind_census_run(&store, 2026, 1)?;
        let (mut school, _) = CanonicalSchool::new(
            UsJurisdiction::Wisconsin,
            "Measured Zero High",
            "measured zero high",
            None,
        );
        school.evidence.push(Evidence::parsed(
            SourceRef::new(
                "milesplit",
                Some("https://example.test/measured-zero/school".into()),
            ),
            "2026-10-07",
        ));
        store.append(Table::Schools, &school)?;
        if with_access {
            store.replace(Table::SourceAccess, &retained_access())?;
        }
        if shape == Shape::EmptyReview {
            add_optional_artifacts(&store)?;
        }
        assert_shape(&store, shape, with_access)?;
        let path = census_report::workbook::build(&store, &super::super::full_options()?)?;
        let fixture = Self {
            directory,
            store,
            path,
            run,
            shape,
            with_access,
        };
        fixture.assert_shape()?;
        Ok(fixture)
    }

    pub(super) fn request(&self, write: bool) -> SealRequest {
        SealRequest {
            grad_year: 2027,
            scope: Scope::AllSources,
            jurisdictions: Vec::new(),
            workbook: Some(self.path.clone()),
            write,
            journal: Some(JournalCounts {
                jurisdiction_sweeps: Some(0),
                source_objects: Some(0),
                silent_sources: Vec::new(),
            }),
            source_failures: Some(0),
            run: Some(self.run),
        }
    }

    pub(super) fn inspect(&self) -> TestResult<SealEvidence> {
        Ok(seal(&self.store, &self.request(false))?.evidence)
    }

    pub(super) fn witness_bytes(&self) -> TestResult<Vec<u8>> {
        Ok(std::fs::read(self.store.out_dir().join("seal.json"))?)
    }

    pub(super) fn sequence(&self) -> u64 {
        self.store.snapshot().sequence()
    }

    pub(super) fn source_rows(&self) -> TestResult<serde_json::Value> {
        let snapshot = self.store.snapshot();
        Ok(serde_json::json!({
            "schools": snapshot.scan::<CanonicalSchool>(Table::Schools)?,
            "access": snapshot.scan::<SourceAccessCondition>(Table::SourceAccess)?,
            "snapshots": snapshot.scan::<CollectionSnapshot>(Table::Snapshots)?,
            "coverage": snapshot.scan::<CoverageRow>(Table::Coverage)?,
            "cases": snapshot.scan::<ReviewCase>(Table::ReviewCases)?,
        }))
    }

    pub(super) fn publication_bytes(&self) -> TestResult<Vec<(String, Vec<u8>)>> {
        let generation = self.path.parent().ok_or("generation directory")?;
        let mut rows = Vec::new();
        for entry in std::fs::read_dir(generation)? {
            let entry = entry?;
            check!(
                entry.file_type()?.is_file(),
                "generation artifact is a regular file"
            );
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "artifact name")?;
            rows.push((name, std::fs::read(entry.path())?));
        }
        let pointer = std::fs::read_link(self.store.out_dir().join("publication/current"))?;
        let pointer = pointer
            .into_os_string()
            .into_string()
            .map_err(|_| "current pointer")?;
        rows.push(("current-pointer".into(), pointer.into_bytes()));
        rows.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(rows)
    }

    pub(super) fn reopen(self) -> TestResult<Self> {
        let Self {
            directory,
            store,
            path,
            run,
            shape,
            with_access,
        } = self;
        let root = store.root().to_path_buf();
        store.flush()?;
        drop(store);
        let store = Store::open(root)?;
        Ok(Self {
            directory,
            store,
            path,
            run,
            shape,
            with_access,
        })
    }

    pub(super) fn assert_bundle(&self) -> TestResult {
        self.assert_shape()?;
        let inspection = super::super::super::workbook::inspect_workbook(
            &self.path,
            &self.store,
            GradYear::CO2027,
            Scope::AllSources,
            &[],
        )?;
        check!(eq; inspection.check.mapped_athletes, 0);
        check!(eq; inspection.check.export_verified, Verification::Verified);
        check!(eq; inspection.check.counts_reconciled, Verification::Verified);
        check!(eq; inspection.check.coverage_reconciled, Verification::Verified);
        check!(eq; inspection.check.metrics_reconciled, Verification::Verified);
        check!(inspection.check.discrepancies.is_empty());
        check!(eq; inspection.dataset.schools.len(), 1);
        check!(eq;
            serde_json::to_value(inspection.dataset.schools.values().collect::<Vec<_>>())?,
            serde_json::to_value(self.store.scan::<CanonicalSchool>(Table::Schools)?)?
        );
        check!(inspection.dataset.athletes.is_empty());
        check!(inspection.dataset.performances.is_empty());
        check!(inspection.dataset.review_cases.is_empty());
        check!(inspection.dataset.verdicts.is_empty());
        check!(eq; inspection.dataset.source_access.len(), usize::from(self.with_access));
        check!(inspection.dataset.coverage_evidence.coach_lanes.is_empty());
        let coverage_rows = usize::from(self.shape == Shape::EmptyReview);
        check!(eq; inspection.dataset.coverage_evidence.rows.len(), coverage_rows);
        if self.shape == Shape::EmptyReview {
            let row = inspection
                .dataset
                .coverage_evidence
                .rows
                .first()
                .ok_or("frozen coverage row")?;
            check!(eq; row.subject, "WI");
            check!(eq; row.scope, CoverageScope::Jurisdiction);
            for metric in [
                "registered",
                "applicable",
                "attempted",
                "captured",
                "parsed",
                "accepted",
            ] {
                check!(eq; row.metrics.get(metric), Some(&1));
            }
            check!(eq; row.metrics.get("withheld"), Some(&0));
        }
        drop(inspection);
        check!(eq;
            census_report::workbook::publication::current_workbook(
                &self.store.out_dir().join("publication")
            )?,
            self.path
        );
        assert_readback(&self.path)?;
        Ok(())
    }

    fn assert_shape(&self) -> TestResult {
        assert_shape(&self.store, self.shape, self.with_access)
    }
}

fn assert_shape(store: &Store, shape: Shape, with_access: bool) -> TestResult {
    let optional = u64::from(shape == Shape::EmptyReview);
    for (table, count) in [
        (Table::Schools, 1),
        (Table::SourceAccess, u64::from(with_access)),
        (Table::Snapshots, optional),
        (Table::Coverage, optional),
        (Table::Athletes, 0),
        (Table::Teams, 0),
        (Table::Meets, 0),
        (Table::Events, 0),
        (Table::Performances, 0),
        (Table::Conflicts, 0),
        (Table::ReviewCases, 0),
        (Table::IdentityVerdicts, 0),
        (Table::CoachLanes, 0),
    ] {
        check!(eq; store.walk_table(table)?.rows, count, "fixture table {table:?}");
    }
    Ok(())
}

fn retained_access() -> SourceAccessCondition {
    SourceAccessCondition::new(
        "milesplit",
        "example.test",
        AccessBlockKind::Timeout,
        0,
        "2026-09-22T00:00:00Z",
        "synthetic prior timeout retained after fixture acquisition",
    )
    .with_cooldown_until(Some("2026-09-23T00:00:00Z".into()))
}

fn add_optional_artifacts(store: &Store) -> TestResult {
    let observations = store.snapshot().appended_counts()?.into_iter().collect();
    let mut snapshot = CollectionSnapshot::new("derive", "2026-10-07");
    snapshot.observations = observations;
    store.replace(Table::Snapshots, &snapshot)?;
    let coverage = CoverageRow::new(CoverageScope::Jurisdiction, "WI")
        .with("registered", 1)
        .with("applicable", 1)
        .with("attempted", 1)
        .with("captured", 1)
        .with("parsed", 1)
        .with("accepted", 1)
        .with("withheld", 0);
    store.replace(Table::Coverage, &coverage)?;
    Ok(())
}
