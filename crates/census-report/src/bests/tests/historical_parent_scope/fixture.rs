use super::super::fixture::{
    athlete_row, event_row, meet_row, performance_row, school_row, team_row, AthleteParams,
    EventParams, MeetParams, PerformanceParams, TeamParams,
};
use super::*;
use census_domain::model::{
    CanonicalCoach, CanonicalTeam, CoachRole, CompetitionLevel, Evidence, SchoolYear,
    SourceEventLabel, SourceRef,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Case {
    AlaskaSchoolHawaiiVenue,
    HawaiiSchoolAlaskaVenue,
    MissingTeam,
    MissingSchool,
    MissingMeet,
    NonCoreMeet,
    NonCoreEvent,
    OutsideCurrent,
}

impl Case {
    pub(super) const ALL: [Self; 8] = [
        Self::AlaskaSchoolHawaiiVenue,
        Self::HawaiiSchoolAlaskaVenue,
        Self::MissingTeam,
        Self::MissingSchool,
        Self::MissingMeet,
        Self::NonCoreMeet,
        Self::NonCoreEvent,
        Self::OutsideCurrent,
    ];

    pub(super) const fn owner(self) -> &'static str {
        match self {
            Self::AlaskaSchoolHawaiiVenue => "parent-scope-alaska-history",
            Self::HawaiiSchoolAlaskaVenue => "parent-scope-hawaii-history",
            Self::MissingTeam => "parent-scope-missing-team",
            Self::MissingSchool => "parent-scope-missing-school",
            Self::MissingMeet => "parent-scope-missing-meet",
            Self::NonCoreMeet => "parent-scope-noncore-meet",
            Self::NonCoreEvent => "parent-scope-noncore-event",
            Self::OutsideCurrent => "parent-scope-outside-current",
        }
    }

    pub(super) const fn athlete_name(self) -> &'static str {
        match self {
            Self::AlaskaSchoolHawaiiVenue => "Alaska history runner",
            Self::HawaiiSchoolAlaskaVenue => "Hawaii history runner",
            Self::MissingTeam => "Missing historical team runner",
            Self::MissingSchool => "Missing historical school runner",
            Self::MissingMeet => "Missing historical meet runner",
            Self::NonCoreMeet => "Non-Core historical meet runner",
            Self::NonCoreEvent => "Non-Core historical event runner",
            Self::OutsideCurrent => "Outside current school runner",
        }
    }

    pub(super) const fn historical_name(self) -> &'static str {
        match self {
            Self::AlaskaSchoolHawaiiVenue => "Historical Alaska High",
            Self::HawaiiSchoolAlaskaVenue => "Historical Hawaii High",
            Self::MissingTeam => "Missing team Alaska High",
            Self::MissingSchool => "Missing school Hawaii High",
            Self::MissingMeet => "Missing meet Alaska High",
            Self::NonCoreMeet => "Non-Core meet Alaska High",
            Self::NonCoreEvent => "Non-Core event Alaska High",
            Self::OutsideCurrent => "Outside current historical Hawaii High",
        }
    }

    pub(super) const fn historical_state(self) -> UsJurisdiction {
        match self {
            Self::HawaiiSchoolAlaskaVenue | Self::MissingSchool | Self::OutsideCurrent => {
                UsJurisdiction::Hawaii
            }
            _ => UsJurisdiction::Alaska,
        }
    }

    pub(super) const fn venue_state(self) -> UsJurisdiction {
        match self {
            Self::HawaiiSchoolAlaskaVenue | Self::MissingSchool => UsJurisdiction::Alaska,
            Self::OutsideCurrent => UsJurisdiction::Wisconsin,
            _ => UsJurisdiction::Hawaii,
        }
    }

    pub(super) const fn current_state(self) -> UsJurisdiction {
        match self {
            Self::OutsideCurrent => UsJurisdiction::Alaska,
            _ => UsJurisdiction::Illinois,
        }
    }

    pub(super) const fn current_name(self) -> &'static str {
        match self {
            Self::OutsideCurrent => "Current Alaska High",
            _ => "Current Illinois High",
        }
    }

    pub(super) const fn school(self) -> Option<&'static str> {
        match self {
            Self::MissingTeam | Self::MissingSchool => None,
            _ => Some(self.historical_name()),
        }
    }

    pub(super) const fn school_text(self) -> &'static str {
        match self.school() {
            Some(name) => name,
            None => "",
        }
    }

    pub(super) const fn school_bucket(self) -> JurisdictionBucket {
        match self.school() {
            Some(_) => JurisdictionBucket::Jurisdiction(self.historical_state()),
            None => JurisdictionBucket::Unplaced,
        }
    }

    pub(super) const fn meet_name(self) -> &'static str {
        match self {
            Self::HawaiiSchoolAlaskaVenue | Self::MissingSchool => "Alaska historical invitational",
            Self::OutsideCurrent => "Wisconsin population control invitational",
            _ => "Hawaii historical invitational",
        }
    }

    pub(super) fn meet_metadata(self, scope: Scope) -> (&'static str, Option<&'static str>) {
        match (self, scope) {
            (Self::MissingMeet, _) | (Self::NonCoreMeet, Scope::Core) => ("", None),
            _ => (self.meet_name(), Some(self.venue_state().code())),
        }
    }

    pub(super) const fn event_label(self, scope: Scope) -> &'static str {
        match (self, scope) {
            (Self::NonCoreEvent, Scope::Core) => "",
            _ => "Track400m",
        }
    }

    pub(super) const fn has_pr(self, scope: Scope) -> bool {
        !matches!(
            (self, scope),
            (Self::MissingMeet | Self::OutsideCurrent, _)
                | (Self::NonCoreMeet | Self::NonCoreEvent, Scope::Core)
        )
    }
}

