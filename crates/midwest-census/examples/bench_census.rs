//! Census throughput harness: a deterministic synthetic corpus through the whole pipeline.
//!
//! ```text
//! cargo run --release --example bench_census -- --schools 500
//! ```
//!
//! **Bound.** The corpus is exactly `--schools` schools, one team and one meet per school
//! (`--schools` each), `8` athletes per school, one event per athlete, and two performances per
//! athlete — `35 * --schools` appended rows in total. After the merge the distinct count is `27` per
//! school (school, team, meet, 8 athletes, 16 performances) plus its deduplicated events, at most 6
//! per meet; the run reports and asserts the exact number. Nothing else caps the corpus, so
//! `--schools` is the dataset bound.
//!
//! **Determinism.** Every value comes from a seeded 32-bit LCG, so two runs with the same
//! `--schools` build byte-identical corpora and their measurements are comparable.
//!
//! **Output.** `metric=<name> items=<n> unit=<unit>`, `metric=<name> seconds=<s>`,
//! `metric=<name> rate=<n> unit=<unit>/s`, and one `json={...}` summary line. Every phase asserts
//! the counts it produced before reporting its rate.

use anyhow::{Context, Result};
use clap::Parser;
use midwest_census::model::{
    normalize_name, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, CompetitionLevel, EventKind, Evidence, Gender, GradYear, Grade,
    Id, Mark, MeetId, ObservedGrade, SchoolId, SchoolYear, SourceIdentity, SourceNamespace,
    SourceRef, Sport, TeamId, TimingMethod,
};
use midwest_census::report::{self, Scope};
use midwest_census::store::{Store, Table};
use midwest_census::{bests, census, workbook};
use serde::Serialize;
use serde_json::json;
use std::collections::HashSet;
use std::time::{Duration, Instant};
use tempfile::TempDir;

const DEFAULT_SCHOOLS: usize = 500;
const ATHLETES_PER_SCHOOL: usize = 8;
const PERFORMANCES_PER_ATHLETE: usize = 2;
/// Rows per `append_many` call; keeps one batch bounded regardless of corpus size.
const APPEND_BATCH: usize = 1_000;
/// Evidence source id for every synthetic row; a core adapter id so `Scope::Core` keeps the rows.
const SOURCE_ID: &str = "mshsl_results";
const MEET_DATE: &str = "2026-05-02";
const SEED: u32 = 0x5eed_2027;

#[derive(Debug, Parser)]
#[command(
    name = "bench_census",
    about = "Synthetic-corpus throughput harness: append, consolidate, census, bests, workbook"
)]
struct Options {
    /// Schools in the synthetic corpus; athletes, meets and performances scale from this.
    #[arg(long, default_value_t = DEFAULT_SCHOOLS)]
    schools: usize,
}

/// One measured phase: how many items moved, how long it took, and the resulting rate.
#[derive(Debug, Clone, Copy, Serialize)]
struct Phase {
    items: usize,
    seconds: f64,
    rate_per_second: f64,
}

/// The whole synthetic corpus, kept in memory so every phase can assert against the input.
struct Corpus {
    schools: Vec<CanonicalSchool>,
    teams: Vec<CanonicalTeam>,
    athletes: Vec<CanonicalAthlete>,
    meets: Vec<CanonicalMeet>,
    events: Vec<CanonicalEvent>,
    performances: Vec<CanonicalPerformance>,
    /// Event ids seen during generation; the merge must reduce `events` to exactly this count.
    distinct_events: HashSet<String>,
}

