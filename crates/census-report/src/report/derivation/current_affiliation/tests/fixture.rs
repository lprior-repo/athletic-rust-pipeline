use super::*;
use census_domain::model::{
    CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool, CanonicalTeam,
    CompetitionLevel, EventIdentity, EventKind, EventSpecification, Evidence, ExactSeconds, Gender,
    GradYear, Mark, SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_store::{Store, Table};

pub(super) struct Fixture {
    pub(super) directory: tempfile::TempDir,
    pub(super) dataset: ExportDataset,
    pub(super) store: Store,
    pub(super) athlete: AthleteId,
    pub(super) previous: SchoolId,
    pub(super) current: SchoolId,
}

impl Fixture {
    pub(super) fn new() -> TestResult<Self> {
        Self::at_cutoff("2026-10-01")
    }

    pub(super) fn at_cutoff(as_of: &str) -> TestResult<Self> {
        let mut fixture = Self::retained()?;
        fixture.dataset.bind_selection_for_test(
            ProfileId::parse(&"a".repeat(64))?,
            ProfileDate::parse(as_of)?,
        )?;
        fixture.dataset.bind_participation_rule_for_test(
            census_domain::participation_candidates::participation_policy_v2()?,
        )?;
        fixture
            .dataset
            .bind_public_identity_fixture_from_store(&fixture.store)?;
        Ok(fixture)
    }

    pub(super) fn retained() -> TestResult<Self> {
        let directory = tempfile::tempdir()?;
        Store::create_public_identity_qualification_v3(directory.path())?;
        let (store, writer) =
            Store::open_public_identity_qualification_v3(directory.path())?.into_parts();
        writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
        let previous = school(&store, UsJurisdiction::Wisconsin, "Previous school")?;
        let current = school(&store, UsJurisdiction::Michigan, "Current school")?;
        let mut athlete = CanonicalAthlete::new(
            &previous,
            "Transfer runner",
            GradYear::CO2027,
            Gender::Girls,
            SourceIdentity::new(SourceNamespace::MilesplitAthlete, "9701"),
        )?;
        athlete.evidence = vec![evidence()];
        store.append(Table::Athletes, &athlete)?;
        append_performance(&store, &athlete)?;
        writer.establish_source_bound(
            &store,
            "current-affiliation-report-source-birth",
            &athlete.id.cast(),
            store.snapshot().revision(),
            &writer.head(&store)?,
        )?;
        let dataset = ExportDataset::load_public_identity_fixture(&store)?;
        Ok(Self {
            directory,
            dataset,
            store,
            athlete: athlete.id,
            previous,
            current,
        })
    }
}

fn evidence() -> Evidence {
    Evidence::parsed(
        SourceRef::new(
            "milesplit_roster",
            Some("https://wi.milesplit.com/athletes/9701".into()),
        ),
        "2026-09-01",
    )
}

fn school(store: &Store, state: UsJurisdiction, name: &str) -> TestResult<SchoolId> {
    let (mut school, id) = CanonicalSchool::new(state, name, name, None);
    school.evidence = vec![evidence()];
    store.append(Table::Schools, &school)?;
    Ok(id)
}

fn append_performance(store: &Store, athlete: &CanonicalAthlete) -> TestResult {
    let season = SchoolYear::new(2025).ok_or("fixture season")?;
    let team = CanonicalTeam {
        id: CanonicalTeam::mint(&athlete.school, Sport::OutdoorTrack, Gender::Girls, season),
        school: athlete.school.clone(),
        sport: Sport::OutdoorTrack,
        gender: Gender::Girls,
        school_year: season,
        level: None,
        source_identities: Vec::new(),
        evidence: vec![evidence()],
        retained_conflicts: Vec::new(),
    };
    store.append(Table::Teams, &team)?;
    let mut meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "Old school meet",
        "2026-05-01",
        CompetitionLevel::Invitational,
    );
    meet.evidence = vec![evidence()];
    store.append(Table::Meets, &meet)?;
    let mut event = crate::test_events::owned(CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::Track400m,
            gender: Gender::Girls,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?);
    event.evidence = vec![evidence()];
    store.append(Table::Events, &event)?;
    let performance = CanonicalPerformance {
        id: CanonicalPerformance::mint(
            &athlete.id,
            &meet.id,
            &event.id,
            "2026-05-01",
            "old-result",
        ),
        athlete: athlete.id.clone(),
        team: team.id,
        event: event.id,
        meet: meet.id,
        date: "2026-05-01".into(),
        mark: Mark::TimeSeconds(ExactSeconds::parse("60.00")?),
        adjustment: None,
        wind_mps: None,
        place: None,
        heat: None,
        round: None,
        timing: None,
        observed_grade: None,
        evidence: vec![evidence()],
        source_key: "old-result".into(),
        source_athlete: None,
        retained_conflicts: Vec::new(),
        combined_completion: None,
    };
    store.append(Table::Performances, &performance)?;
    Ok(())
}
