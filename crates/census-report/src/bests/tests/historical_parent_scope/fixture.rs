use super::*;
use census_domain::model::{
    CanonicalCoach, CanonicalTeam, CoachRole, CompetitionLevel, Evidence, PublishedGraduation,
    SchoolYear, SourceRef,
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
        match (self, scope) {
            (Self::MissingMeet | Self::OutsideCurrent, _) => false,
            (Self::NonCoreMeet | Self::NonCoreEvent, Scope::Core) => false,
            _ => true,
        }
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
        let current = school(case.current_state(), case.current_name());
        let historical = school(case.historical_state(), case.historical_name());
        let athlete = athlete(&current, case)?;
        let team = team(&historical)?;
        let (meet, event) = competition(case)?;
        let performance = performance(&athlete, &team, &meet, &event, case)?;
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
    Evidence::parsed(
        SourceRef::new(source, Some(RESULT_URL.into())),
        "2026-10-13",
    )
}

fn school(state: UsJurisdiction, name: &str) -> CanonicalSchool {
    let (mut row, _) = CanonicalSchool::new(
        state,
        name,
        census_domain::model::normalize_name(name),
        None,
    );
    row.evidence = vec![evidence("wiaa_results")];
    row
}

fn athlete(school: &CanonicalSchool, case: Case) -> TestResult<CanonicalAthlete> {
    let mut row = CanonicalAthlete::new_checked(
        &school.id,
        case.athlete_name(),
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, case.owner()),
    )?;
    row.published_graduations = vec![PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: SourceRef::new(
            "wiaa_results",
            Some("https://example.test/roster/2027".into()),
        ),
    }];
    row.public_profile_urls = vec![PROFILE_URL.into()];
    row.evidence = vec![evidence("wiaa_results")];
    row.sports = vec![Sport::OutdoorTrack];
    Ok(row)
}

fn team(school: &CanonicalSchool) -> TestResult<CanonicalTeam> {
    let year = SchoolYear::new(2024).ok_or("invalid synthetic historical team year")?;
    Ok(CanonicalTeam {
        id: CanonicalTeam::mint(&school.id, Sport::OutdoorTrack, Gender::Boys, year),
        school: school.id.clone(),
        sport: Sport::OutdoorTrack,
        gender: Gender::Boys,
        school_year: year,
        level: None,
        source_identities: Vec::new(),
        evidence: vec![evidence("wiaa_results")],
        retained_conflicts: Vec::new(),
    })
}

fn competition(case: Case) -> TestResult<(CanonicalMeet, CanonicalEvent)> {
    let mut meet = CanonicalMeet::new_checked(
        Some(case.venue_state()),
        case.meet_name(),
        RESULT_DATE,
        CompetitionLevel::Invitational,
    )?;
    meet.id = Id::mint("meet", &["historical-parent-scope", case.owner()]);
    meet.sports = vec![Sport::OutdoorTrack];
    meet.evidence = vec![evidence(if case == Case::NonCoreMeet {
        "athleticlive_results"
    } else {
        "wiaa_results"
    })];
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
    event.evidence = vec![evidence(if case == Case::NonCoreEvent {
        "athleticlive_results"
    } else {
        "wiaa_results"
    })];
    Ok((meet, event))
}

fn performance(
    athlete: &CanonicalAthlete,
    team: &CanonicalTeam,
    meet: &CanonicalMeet,
    event: &CanonicalEvent,
    case: Case,
) -> TestResult<CanonicalPerformance> {
    Ok(CanonicalPerformance {
        id: Id::mint("perf", &["historical-parent-scope", case.owner()]),
        athlete: athlete.id.clone(),
        team: team.id.clone(),
        event: event.id.clone(),
        meet: meet.id.clone(),
        date: RESULT_DATE.into(),
        mark: Mark::TimeSeconds(ExactSeconds::parse("48.55")?),
        wind_mps: None,
        place: Some(1),
        heat: Some("2".into()),
        round: Some("Final".into()),
        timing: Some(TimingMethod::Fat),
        observed_grade: None,
        evidence: vec![evidence("wiaa_results")],
        source_key: format!("wiaa_results:{}:400m:final", case.owner()),
        source_athlete: athlete.source.clone(),
        retained_conflicts: Vec::new(),
    })
}
