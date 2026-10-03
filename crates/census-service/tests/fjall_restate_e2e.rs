#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, CentiSeconds, CompetitionLevel, EventKind, Evidence, Gender,
    GradYear, Grade, Mark, ObservedGrade, SchoolId, SchoolYear, SourceIdentity, SourceNamespace,
    SourceRef, Sport, TimingMethod,
};
use census_domain::UsJurisdiction;
use census_store::{Store, StoreStats, Table};
use std::collections::HashSet;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const SOURCE_ID: &str = "mshsl_results";
const MEET_DATE: &str = "2026-05-02";

fn evidence() -> Evidence {
    Evidence::parsed(SourceRef::id(SOURCE_ID), MEET_DATE)
}

struct Corpus {
    schools: Vec<CanonicalSchool>,
    teams: Vec<CanonicalTeam>,
    athletes: Vec<CanonicalAthlete>,
    meets: Vec<CanonicalMeet>,
    events: Vec<CanonicalEvent>,
    performances: Vec<CanonicalPerformance>,
    distinct_events: HashSet<String>,
}

impl Corpus {
    fn append(&self, store: &Store) -> TestResult {
        store.append_many(Table::Schools, &self.schools)?;
        store.append_many(Table::Teams, &self.teams)?;
        store.append_many(Table::Athletes, &self.athletes)?;
        store.append_many(Table::Meets, &self.meets)?;
        store.append_many(Table::Events, &self.events)?;
        store.append_many(Table::Performances, &self.performances)?;
        Ok(())
    }
}

fn synthetic_corpus(school_count: usize, athletes_per_school: usize) -> TestResult<Corpus> {
    let mut corpus = Corpus {
        schools: Vec::new(),
        teams: Vec::new(),
        athletes: Vec::new(),
        meets: Vec::new(),
        events: Vec::new(),
        performances: Vec::new(),
        distinct_events: HashSet::new(),
    };
    for index in 0..school_count {
        add_school(&mut corpus, index, athletes_per_school)?;
    }
    Ok(corpus)
}

fn add_school(corpus: &mut Corpus, index: usize, athletes_per_school: usize) -> TestResult {
    let season = SchoolYear::new(2025).ok_or("invalid fixture season")?;
    let name = format!("E2E School {index}");
    let (mut school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        name.clone(),
        normalize_name(&name),
    );
    school.evidence.push(evidence());
    let team = CanonicalTeam {
        id: CanonicalTeam::mint(&school_id, Sport::OutdoorTrack, Gender::Mixed, season),
        school: school_id.clone(),
        sport: Sport::OutdoorTrack,
        gender: Gender::Mixed,
        school_year: season,
        level: None,
        source_identities: Vec::new(),
        evidence: vec![evidence()],
        retained_conflicts: Vec::new(),
    };
    let team_id = team.id.clone();
    let mut meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        format!("E2E Invite {index}"),
        MEET_DATE,
        CompetitionLevel::Invitational,
    );
    meet.sports.push(Sport::OutdoorTrack);
    meet.evidence.push(evidence());
    let meet_id = meet.id.clone();
    corpus.schools.push(school);
    corpus.teams.push(team);
    corpus.meets.push(meet);
    for slot in 0..athletes_per_school {
        add_athlete(corpus, index, slot, &school_id, &team_id, &meet_id)?;
    }
    Ok(())
}

fn add_athlete(
    corpus: &mut Corpus,
    index: usize,
    slot: usize,
    school_id: &SchoolId,
    team_id: &census_domain::model::TeamId,
    meet_id: &census_domain::model::MeetId,
) -> TestResult {
    let gender = if (index + slot).is_multiple_of(2) {
        Gender::Boys
    } else {
        Gender::Girls
    };
    let source = SourceIdentity::new(
        SourceNamespace::Other("fixture".to_string()),
        format!("e2e-athlete-{index}-{slot}"),
    );
    let mut athlete = CanonicalAthlete::new(
        school_id,
        format!("E2E Runner {index}-{slot}"),
        GradYear::CO2027,
        gender,
        source.clone(),
    );
    athlete.sports.push(Sport::OutdoorTrack);
    athlete.observed_grades.push(ObservedGrade {
        grade: Grade::new(11).ok_or("invalid fixture grade")?,
        school_year: SchoolYear::new(2025).ok_or("invalid fixture season")?,
        source: SourceRef::id(SOURCE_ID),
    });
    athlete.evidence.push(evidence());

    let kind = match slot % 3 {
        0 => EventKind::Track800m,
        1 => EventKind::Track1600m,
        _ => EventKind::Track3200m,
    };
    let mut event = CanonicalEvent::new(meet_id, kind.clone(), gender, None, None);
    event.evidence.push(evidence());
    corpus.distinct_events.insert(event.id.as_str().to_string());
    for attempt in 0..2 {
        let source_key = format!("e2e-{index}-{slot}-{attempt}");
        let id = CanonicalPerformance::mint(&athlete.id, meet_id, &kind, MEET_DATE, &source_key);
        let seconds = 130.0 + f64::from(u32::try_from(index + slot)?) / 10.0;
        corpus.performances.push(CanonicalPerformance {
            id,
            athlete: athlete.id.clone(),
            team: team_id.clone(),
            event: event.id.clone(),
            meet: meet_id.clone(),
            date: MEET_DATE.to_string(),
            mark: Mark::TimeSeconds(
                CentiSeconds::try_from_seconds_f64(seconds).ok_or("invalid fixture time")?,
            ),
            wind_mps: None,
            place: Some(u16::try_from(attempt + 1)?),
            heat: None,
            round: None,
            timing: Some(TimingMethod::Fat),
            observed_grade: Some(Grade::new(11).ok_or("invalid fixture grade")?),
            evidence: vec![evidence()],
            source_key,
            source_athlete: Some(source.clone()),
            retained_conflicts: Vec::new(),
        });
    }
    corpus.events.push(event);
    corpus.athletes.push(athlete);
    Ok(())
}

fn count_of(counts: &[(String, usize)], table: &str) -> TestResult<usize> {
    counts
        .iter()
        .find(|(name, _)| name == table)
        .map(|(_, count)| *count)
        .ok_or_else(|| format!("consolidate reported no count for {table}").into())
}

fn assert_observation_counts(stats: &StoreStats) -> TestResult {
    let count = |table: &str| {
        stats
            .tables
            .iter()
            .find(|(name, _)| name == table)
            .map(|(_, count)| *count)
            .map_or(0, |value| value)
    };
    check!(eq; count("schools"), 2, "two school observations");
    check!(eq; count("athletes"), 1);
    check!(eq; count("performances"),
    0,
    "a table with no writes counts zero");
    check!(eq; stats.tables.len(),
    Table::ALL.len(),
    "every table is reported");
    check!(eq; stats.observations, 3);
    Ok(())
}

#[path = "fjall_restate_e2e/report_chain.rs"]
mod report_chain;
#[path = "fjall_restate_e2e/restate_endpoint.rs"]
mod restate_endpoint;
#[path = "fjall_restate_e2e/store_roundtrip.rs"]
mod store_roundtrip;
