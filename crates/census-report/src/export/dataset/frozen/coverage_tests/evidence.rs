use super::super::fixtures::{seeded_store_with_source_athlete, SeededStore, TestResult};
use crate::bests::{self, SharedSelection};
use crate::export::ExportDataset;
use crate::report::{coverage_report, CoverageReport, Scope};
use census_domain::model::performance_comparison::SurfaceClass;
use census_domain::model::{
    serialized_bytes, serialized_digest, CanonicalAthlete, CanonicalEvent, CanonicalMeet,
    CanonicalPerformance, CoachLaneObligation, Confidence, CoverageRow, CoverageScope, EventKind,
    Evidence, GradYear, PendingCoachLane, PublishedGraduation, SchoolYear, SourceEventLabel,
    SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::path::PathBuf;

const RESULT_URL: &str = "https://example.test/frozen-coverage/results";

#[derive(Clone, Copy)]
pub(super) enum EvidenceShape {
    Both,
    RowsOnly,
    LanesOnly,
    Empty,
}

impl EvidenceShape {
    pub(super) const fn has_rows(self) -> bool {
        matches!(self, Self::Both | Self::RowsOnly)
    }

    pub(super) const fn has_lanes(self) -> bool {
        matches!(self, Self::Both | Self::LanesOnly)
    }
}

pub(super) fn row() -> CoverageRow {
    CoverageRow::new(CoverageScope::Jurisdiction, "WI")
        .with("registered", 7)
        .with("applicable", 7)
        .with("attempted", 6)
        .with("captured", 5)
        .with("parsed", 4)
        .with("accepted", 3)
        .with("withheld", 2)
}

pub(super) fn lane() -> TestResult<CoachLaneObligation> {
    Ok(CoachLaneObligation::pending(PendingCoachLane {
        jurisdiction: UsJurisdiction::Wisconsin,
        season: SchoolYear::new(2026).ok_or("fixture season")?,
        revision: 1,
        arm: "frozen-coverage-fixture".to_string(),
        arm_basis: "synthetic state association directory".to_string(),
        observed_on: "2026-05-01".to_string(),
    })?)
}

pub(super) fn seed(shape: EvidenceShape) -> TestResult<SeededStore> {
    let seeded = seeded_store_with_source_athlete(Some(SourceIdentity::new(
        SourceNamespace::MilesplitAthlete,
        "1",
    )))?;
    seed_accepted_discus(&seeded.store)?;
    if shape.has_rows() {
        check!(eq; CoverageReport::publish_coverage_rows(&seeded.store, &[row()])?, 1);
    }
    if shape.has_lanes() {
        seeded.store.replace(Table::CoachLanes, &lane()?)?;
    }
    Ok(seeded)
}

fn seed_accepted_discus(store: &Store) -> TestResult {
    let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
    check!(eq; performances.len(), 1);
    let mut performance = performances
        .into_iter()
        .next()
        .ok_or("fixture performance missing")?;
    let mut athlete = store
        .scan::<CanonicalAthlete>(Table::Athletes)?
        .into_iter()
        .find(|athlete| athlete.id == performance.athlete)
        .ok_or("fixture athlete missing")?;
    let roster = SourceRef::new(
        "wiaa_roster",
        Some("https://example.test/frozen-coverage/roster".into()),
    );
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: roster.clone(),
    });
    athlete
        .evidence
        .push(Evidence::parsed(roster, "2026-05-01"));
    athlete.sports.push(Sport::OutdoorTrack);
    check!(eq; athlete.derived_cohort_confidence(), Some(Confidence::HIGH));
    store.append(Table::Athletes, &athlete)?;
    let source = SourceRef::new("wiaa_results", Some(RESULT_URL.into()));
    let evidence = Evidence::parsed(source.clone(), "2026-05-01");
    let mut meet = store
        .scan::<CanonicalMeet>(Table::Meets)?
        .into_iter()
        .find(|meet| meet.id == performance.meet)
        .ok_or("fixture meet missing")?;
    meet.sports.push(Sport::OutdoorTrack);
    meet.evidence.push(evidence.clone());
    meet.source_urls.push(RESULT_URL.into());
    store.append(Table::Meets, &meet)?;
    let mut event = store
        .scan::<CanonicalEvent>(Table::Events)?
        .into_iter()
        .find(|event| event.id == performance.event)
        .ok_or("fixture event missing")?;
    check!(eq; event.kind, EventKind::Discus);
    event.source_labels.push(SourceEventLabel {
        source,
        label: "Discus Throw".into(),
    });
    event.evidence.push(evidence.clone());
    store.append(Table::Events, &event)?;
    performance.evidence.push(evidence);
    store.append(Table::Performances, &performance)?;
    Ok(())
}

