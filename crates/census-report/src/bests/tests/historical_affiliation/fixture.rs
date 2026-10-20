use super::super::fixture::{
    athlete_row, event_row, meet_row, performance_row, school_row, team_row, AthleteParams,
    EventParams, MeetParams, PerformanceParams, TeamParams,
};
use super::*;
use census_domain::model::{CanonicalTeam, CompetitionLevel, Evidence, SchoolYear};
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
        let (current, current_id) = school_row(
            UsJurisdiction::Illinois,
            CURRENT_SCHOOL,
            census_domain::model::normalize_name(CURRENT_SCHOOL),
            evidence(),
        );
        let (mut historical, historical_id) = school_row(
            UsJurisdiction::Wisconsin,
            HISTORICAL_SCHOOL,
            census_domain::model::normalize_name(HISTORICAL_SCHOOL),
            evidence(),
        );
        if case == AffiliationCase::UnknownHistoricalState {
            historical.state = None;
        }
        let athlete = athlete_row(AthleteParams {
            school: &current_id,
            name: ATHLETE_NAME,
            grad_year: GradYear::CO2027,
            gender: Gender::Boys,
            namespace: SourceNamespace::MilesplitAthlete,
            owner: SOURCE_OWNER,
            roster_source: "wiaa_results",
            roster_url: Some("https://example.test/roster/2027".to_owned()),
            profile_url: Some(PROFILE_URL),
            sport: Sport::OutdoorTrack,
            evidence: evidence(),
        })?;
        let school_year = SchoolYear::new(2024).ok_or("invalid historical team season")?;
        let team = team_row(TeamParams {
            school: &historical_id,
            sport: Sport::OutdoorTrack,
            gender: Gender::Boys,
            year: school_year,
            evidence: evidence(),
        });
        let (meet, event) = competition()?;
        let performance = performance_row(
            &athlete,
            &team,
            &meet,
            &event,
            PerformanceParams {
                id_seed: &["historical-affiliation"],
                date: RESULT_DATE,
                mark: Mark::TimeSeconds(ExactSeconds::parse("48.55")?),
                wind_mps: None,
                place: Some(1),
                heat: Some("2"),
                round: Some("Final"),
                timing: Some(TimingMethod::Fat),
                source_key: SOURCE_KEY.into(),
                evidence: evidence(),
            },
        );
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
        let (historical, historical_id) = school_row(
            UsJurisdiction::Wisconsin,
            historical_name,
            census_domain::model::normalize_name(historical_name),
            evidence(),
        );
        fixture.historical = historical;
        if case == AffiliationCase::UnknownHistoricalState {
            fixture.historical.state = None;
        }
        fixture.athlete = athlete_row(AthleteParams {
            school: &fixture.current.id,
            name: athlete_name,
            grad_year: GradYear::CO2027,
            gender: Gender::Boys,
            namespace: SourceNamespace::MilesplitAthlete,
            owner: source_owner,
            roster_source: "wiaa_results",
            roster_url: Some("https://example.test/roster/2027".to_owned()),
            profile_url: Some(PROFILE_URL),
            sport: Sport::OutdoorTrack,
            evidence: evidence(),
        })?;
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

fn evidence() -> Evidence {
    super::super::fixture::evidence("wiaa_results", Some(RESULT_URL.into()), "2026-10-13")
}

fn competition() -> TestResult<(CanonicalMeet, CanonicalEvent)> {
    let meet = meet_row(MeetParams {
        state: Some(UsJurisdiction::Michigan),
        name: "Historical affiliation invitational",
        date: RESULT_DATE,
        level: CompetitionLevel::Invitational,
        sport: Sport::OutdoorTrack,
        evidence: evidence(),
        id_seed: None,
    })?;
    let event = event_row(
        &meet,
        EventParams {
            kind: EventKind::Track400m,
            gender: Gender::Boys,
            specification: EventSpecification::default(),
            owned: true,
            labels: Vec::new(),
            evidence: vec![evidence()],
        },
    )?;
    Ok((meet, event))
}
