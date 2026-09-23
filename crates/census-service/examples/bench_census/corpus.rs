//! The synthetic corpus and the loop that builds it: `--schools` schools, each with one team, one
//! meet, `ATHLETES_PER_SCHOOL` athletes, one event per athlete and `PERFORMANCES_PER_ATHLETE` marks
//! in that event.
//!
//! Every row is built from the seeded generator in `lcg` and the row shapes in `fixtures`, so two
//! runs of the same `--schools` build the same corpus; the counts recorded here are what the
//! measured phases assert the store and the merged snapshot against.

use anyhow::Result;
use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, EventKind, Gender, GradYear, Mark, MeetId, SchoolId,
    SourceIdentity, SourceNamespace, Sport, TeamId, TimingMethod,
};
use census_domain::UsJurisdiction;
use std::collections::HashSet;

use super::fixtures::{
    base_seconds, event_kind, evidence, grade, meet_of, observed_grade, place_of, team_of,
    MEET_DATE,
};
use super::lcg::{Lcg, SEED};

/// Athletes in every synthetic school; one team and one meet per school scale with it.
pub(super) const ATHLETES_PER_SCHOOL: usize = 8;
/// Marks in every athlete's one event.
pub(super) const PERFORMANCES_PER_ATHLETE: usize = 2;

/// The whole synthetic corpus, kept in memory so every phase can assert against the input.
pub(super) struct Corpus {
    pub(super) schools: Vec<CanonicalSchool>,
    pub(super) teams: Vec<CanonicalTeam>,
    pub(super) athletes: Vec<CanonicalAthlete>,
    pub(super) meets: Vec<CanonicalMeet>,
    pub(super) events: Vec<CanonicalEvent>,
    pub(super) performances: Vec<CanonicalPerformance>,
    /// Event ids seen during generation; the merge must reduce `events` to exactly this count.
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

    /// Rows handed to `append_many`, i.e. observations, not merged entities.
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

    /// Distinct rows after the merge: what `consolidate` writes and the workbook reads.
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

/// One school, its team, its meet, and the athletes that compete at that meet.
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
    let team = team_of(&school_id, index);
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

/// One athlete with one event at `meet_id` and `PERFORMANCES_PER_ATHLETE` marks in it.
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

/// Boys and girls alternate across the corpus: one slot per athlete, offset by the school index.
fn gender_of(index: usize, slot: usize) -> Gender {
    if index.saturating_add(slot).is_multiple_of(2) {
        Gender::Boys
    } else {
        Gender::Girls
    }
}

/// The athlete row: one outdoor-track entry, one observed grade and one MileSplit profile.
fn athlete_of(
    school_id: &SchoolId,
    index: usize,
    slot: usize,
    gender: Gender,
) -> Result<CanonicalAthlete> {
    let mut athlete = CanonicalAthlete::new(
        school_id,
        format!("Runner {index}-{slot}"),
        GradYear::CO2027,
        gender,
    );
    athlete.sports.push(Sport::OutdoorTrack);
    athlete.observed_grades.push(observed_grade()?);
    athlete.evidence.push(evidence());
    athlete.source_identities.push(SourceIdentity::new(
        SourceNamespace::MilesplitAthlete,
        format!("profile-{index}-{slot}"),
    ));
    athlete
        .public_profile_urls
        .push(format!("https://example.invalid/athletes/{index}-{slot}"));
    Ok(athlete)
}

/// The athlete's event at this meet: one kind per athlete, so a meet holds a handful of events.
fn event_of(meet_id: &MeetId, kind: &EventKind, gender: Gender) -> CanonicalEvent {
    let mut event = CanonicalEvent::new(meet_id, kind.clone(), gender, None, None);
    event.evidence.push(evidence());
    event
}

/// One mark in that event, keyed by the `source_key` the merge deduplicates on.
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
    let mark = Mark::TimeSeconds(base_seconds(kind, rng));
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
        source_athlete: None,
        retained_conflicts: Vec::new(),
    })
}
