use super::*;
use census_domain::model::{
    CanonicalTeam, CompetitionLevel, Evidence, PublishedGraduation, SchoolYear, SourceRef,
};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AffiliationCase {
    MissingTeam,
    MissingSchool,
    UnknownHistoricalState,
    CrossStateTransfer,
}

impl AffiliationCase {
    pub(super) const ALL: [Self; 4] = [
        Self::MissingTeam,
        Self::MissingSchool,
        Self::UnknownHistoricalState,
        Self::CrossStateTransfer,
    ];

    pub(super) const fn school(self) -> Option<&'static str> {
        match self {
            Self::MissingTeam | Self::MissingSchool => None,
            Self::UnknownHistoricalState | Self::CrossStateTransfer => Some(HISTORICAL_SCHOOL),
        }
    }

    pub(super) const fn school_text(self) -> &'static str {
        match self {
            Self::MissingTeam | Self::MissingSchool => "",
            Self::UnknownHistoricalState | Self::CrossStateTransfer => HISTORICAL_SCHOOL,
        }
    }

    pub(super) const fn state(self) -> JurisdictionBucket {
        match self {
            Self::MissingTeam | Self::MissingSchool | Self::UnknownHistoricalState => {
                JurisdictionBucket::Unplaced
            }
            Self::CrossStateTransfer => JurisdictionBucket::Jurisdiction(UsJurisdiction::Wisconsin),
        }
    }
}

pub(super) struct Fixture {
    pub(super) case: AffiliationCase,
    pub(super) current: CanonicalSchool,
    historical: CanonicalSchool,
    pub(super) athlete: CanonicalAthlete,
    team: CanonicalTeam,
    pub(super) meet: CanonicalMeet,
    event: CanonicalEvent,
    pub(super) performance: CanonicalPerformance,
}

impl Fixture {
    pub(super) fn new(case: AffiliationCase) -> TestResult<Self> {
        let (mut current, current_id) = school(UsJurisdiction::Illinois, CURRENT_SCHOOL);
        let (mut historical, historical_id) = school(UsJurisdiction::Wisconsin, HISTORICAL_SCHOOL);
        current.evidence = vec![evidence()];
        historical.evidence = vec![evidence()];
        if case == AffiliationCase::UnknownHistoricalState {
            historical.state = None;
        }
        let athlete = athlete(&current_id)?;
        let school_year = SchoolYear::new(2024).ok_or("invalid historical team season")?;
        let team = CanonicalTeam {
            id: CanonicalTeam::mint(
                &historical_id,
                Sport::OutdoorTrack,
                Gender::Boys,
                school_year,
            ),
            school: historical_id,
            sport: Sport::OutdoorTrack,
            gender: Gender::Boys,
            school_year,
            level: None,
            source_identities: Vec::new(),
            evidence: vec![evidence()],
            retained_conflicts: Vec::new(),
        };
        let (meet, event) = competition()?;
        let performance = performance(&athlete, &team, &meet, &event)?;
        Ok(Self {
            case,
            current,
            historical,
            athlete,
            team,
            meet,
            event,
            performance,
        })
    }

    pub(super) fn store(&self, path: &Path) -> TestResult<Store> {
        let store = Store::open(path)?;
        self.append_to(&store)?;
        Ok(store)
    }

    pub(super) fn append_to(&self, store: &Store) -> TestResult {
        store.append(Table::Schools, &self.current)?;
        if self.case != AffiliationCase::MissingSchool {
            store.append(Table::Schools, &self.historical)?;
        }
        if self.case != AffiliationCase::MissingTeam {
            store.append(Table::Teams, &self.team)?;
        }
        store.append(Table::Athletes, &self.athlete)?;
        store.append(Table::Meets, &self.meet)?;
        store.append(Table::Events, &self.event)?;
        store.append(Table::Performances, &self.performance)?;
        Ok(())
    }

    pub(super) fn mixed_row(
        case: AffiliationCase,
        athlete_name: &str,
        historical_name: &str,
        source_owner: &str,
    ) -> TestResult<Self> {
        let mut fixture = Self::new(case)?;
        let (mut historical, historical_id) = school(UsJurisdiction::Wisconsin, historical_name);
        historical.evidence = vec![evidence()];
        if case == AffiliationCase::UnknownHistoricalState {
            historical.state = None;
        }
        fixture.historical = historical;
        fixture.athlete = named_athlete(&fixture.current.id, athlete_name, source_owner)?;
        fixture.team.school = historical_id.clone();
        fixture.team.id = CanonicalTeam::mint(
            &historical_id,
            Sport::OutdoorTrack,
            Gender::Boys,
            fixture.team.school_year,
        );
        fixture.performance.id = Id::mint("perf", &["historical-affiliation-mixed", source_owner]);
        fixture.performance.athlete = fixture.athlete.id.clone();
        fixture.performance.team = fixture.team.id.clone();
        fixture.performance.source_athlete = fixture.athlete.source.clone();
        fixture.performance.source_key = format!("wiaa_results:{source_owner}:400m:final");
        Ok(fixture)
    }