impl Corpus {
    fn new() -> Self {
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
    fn appended_rows(&self) -> usize {
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
    fn merged_rows(&self) -> usize {
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

/// Deterministic 32-bit LCG (Numerical Recipes constants): no `rand` dependency, identical corpus
/// for a given seed on every machine.
struct Lcg(u32);

impl Lcg {
    fn new(seed: u32) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
}

fn main() -> Result<()> {
    let options = Options::parse();
    anyhow::ensure!(options.schools >= 1, "--schools must be at least 1");
    let started = Instant::now();

    let dir = tempfile::tempdir().context("creating a temporary root")?;
    let store = Store::open(dir.path()).context("opening the store")?;
    let corpus = build_corpus(options.schools)?;

    let appended = append_corpus(&store, &corpus)?;
    let consolidated = consolidate_phase(&store, &corpus)?;
    let (core_census, all_sources_census) = census_phases(&store, corpus.athletes.len())?;
    let (bests_measured, best_rows) =
        bests_phase(&store, corpus.athletes.len(), corpus.performances.len())?;
    let workbook_measured = workbook_phase(&store, &dir, corpus.merged_rows())?;

    let wall = started.elapsed().as_secs_f64();
    println!("metric=wall_seconds value={wall:.3} unit=s");
    let summary = json!({
        "harness": "bench_census",
        "schools": options.schools,
        "athletes": corpus.athletes.len(),
        "meets": corpus.meets.len(),
        "distinct_events": corpus.distinct_events.len(),
        "performances": corpus.performances.len(),
        "appended_rows": corpus.appended_rows(),
        "append": appended,
        "consolidate": consolidated,
        "census_core": core_census,
        "census_all_sources": all_sources_census,
        "bests": bests_measured,
        "best_rows": best_rows,
        "workbook": workbook_measured,
        "wall_seconds": wall,
    });
    println!("json={}", serde_json::to_string(&summary)?);
    println!(
        "note=store root {} is removed when this process exits",
        dir.path().display()
    );
    Ok(())
}

fn build_corpus(school_count: usize) -> Result<Corpus> {
    let mut corpus = Corpus::new();
    let mut rng = Lcg::new(SEED);
    for index in 0..school_count {
        append_school(&mut corpus, &mut rng, index)?;
    }
    Ok(corpus)
}

/// One school, its team, its meet, and the athletes that compete at that meet.
fn append_school(corpus: &mut Corpus, rng: &mut Lcg, index: usize) -> Result<()> {
    let state = match index % 3 {
        0 => "WI",
        1 => "MN",
        _ => "IA",
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
    let gender = if (index + slot).is_multiple_of(2) {
        Gender::Boys
    } else {
        Gender::Girls
    };
    let mut athlete = CanonicalAthlete::new(
        school_id,
        format!("Runner {index}-{slot}"),
        GradYear::CO2027,
        gender,
    );
    athlete.sports.push(Sport::OutdoorTrack);
    athlete.observed_grades.push(ObservedGrade {
        grade: grade(11)?,
        school_year: SchoolYear(2025),
        source: SourceRef::id(SOURCE_ID),
    });
    athlete.evidence.push(evidence());
    athlete.source_identities.push(SourceIdentity::new(
        SourceNamespace::MilesplitAthlete,
        format!("profile-{index}-{slot}"),
    ));
    athlete
        .public_profile_urls
        .push(format!("https://example.invalid/athletes/{index}-{slot}"));

    let kind = event_kind(rng.next());
    let mut event = CanonicalEvent::new(meet_id, kind.clone(), gender, None, None);
    event.evidence.push(evidence());
    corpus.distinct_events.insert(event.id.as_str().to_string());
    for attempt in 0..PERFORMANCES_PER_ATHLETE {
        let source_key = format!("perf-{index}-{slot}-{attempt}");
        let id = CanonicalPerformance::mint(&athlete.id, meet_id, &kind, MEET_DATE, &source_key);
        corpus.performances.push(CanonicalPerformance {
            id,
            athlete: athlete.id.clone(),
            team: team_id.clone(),
            event: event.id.clone(),
            meet: meet_id.clone(),
            date: MEET_DATE.to_string(),
            mark: Mark::TimeSeconds(base_seconds(&kind, rng)),
            wind_mps: None,
            place: Some(place_of(rng.next())?),
            heat: None,
            round: None,
            timing: Some(TimingMethod::Fat),
            observed_grade: Some(grade(11)?),
            evidence: vec![evidence()],
            source_key,
        });
    }
    corpus.events.push(event);
    corpus.athletes.push(athlete);
    Ok(())
}

/// Append every table, then require the store's own observation counts to match the corpus.
fn append_corpus(store: &Store, corpus: &Corpus) -> Result<Phase> {
    let started = Instant::now();
    append_all(store, Table::Schools, &corpus.schools)?;
    append_all(store, Table::Teams, &corpus.teams)?;
    append_all(store, Table::Athletes, &corpus.athletes)?;
    append_all(store, Table::Meets, &corpus.meets)?;
    append_all(store, Table::Events, &corpus.events)?;
    append_all(store, Table::Performances, &corpus.performances)?;
    let phase = measure("append", corpus.appended_rows(), "rows", started.elapsed())?;
    expect_observations(store, corpus)?;
    Ok(phase)
}

/// One `append_many` per `APPEND_BATCH` rows.
fn append_all<T: Serialize>(store: &Store, table: Table, rows: &[T]) -> Result<()> {
    for chunk in rows.chunks(APPEND_BATCH) {
        store
            .append_many(table, chunk)
            .with_context(|| format!("appending to {}", table.file()))?;
    }
    Ok(())
}

fn expect_observations(store: &Store, corpus: &Corpus) -> Result<()> {
    let expected = [
        (Table::Schools, corpus.schools.len()),
        (Table::Teams, corpus.teams.len()),
        (Table::Athletes, corpus.athletes.len()),
        (Table::Meets, corpus.meets.len()),
        (Table::Events, corpus.events.len()),
        (Table::Performances, corpus.performances.len()),
    ];
    let stats = store.stats().context("reading store stats")?;
    for (table, rows) in expected {
        let found = stats
            .tables
            .iter()
            .find(|(name, _)| name == table.file())
            .map(|(_, count)| *count)
            .unwrap_or(0);
        let rows = u64::try_from(rows).context("row count does not fit u64")?;
        anyhow::ensure!(
            found == rows,
            "{} holds {found} observations, expected {rows}",
            table.file()
        );
    }
    Ok(())
}

/// Merge every append log into `out/`, then require the merged counts to match the corpus.
fn consolidate_phase(store: &Store, corpus: &Corpus) -> Result<Phase> {
    let started = Instant::now();
    let counts = census::consolidate(store).context("consolidating the append logs")?;
    let written = merged_rows(&counts);
    let phase = measure("consolidate", written, "entities", started.elapsed())?;
    ensure_count(&counts, "schools", corpus.schools.len())?;
    ensure_count(&counts, "teams", corpus.teams.len())?;
    ensure_count(&counts, "athletes", corpus.athletes.len())?;
    ensure_count(&counts, "meets", corpus.meets.len())?;
    ensure_count(&counts, "events", corpus.distinct_events.len())?;
    ensure_count(&counts, "performances", corpus.performances.len())?;
    Ok(phase)
}

/// Rows over the seven real tables (the consolidated report also carries a `coaches_email_withheld`
/// counter, which is not a table).
fn merged_rows(counts: &[(String, usize)]) -> usize {
    counts
        .iter()
        .filter(|(table, _)| Table::ALL.iter().any(|known| known.file() == table))
        .map(|(_, count)| *count)
        .sum()
}

fn ensure_count(counts: &[(String, usize)], table: &str, expected: usize) -> Result<()> {
    let found = counts
        .iter()
        .find(|(name, _)| name == table)
        .map(|(_, count)| *count)
        .with_context(|| format!("consolidate did not report a count for {table}"))?;
    anyhow::ensure!(
        found == expected,
        "{table}: consolidated {found} rows, expected {expected}"
    );
    Ok(())
}

/// Both report scopes over the same consolidated snapshot.
fn census_phases(store: &Store, cohort: usize) -> Result<(Phase, Phase)> {
    let started = Instant::now();
    let core = report::build_census(store, Scope::Core).context("building the core census")?;
    let core_phase = measure("census_core", cohort, "athletes", started.elapsed())?;
    anyhow::ensure!(
        core.totals.athletes == cohort,
        "core census counted {} athletes, expected {cohort}",
        core.totals.athletes
    );
    anyhow::ensure!(
        core.totals.class_of_2027 == cohort,
        "core census counted {} class-of-2027 athletes, expected {cohort}",
        core.totals.class_of_2027
    );

    let started = Instant::now();
    let all_sources = report::build_census(store, Scope::AllSources)
        .context("building the all-sources census")?;
    let all_sources_phase = measure("census_all_sources", cohort, "athletes", started.elapsed())?;
    anyhow::ensure!(
        all_sources.totals.athletes == cohort,
        "all-sources census counted {} athletes, expected {cohort}",
        all_sources.totals.athletes
    );
    Ok((core_phase, all_sources_phase))
}

/// Reduce best marks and require exactly one row per athlete, each resting on both marks.
fn bests_phase(store: &Store, athletes: usize, performances: usize) -> Result<(Phase, usize)> {
    let started = Instant::now();
    let rows = bests::build(
        store,
        &bests::Options {
            scope: Scope::Core,
            grad_year: Some(2027),
            limit: None,
        },
    )
    .context("reducing best marks")?;
    let phase = measure("bests", performances, "performances", started.elapsed())?;
    anyhow::ensure!(
        rows.len() == athletes,
        "bests produced {} rows, expected {athletes}",
        rows.len()
    );
    for row in &rows {
        anyhow::ensure!(
            row.marks_in_event == PERFORMANCES_PER_ATHLETE,
            "athlete {} reduced {} marks, expected {}",
            row.athlete_id,
            row.marks_in_event,
            PERFORMANCES_PER_ATHLETE
        );
    }
    Ok((phase, rows.len()))
}

/// Build the workbook into the temporary directory and check the artifact is a real xlsx. The item
/// count is the number of merged rows the workbook reads, not the append count.
fn workbook_phase(store: &Store, dir: &TempDir, entities: usize) -> Result<Phase> {
    let out = dir.path().join("synthetic-census.xlsx");
    let started = Instant::now();
    let path = workbook::build(
        store,
        &workbook::Options {
            grad_year: Some(2027),
            out: Some(out.clone()),
            limit: None,
            scope: Scope::Core,
        },
    )
    .context("building the workbook")?;
    let phase = measure("workbook", entities, "entities", started.elapsed())?;
    anyhow::ensure!(
        path == out,
        "workbook was written to {} instead of {}",
        path.display(),
        out.display()
    );
    let bytes = std::fs::read(&path).context("reading the workbook back")?;
    anyhow::ensure!(
        bytes.starts_with(b"PK\x03\x04"),
        "workbook {} is not an xlsx (zip) container",
        path.display()
    );
    let eocd = bytes
        .len()
        .checked_sub(22)
        .context("workbook is too short to carry a zip end-of-central-directory record")?;
    let signature = bytes
        .get(eocd..eocd.saturating_add(4))
        .context("reading the zip end-of-central-directory signature")?;
    anyhow::ensure!(
        signature == b"PK\x05\x06".as_slice(),
        "workbook {} is a truncated zip container",
        path.display()
    );
    println!("metric=workbook_bytes value={} unit=bytes", bytes.len());
    println!("note=workbook path {}", path.display());
    Ok(phase)
}

fn evidence() -> Evidence {
    Evidence::parsed(SourceRef::id(SOURCE_ID), MEET_DATE)
}

fn grade(value: u8) -> Result<Grade> {
    Grade::new(value).context("grade must be between 9 and 12")
}

fn team_of(school_id: &SchoolId, index: usize) -> CanonicalTeam {
    let id: TeamId = Id::mint("team", &[school_id.as_str(), "outdoor", &index.to_string()]);
    CanonicalTeam {
        id,
        school: school_id.clone(),
        sport: Sport::OutdoorTrack,
        gender: Gender::Mixed,
        school_year: SchoolYear(2025),
        level: None,
        source_identities: Vec::new(),
        evidence: vec![evidence()],
    }
}

fn meet_of(state: &str, index: usize) -> CanonicalMeet {
    let mut meet = CanonicalMeet::new(
        state,
        format!("Synthetic Invite {index}"),
        MEET_DATE,
        CompetitionLevel::Invitational,
    );
    meet.evidence.push(evidence());
    meet.source_identities.push(SourceIdentity::new(
        SourceNamespace::TimerMeet {
            provider: "synthetic_timer".to_string(),
        },
        format!("meet-{index}"),
    ));
    meet
}

/// Three track events, so every meet holds a handful of distinct events rather than one.
fn event_kind(value: u32) -> EventKind {
    match value % 3 {
        0 => EventKind::Track800m,
        1 => EventKind::Track1600m,
        _ => EventKind::Track3200m,
    }
}

/// A plausible time for the event, jittered from 0.00 to 4.99 seconds.
fn base_seconds(kind: &EventKind, rng: &mut Lcg) -> f64 {
    let base = match kind {
        EventKind::Track800m => 130.0,
        EventKind::Track1600m => 290.0,
        _ => 620.0,
    };
    base + f64::from(rng.next() % 500) / 100.0
}

fn place_of(value: u32) -> Result<u16> {
    let lane = u16::try_from(value % 8).context("lane does not fit u16")?;
    Ok(lane.saturating_add(1))
}

/// Time a phase, print its machine-readable lines, and return the measurement.
fn measure(name: &str, items: usize, unit: &str, elapsed: Duration) -> Result<Phase> {
    let rate_per_second = per_second(items, elapsed)?;
    let seconds = elapsed.as_secs_f64();
    println!("metric={name}_items value={items} unit={unit}");
    println!("metric={name}_seconds value={seconds:.3} unit=s");
    println!("metric={name}_rate value={rate_per_second:.1} unit={unit}/s");
    Ok(Phase {
        items,
        seconds,
        rate_per_second,
    })
}

/// Items per second. Item counts are converted with `try_from` rather than a lossy cast.
fn per_second(items: usize, elapsed: Duration) -> Result<f64> {
    let items = u32::try_from(items).context("item count does not fit u32")?;
    let seconds = elapsed.as_secs_f64();
    anyhow::ensure!(seconds > 0.0, "elapsed time is not positive");
    Ok(f64::from(items) / seconds)
}