pub(super) struct Fixture {
    pub(super) case: Case,
    pub(super) current: CanonicalSchool,
    pub(super) historical: CanonicalSchool,
    pub(super) athlete: CanonicalAthlete,
    pub(super) team: CanonicalTeam,
    pub(super) meet: CanonicalMeet,
    pub(super) event: CanonicalEvent,
    pub(super) performance: CanonicalPerformance,
    pub(super) coach: CanonicalCoach,
}

impl Fixture {
    pub(super) fn new(case: Case) -> TestResult<Self> {
        let (current, _) = school_row(
            case.current_state(),
            case.current_name(),
            census_domain::model::normalize_name(case.current_name()),
            evidence("wiaa_results"),
        );
        let (historical, _) = school_row(
            case.historical_state(),
            case.historical_name(),
            census_domain::model::normalize_name(case.historical_name()),
            evidence("wiaa_results"),
        );
        let athlete = athlete_row(AthleteParams {
            school: &current.id,
            name: case.athlete_name(),
            grad_year: GradYear::CO2027,
            gender: Gender::Boys,
            namespace: SourceNamespace::MilesplitAthlete,
            owner: case.owner(),
            roster_source: "wiaa_results",
            roster_url: Some("https://example.test/roster/2027".to_owned()),
            profile_url: Some(PROFILE_URL),
            sport: Sport::OutdoorTrack,
            evidence: evidence("wiaa_results"),
        })?;
        let year = SchoolYear::new(2024).ok_or("invalid synthetic historical team year")?;
        let team = team_row(TeamParams {
            school: &historical.id,
            sport: Sport::OutdoorTrack,
            gender: Gender::Boys,
            year,
            evidence: evidence("wiaa_results"),
        });
        let (meet, event) = competition(case)?;
        let performance = performance_row(
            &athlete,
            &team,
            &meet,
            &event,
            PerformanceParams {
                id_seed: &["historical-parent-scope", case.owner()],
                date: RESULT_DATE,
                mark: Mark::TimeSeconds(ExactSeconds::parse("48.55")?),
                wind_mps: None,
                place: Some(1),
                heat: Some("2"),
                round: Some("Final"),
                timing: Some(TimingMethod::Fat),
                source_key: format!("wiaa_results:{}:400m:final", case.owner()),
                evidence: evidence("wiaa_results"),
            },
        );
        let coach = CanonicalCoach::new(
            &historical.id,
            "Historical outside-scope coach",
            Some(Sport::OutdoorTrack),
            Gender::Boys,
            CoachRole::HeadCoach,
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
            coach,
        })
    }

    pub(super) fn append_to(&self, store: &Store) -> TestResult {
        store.append(Table::Schools, &self.current)?;
        if self.case != Case::MissingSchool {
            store.append(Table::Schools, &self.historical)?;
            store.append(Table::Coaches, &self.coach)?;
        }
        if self.case != Case::MissingTeam {
            store.append(Table::Teams, &self.team)?;
        }
        store.append(Table::Athletes, &self.athlete)?;
        if self.case != Case::MissingMeet {
            store.append(Table::Meets, &self.meet)?;
        }
        store.append(Table::Events, &self.event)?;
        store.append(Table::Performances, &self.performance)?;
        Ok(())
    }
}

fn evidence(source: &str) -> Evidence {
    super::super::fixture::evidence(source, Some(RESULT_URL.into()), "2026-10-13")
}

fn competition(case: Case) -> TestResult<(CanonicalMeet, CanonicalEvent)> {
    let meet = meet_row(MeetParams {
        state: Some(case.venue_state()),
        name: case.meet_name(),
        date: RESULT_DATE,
        level: CompetitionLevel::Invitational,
        sport: Sport::OutdoorTrack,
        evidence: evidence(if case == Case::NonCoreMeet {
            "athleticlive_results"
        } else {
            "wiaa_results"
        }),
        id_seed: Some(&["historical-parent-scope", case.owner()]),
    })?;
    let owner = if case == Case::NonCoreEvent {
        "athleticlive_results"
    } else {
        "wiaa_results"
    };
    let event = event_row(
        &meet,
        EventParams {
            kind: EventKind::Track400m,
            gender: Gender::Boys,
            specification: EventSpecification::default(),
            owned: false,
            labels: vec![SourceEventLabel {
                source: SourceRef::new(owner, Some(RESULT_URL.into())),
                label: "400m".into(),
            }],
            evidence: vec![evidence(owner)],
        },
    )?;
    Ok((meet, event))
}