pub(super) fn job_archive(store: &Store, job: &str) -> TestResult<PathBuf> {
    Ok(store
        .out_dir()
        .join("export-inputs")
        .join(format!("{}.json", serialized_digest(&job)?)))
}

#[derive(Debug, PartialEq)]
pub(super) struct Interpretation {
    coverage: serde_json::Value,
    selections: Vec<SharedSelection>,
}

pub(super) fn interpretation(dataset: &ExportDataset) -> TestResult<Interpretation> {
    Ok(Interpretation {
        coverage: serde_json::to_value(coverage_report(dataset, Some(2027))?)?,
        selections: bests::build_from_dataset(
            dataset,
            &bests::Options {
                scope: Scope::AllSources,
                grad_year: Some(2027),
                limit: None,
                jurisdictions: Vec::new(),
            },
        )?,
    })
}

fn check_report(dataset: &ExportDataset, shape: EvidenceShape) -> TestResult {
    let report = coverage_report(dataset, Some(2027))?;
    let wi = report
        .jurisdictions
        .iter()
        .find(|entry| entry.jurisdiction.code() == "WI")
        .ok_or("Wisconsin coverage missing")?;
    let population = if shape.has_rows() {
        [
            Some(7),
            Some(7),
            Some(6),
            Some(5),
            Some(4),
            Some(3),
            Some(2),
        ]
    } else {
        [None; 7]
    };
    [&report.source_population, &wi.source_population]
        .into_iter()
        .try_for_each(|observed| -> TestResult {
            check!(eq; [observed.registered, observed.applicable, observed.attempted,
                observed.captured, observed.parsed, observed.accepted, observed.withheld], population);
            Ok(())
        })?;
    let state = if shape.has_lanes() {
        "pending"
    } else {
        "unresearched"
    };
    check!(eq; serde_json::to_value(wi.state)?, serde_json::json!(state));
    check!(eq; report.contact.pending, usize::from(shape.has_lanes()));
    check!(eq; report.contact.never_attempted, if shape.has_lanes() { 48 } else { 49 });
    Ok(())
}

pub(super) fn check_shape(dataset: &ExportDataset, shape: EvidenceShape) -> TestResult {
    let evidence = &dataset.coverage_evidence;
    let expected_rows = if shape.has_rows() {
        vec![row()]
    } else {
        Vec::new()
    };
    let expected_lanes = if shape.has_lanes() {
        vec![lane()?]
    } else {
        Vec::new()
    };
    check!(eq; evidence.rows, expected_rows);
    check!(eq; evidence.coach_lanes, expected_lanes);
    let expected = format!(
        ",\"coverage_evidence\":{},\"relays\":{}}}",
        String::from_utf8(serialized_bytes(evidence)?)?,
        String::from_utf8(serialized_bytes(&dataset.relays)?)?
    );
    check!(serialized_bytes(dataset)?.ends_with(expected.as_bytes()));
    check_report(dataset, shape)?;
    let interpreted = interpretation(dataset)?;
    check!(eq; interpreted.selections.len(), 1);
    let selection = interpreted
        .selections
        .first()
        .ok_or("normalized selection missing")?;
    check!(eq; selection.result.normalized, Some(45.72));
    check!(eq; selection.result.value, 45_720_000);
    check!(eq; selection.key.event_kind, EventKind::Discus);
    check!(eq; selection.key.surface, SurfaceClass::Outdoor);
    check!(eq; selection.athlete.grad_year, 2027);
    check!(eq; selection.source.result_url, RESULT_URL);
    check!(eq; selection.source.source_athlete, "1");
    check!(eq; selection.source.source_key, "fixture-1");
    Ok(())
}
