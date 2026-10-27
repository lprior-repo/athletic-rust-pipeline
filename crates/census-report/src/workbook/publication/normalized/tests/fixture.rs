use super::TestResult;
use crate::bests::SharedSelection;
use crate::export::ExportDataset;
use crate::report::Scope;
use crate::workbook::Options;
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, CompetitionLevel, EventIdentity, EventKind, EventSpecification, Evidence,
    ExactSeconds, Gender, GradYear, Mark, PerformanceIdentity, PerformanceResult,
    PublishedGraduation, SchoolId, SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
    TimingMethod,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

pub(super) struct Population {
    pub(super) dataset: ExportDataset,
    pub(super) options: Options,
    pub(super) bests: Vec<SharedSelection>,
    pub(super) school: SchoolId,
    _directory: tempfile::TempDir,
    _store: Store,
}

impl Population {
    fn build(
        name: &str,
        marks: &[&str],
        indoor_second: bool,
        historical_school: bool,
    ) -> TestResult<(Self, Option<SchoolId>)> {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path().join("store"))?;
        let (mut school, school_id) = CanonicalSchool::new(
            UsJurisdiction::Michigan,
            "Fixture High",
            "fixture high",
            Some("Lansing"),
        );
        let evidence = Evidence::parsed(
            SourceRef::new(
                "captured-fixture-results",
                Some("https://example.test/results".into()),
            ),
            "2026-05-01",
        );
        school.evidence.push(evidence.clone());
        store.append(Table::Schools, &school)?;
        let mut athlete = CanonicalAthlete::new(
            &school_id,
            name,
            GradYear::CO2027,
            Gender::Girls,
            SourceIdentity::new(SourceNamespace::MilesplitAthlete, "123456")
                .with_url("https://example.test/athletes/123456"),
        )?;
        athlete.evidence.push(evidence.clone());
        athlete.sports.push(Sport::OutdoorTrack);
        if indoor_second {
            athlete.sports.push(Sport::IndoorTrack);
        }
        athlete.published_graduations.push(PublishedGraduation {
            grad_year: GradYear::CO2027,
            source: evidence.source.clone(),
        });
        store.append(Table::Athletes, &athlete)?;
        let outdoor_team = team(&school_id, Sport::OutdoorTrack);
        store.append(Table::Teams, &outdoor_team)?;
        let mut historical_id = None;
        let mut historical_team = None;
        if historical_school {
            let (historical, hid) = CanonicalSchool::new(
                UsJurisdiction::NewJersey,
                "Prior High",
                "prior high",
                Some("Jersey City"),
            );
            store.append(Table::Schools, &historical)?;
            let retained_team = team(&hid, Sport::OutdoorTrack);
            store.append(Table::Teams, &retained_team)?;
            historical_team = Some(retained_team);
            historical_id = Some(hid);
        }
        for (index, mark) in marks.iter().enumerate() {
            let sport = if indoor_second && index == 1 {
                Sport::IndoorTrack
            } else {
                Sport::OutdoorTrack
            };
            let perf_team = if let Some(retained_team) = &historical_team {
                retained_team.id.clone()
            } else if sport == Sport::IndoorTrack {
                let indoor_team = team(&school_id, sport);
                store.append(Table::Teams, &indoor_team)?;
                indoor_team.id
            } else {
                outdoor_team.id.clone()
            };
            performance(
                &store,
                &athlete,
                &perf_team,
                evidence.clone(),
                index,
                mark,
                sport,
            )?;
        }
        coaches(&store, &school_id)?;
        let mut dataset = ExportDataset::load(&store)?;
        dataset.lineage.policy_revision = 21;
        let options = Options {
            address_corpus: None,
            grad_year: Some(2027),
            out: None,
            limit: None,
            scope: Scope::AllSources,
            jurisdictions: vec![UsJurisdiction::Michigan],
            school_year: SchoolYear::DEFAULT,
        };
        let bests = crate::bests::build_from_dataset(
            &dataset,
            &crate::bests::Options {
                scope: options.scope,
                grad_year: options.grad_year,
                limit: None,
                jurisdictions: options.jurisdictions.clone(),
            },
        )?;
        Ok((
            Self {
                dataset,
                options,
                bests,
                school: school_id,
                _directory: directory,
                _store: store,
            },
            historical_id,
        ))
    }

    pub(super) fn new(name: &str, marks: &[&str]) -> TestResult<Self> {
        let (pop, _) = Self::build(name, marks, false, false)?;
        Ok(pop)
    }

    pub(super) fn new_with_historical_school(
        name: &str,
        marks: &[&str],
    ) -> TestResult<(Self, SchoolId)> {
        let (pop, hist) = Self::build(name, marks, false, true)?;
        Ok((pop, hist.ok_or("historical school missing from fixture")?))
    }

    pub(super) fn new_with_indoor_second_meet(name: &str, marks: &[&str]) -> TestResult<Self> {
        let (pop, _) = Self::build(name, marks, true, false)?;
        Ok(pop)
    }
}

fn coaches(store: &Store, school: &SchoolId) -> TestResult {
    use census_domain::model::{CanonicalCoach, CoachRole};
    for (role, name) in [
        (CoachRole::HeadCoach, "Pat Head"),
        (CoachRole::AssistantCoach, "Sam Assistant"),
    ] {
        let mut coach =
            CanonicalCoach::new(school, name, Some(Sport::OutdoorTrack), Gender::Girls, role);
        coach.professional_email = Some("not-verified@example.test".into());
        coach.personal_email = Some("private@example.test".into());
        coach.phone = Some("555-0101".into());
        store.append(Table::Coaches, &coach)?;
    }
    Ok(())
}

fn team(school: &SchoolId, sport: Sport) -> CanonicalTeam {
    CanonicalTeam {
        id: CanonicalTeam::mint(school, sport, Gender::Girls, SchoolYear::DEFAULT),
        school: school.clone(),
        sport,
        gender: Gender::Girls,
        school_year: SchoolYear::DEFAULT,
        level: None,
        source_identities: Vec::new(),
        evidence: Vec::new(),
        retained_conflicts: Vec::new(),
    }
}

fn performance(
    store: &Store,
    athlete: &CanonicalAthlete,
    team_id: &census_domain::model::TeamId,
    evidence: Evidence,
    index: usize,
    mark: &str,
    sport: Sport,
) -> TestResult {
    let mut meet = CanonicalMeet::new_checked(
        Some(UsJurisdiction::Michigan),
        format!("Fixture Meet {index}"),
        "2026-05-01",
        CompetitionLevel::Invitational,
    )?;
    meet.sports.push(sport);
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
            source_key: &format!("result-{index}"),
        },
        PerformanceResult {
            team: team_id,
            mark: Mark::TimeSeconds(ExactSeconds::parse(mark)?),
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