    pub(super) fn assert_dataset(&self, dataset: &ExportDataset) -> TestResult {
        check!(eq; dataset.athletes.as_slice(), std::slice::from_ref(&self.athlete));
        check!(eq; dataset.performances.as_slice(), std::slice::from_ref(&self.performance));
        check!(eq; dataset.canonical_aliases.len(), 0);
        let team = dataset.teams.get(&self.team.id);
        let school = dataset.schools.get(&self.historical.id);
        match self.case {
            AffiliationCase::MissingTeam => {
                check!(eq; team, None);
                check!(eq; school, Some(&self.historical));
            }
            AffiliationCase::MissingSchool => {
                check!(eq; team, Some(&self.team));
                check!(eq; school, None);
            }
            AffiliationCase::UnknownHistoricalState => {
                check!(eq; team, Some(&self.team));
                check!(eq; school, Some(&self.historical));
                check!(eq; school.and_then(|row| row.state), None);
            }
            AffiliationCase::CrossStateTransfer => {
                check!(eq; team, Some(&self.team));
                check!(eq; school, Some(&self.historical));
                check!(eq; school.and_then(|row| row.state), Some(UsJurisdiction::Wisconsin));
            }
        }
        check!(eq; dataset.schools.get(&self.current.id), Some(&self.current));
        Ok(())
    }
}

fn school(state: UsJurisdiction, name: &str) -> (CanonicalSchool, census_domain::model::SchoolId) {
    CanonicalSchool::new(
        state,
        name,
        census_domain::model::normalize_name(name),
        None,
    )
}

fn evidence() -> Evidence {
    Evidence::parsed(
        SourceRef::new("wiaa_results", Some(RESULT_URL.into())),
        "2026-10-13",
    )
}

fn athlete(school: &census_domain::model::SchoolId) -> TestResult<CanonicalAthlete> {
    named_athlete(school, ATHLETE_NAME, SOURCE_OWNER)
}

fn named_athlete(
    school: &census_domain::model::SchoolId,
    name: &str,
    source_owner: &str,
) -> TestResult<CanonicalAthlete> {
    let mut row = CanonicalAthlete::new_checked(
        school,
        name,
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, source_owner),
    )?;
    row.published_graduations = vec![PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: SourceRef::new(
            "wiaa_results",
            Some("https://example.test/roster/2027".into()),
        ),
    }];
    row.public_profile_urls = vec![PROFILE_URL.into()];
    row.evidence = vec![evidence()];
    row.sports = vec![Sport::OutdoorTrack];
    Ok(row)
}

fn competition() -> TestResult<(CanonicalMeet, CanonicalEvent)> {
    let mut meet = CanonicalMeet::new_checked(
        Some(UsJurisdiction::Michigan),
        "Historical affiliation invitational",
        RESULT_DATE,
        CompetitionLevel::Invitational,
    )?;
    meet.sports = vec![Sport::OutdoorTrack];
    meet.evidence = vec![evidence()];
    let mut event = CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::Track400m,
            gender: Gender::Boys,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?;
    event.evidence = vec![evidence()];
    Ok((meet, event))
}

fn performance(
    athlete: &CanonicalAthlete,
    team: &CanonicalTeam,
    meet: &CanonicalMeet,
    event: &CanonicalEvent,
) -> TestResult<CanonicalPerformance> {
    Ok(CanonicalPerformance {
        id: Id::mint("perf", &["historical-affiliation"]),
        athlete: athlete.id.clone(),
        team: team.id.clone(),
        event: event.id.clone(),
        meet: meet.id.clone(),
        date: RESULT_DATE.into(),
        mark: Mark::TimeSeconds(ExactSeconds::parse("48.55")?),
        wind_mps: None,
        adjustment: None,
        place: Some(1),
        heat: Some("2".into()),
        round: Some("Final".into()),
        timing: Some(TimingMethod::Fat),
        observed_grade: None,
        evidence: vec![evidence()],
        source_key: SOURCE_KEY.into(),
        source_athlete: athlete.source.clone(),
        retained_conflicts: Vec::new(),
    })
}
