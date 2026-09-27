use anyhow::Result;
use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, CentiSeconds, EventKind, Gender, GradYear, Mark, MeetId,
    SchoolId, SourceIdentity, SourceNamespace, Sport, TeamId, TimingMethod,
};
use census_domain::UsJurisdiction;
use std::collections::HashSet;

use super::fixtures::{
    base_seconds, event_kind, evidence, grade, meet_of, observed_grade, place_of, team_of,
    MEET_DATE,
};
use super::lcg::{Lcg, SEED};

pub(super) const ATHLETES_PER_SCHOOL: usize = 8;
pub(super) const PERFORMANCES_PER_ATHLETE: usize = 2;

pub(super) struct Corpus {
    pub(super) schools: Vec<CanonicalSchool>,
    pub(super) teams: Vec<CanonicalTeam>,
    pub(super) athletes: Vec<CanonicalAthlete>,
    pub(super) meets: Vec<CanonicalMeet>,
    pub(super) events: Vec<CanonicalEvent>,
    pub(super) performances: Vec<CanonicalPerformance>,
    pub(super) distinct_events: HashSet<String>,
}

impl Corpus {
    pub(super) fn new() -> Self {
        Self {
            schools: Vec::new(),
            teams: Vec::new(),
            athletes: Vec::new(),
            meets: Vec::new(),
            events: Vec::new(),
            performances: Vec::new(),
            distinct_events: HashSet::new(),
        }
    }

    pub(super) fn appended_rows(&self) -> usize {
        [
            self.schools.len(),
            self.teams.len(),
            self.athletes.len(),
            self.meets.len(),
            self.events.len(),
            self.performances.len(),
        ]
        .into_iter()
        .sum()
    }

    pub(super) fn merged_rows(&self) -> usize {
        [
            self.schools.len(),
            self.teams.len(),
            self.athletes.len(),
            self.meets.len(),
            self.distinct_events.len(),
            self.performances.len(),
        ]
        .into_iter()
        .sum()
    }
}

pub(super) fn build_corpus(school_count: usize) -> Result<Corpus> {
    let mut corpus = Corpus::new();
    let mut rng = Lcg::new(SEED);
    for index in 0..school_count {
        append_school(&mut corpus, &mut rng, index)?;
    }
    Ok(corpus)
}

pub(super) fn append_school(corpus: &mut Corpus, rng: &mut Lcg, index: usize) -> Result<()> {
    let state = match index % 3 {
        0 => UsJurisdiction::Wisconsin,
        1 => UsJurisdiction::Minnesota,
        _ => UsJurisdiction::Iowa,
    };
    let name = format!("Synthetic School {index}");
    let (mut school, school_id) = CanonicalSchool::new(state, name.clone(), normalize_name(&name));
    school.evidence.push(evidence());
    school.source_identities.push(SourceIdentity::new(
        SourceNamespace::AssociationSchool {
            association: "synthetic".to_string(),
        },
        format!("school-{index}"),
    ));
    let team = team_of(&school_id, index)?;
    let team_id = team.id.clone();
    let meet = meet_of(state, index);
    let meet_id = meet.id.clone();
    corpus.schools.push(school);
    corpus.teams.push(team);
    corpus.meets.push(meet);
    for slot in 0..ATHLETES_PER_SCHOOL {
        append_athlete(corpus, rng, index, slot, &school_id, &team_id, &meet_id)?;
    }
    Ok(())
}

fn append_athlete(
    corpus: &mut Corpus,
    rng: &mut Lcg,
    index: usize,
    slot: usize,
    school_id: &SchoolId,
    team_id: &TeamId,
    meet_id: &MeetId,
) -> Result<()> {
    let gender = gender_of(index, slot);
    let athlete = athlete_of(school_id, index, slot, gender)?;
    let kind = event_kind(rng.next());
    let event = event_of(meet_id, &kind, gender);
    corpus.distinct_events.insert(event.id.as_str().to_string());
    for attempt in 0..PERFORMANCES_PER_ATHLETE {
        let source_key = format!("perf-{index}-{slot}-{attempt}");
        let performance =
            performance_of(&athlete, team_id, meet_id, &event, &kind, source_key, rng)?;
        corpus.performances.push(performance);
    }
    corpus.events.push(event);
    corpus.athletes.push(athlete);
    Ok(())
}

fn gender_of(index: usize, slot: usize) -> Gender {
    if index.saturating_add(slot).is_multiple_of(2) {
        Gender::Boys
    } else {
        Gender::Girls
    }
}

fn athlete_of(
    school_id: &SchoolId,
    index: usize,
    slot: usize,
    gender: Gender,
) -> Result<CanonicalAthlete> {
    let source = SourceIdentity::new(
        SourceNamespace::MilesplitAthlete,
        format!("profile-{index}-{slot}"),
    );
    let mut athlete = CanonicalAthlete::new(
        school_id,
        format!("Runner {index}-{slot}"),
        GradYear::CO2027,
        gender,
        source,
    );
    athlete.sports.push(Sport::OutdoorTrack);
    athlete.observed_grades.push(observed_grade()?);
    athlete.evidence.push(evidence());
    athlete
        .public_profile_urls
        .push(format!("https://example.invalid/athletes/{index}-{slot}"));
    Ok(athlete)
}

fn event_of(meet_id: &MeetId, kind: &EventKind, gender: Gender) -> CanonicalEvent {
    let mut event = CanonicalEvent::new(meet_id, kind.clone(), gender, None, None);
    event.evidence.push(evidence());
    event
}

fn performance_of(
    athlete: &CanonicalAthlete,
    team_id: &TeamId,
    meet_id: &MeetId,
    event: &CanonicalEvent,
    kind: &EventKind,
    source_key: String,
    rng: &mut Lcg,
) -> Result<CanonicalPerformance> {
    let id = CanonicalPerformance::mint(&athlete.id, meet_id, kind, MEET_DATE, &source_key);
    let mark = Mark::TimeSeconds(
        CentiSeconds::try_from_seconds_f64(base_seconds(kind, rng))
            .ok_or_else(|| anyhow::anyhow!("fixture mark is out of range"))?,
    );
    let place = place_of(rng.next())?;
    Ok(CanonicalPerformance {
        id,
        athlete: athlete.id.clone(),
        team: team_id.clone(),
        event: event.id.clone(),
        meet: meet_id.clone(),
        date: MEET_DATE.to_string(),
        mark,
        wind_mps: None,
        place: Some(place),
        heat: None,
        round: None,
        timing: Some(TimingMethod::Fat),
        observed_grade: Some(grade(11)?),
        evidence: vec![evidence()],
        source_key,
        source_athlete: athlete.source.clone(),
        retained_conflicts: Vec::new(),
    })
}
