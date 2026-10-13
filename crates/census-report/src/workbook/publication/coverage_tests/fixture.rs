mod adjustment_roundtrip;

use super::*;
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, CoachLaneObligation, CoachLaneRefusal, CoachLaneResolution, CoachLaneStage,
    CompetitionLevel, CoverageRow, CoverageScope, EventIdentity, EventKind, EventSpecification,
    Evidence, ExactSeconds, Gender, Mark, PendingCoachLane, PerformanceIdentity, PerformanceResult,
    PublishedGraduation, SourceIdentity, SourceNamespace, SourceRef, Sport, TimingMethod,
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
    pub(super) dataset: ExportDataset,
    pub(super) frozen: FrozenRoot,
    pub(super) workbook: PathBuf,
    pub(super) options: Options,
    pub(super) archive: PathBuf,
}

impl Captured {
    pub(super) fn create() -> TestResult<Self> {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path().join("store"))?;
        seed_population(&store)?;
        store.replace(Table::Coverage, &coverage_row(0))?;
        store.replace(Table::CoachLanes, &pending_lane()?)?;
        let frozen = freeze_v2(&store, JOB)?;
        let dataset = ExportDataset::for_job(&store, JOB)?;
        let archive = store.out_dir().join("export-inputs").join(format!(
            "{}.json",
            census_domain::model::serialized_digest(&JOB)?
        ));
        let options = super::super::fence_tests::options(None)?;
        let censuses = crate::workbook::Censuses::of(&dataset, &store.out_dir());
        let workbook = crate::workbook::build_from(&dataset, &store, &options, &censuses)?;
        Ok(Self {
            directory,
            store,
            dataset,
            frozen,
            workbook,
            options,
            archive,
        })
    }
}

fn seed_population(store: &Store) -> TestResult {
    let source = SourceRef::new("wiaa_results", Some("https://example.test/coverage".into()));
    let evidence = Evidence::parsed(source.clone(), "2026-05-01");
    let (mut school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Coverage High",
        "coverage high",
        None,
    );
    school.evidence.push(evidence.clone());
    store.append(Table::Schools, &school)?;
    let mut athlete = CanonicalAthlete::new(
        &school_id,
        "Ada Coverage",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "coverage-1"),
    );
    athlete.evidence.push(evidence.clone());
    athlete.sports.push(Sport::OutdoorTrack);
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source,
    });
    store.append(Table::Athletes, &athlete)?;
    let team = CanonicalTeam {
        id: CanonicalTeam::mint(
            &school_id,
            Sport::OutdoorTrack,
            Gender::Girls,
            SchoolYear::new(2026).ok_or("fixture season")?,
        ),
        school: school_id,
        sport: Sport::OutdoorTrack,
        gender: Gender::Girls,
        school_year: SchoolYear::new(2026).ok_or("fixture season")?,
        level: None,
        source_identities: Vec::new(),
        evidence: Vec::new(),
        retained_conflicts: Vec::new(),
    };
    store.append(Table::Teams, &team)?;
    seed_performance(store, &athlete, &team, evidence)
}

fn seed_performance(
    store: &Store,
    athlete: &CanonicalAthlete,
    team: &CanonicalTeam,
    evidence: Evidence,
) -> TestResult {
    let mut meet = CanonicalMeet::new_checked(
        Some(UsJurisdiction::Wisconsin),
        "Coverage Invitational",
        "2026-05-01",
        CompetitionLevel::Invitational,
    )?;
    meet.sports.push(Sport::OutdoorTrack);
    store.append(Table::Meets, &meet)?;
    let event = crate::test_events::owned(CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::Track400m,
            gender: Gender::Girls,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?);
    store.append(Table::Events, &event)?;
    let mut performance = CanonicalPerformance::new(
        PerformanceIdentity {
            athlete: &athlete.id,
            event: &event.id,
            meet: &meet.id,
            date: "2026-05-01",
            source_key: "coverage-result",
        },
        PerformanceResult {
            team: &team.id,
            mark: Mark::TimeSeconds(ExactSeconds::parse("50.00")?),
            wind_mps: None,
            adjustment: None,
            place: Some(1),
        },
    )?;
    performance.timing = Some(TimingMethod::Fat);
    performance.source_athlete = athlete.source.clone();
    performance.evidence.push(evidence);
    store.append(Table::Performances, &performance)?;
    Ok(())
}

fn coverage_row(extra: u64) -> CoverageRow {
    CoverageRow::new(CoverageScope::Jurisdiction, "WI")
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
        jurisdiction: UsJurisdiction::Wisconsin,
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
