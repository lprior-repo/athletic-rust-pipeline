//! §60 Fjall backup/restore drill: consistent backup → restore into a fresh data directory →
//! integrity verification → reopen → full census read.
//!
//! The drill follows the operator sequence in `docs/OPERATIONS.md` (stop the unit, copy the whole
//! data directory) and asserts what a restore must reproduce: every per-table observation count, the
//! merged observation history of one entity, both census scopes, the best-mark reduction, and the
//! consolidated JSONL snapshots the read model is built from. `docs/FJALL_BACKUP.md` records the
//! command lines and the raw output of the run that produced these assertions.
//!
//! The store's public API is the only thing the drill touches: `midwest-census` owns the Fjall
//! handle, and the census `Store` exposes neither the `Database` nor a `snapshot()` borrow, so a
//! backup is a *file* operation on a stopped store and the consistency argument is the cold copy
//! itself, not a read view. [`a_copy_taken_while_the_store_handle_is_open_keeps_every_committed_batch`]
//! pins the durability property that makes that copy complete, and
//! [`a_second_open_of_a_live_store_is_refused`] pins the lock that forces the cold copy.
//!
//! Nothing here touches the network: the corpus is synthetic and every byte the drill compares comes
//! from a temporary directory.

use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet,
    CanonicalPerformance, CanonicalSchool, CanonicalTeam, CoachRole, CompetitionLevel, EventKind,
    Evidence, Gender, GradYear, Grade, Id, Mark, ObservedGrade, SchoolId, SchoolYear, SourceRef,
    Sport, TeamId, TimingMethod,
};
use census_domain::UsJurisdiction;
use midwest_census::report::{self, Census, Scope};
use midwest_census::store::{Store, StoreError, StoreStats, Table};
use midwest_census::{bests, census};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// A core adapter id: never one of `report::NON_CORE_SOURCE_IDS`, so both census scopes keep every
/// synthetic row.
const SOURCE: &str = "wiaa_schools";
/// The second source that observes the history school, so the merged entity has to union evidence
/// from two origins rather than concatenating one.
const SECOND_SOURCE: &str = "mshsl_schools";
const FIRST_DATE: &str = "2026-09-01";
const SECOND_DATE: &str = "2026-09-02";
const THIRD_DATE: &str = "2026-09-05";
const AFTER_RESTORE_DATE: &str = "2026-09-08";
const MEET_DATE: &str = "2026-05-02";
/// Observations of the history school beyond its corpus row. The corpus row is observation one.
const HISTORY_EXTRA_OBSERVATIONS: usize = 2;

fn observation(source: &str, observed_on: &str) -> Evidence {
    Evidence::parsed(SourceRef::id(source), observed_on)
}

// -------------------------------------------------------------------------------------------------
// Corpus
// -------------------------------------------------------------------------------------------------

/// A synthetic season that fills every table the census reads: three schools in three states, each
/// with a team, a coach, a meet, two athletes, one event per athlete and two performances per
/// athlete.
struct Corpus {
    schools: Vec<CanonicalSchool>,
    teams: Vec<CanonicalTeam>,
    coaches: Vec<CanonicalCoach>,
    athletes: Vec<CanonicalAthlete>,
    meets: Vec<CanonicalMeet>,
    events: Vec<CanonicalEvent>,
    performances: Vec<CanonicalPerformance>,
    /// Event ids minted while building, so a duplicate id would make the count assertion wrong
    /// instead of silently inflating it.
    distinct_events: BTreeSet<String>,
    /// The school the drill re-observes. Its merged evidence is the observation history the
    /// restored store has to reproduce row for row.
    history_school: SchoolId,
}

impl Corpus {
    fn append(&self, store: &Store) {
        store
            .append_many(Table::Schools, &self.schools)
            .expect("appending schools");
        store
            .append_many(Table::Teams, &self.teams)
            .expect("appending teams");
        store
            .append_many(Table::Coaches, &self.coaches)
            .expect("appending coaches");
        store
            .append_many(Table::Athletes, &self.athletes)
            .expect("appending athletes");
        store
            .append_many(Table::Meets, &self.meets)
            .expect("appending meets");
        store
            .append_many(Table::Events, &self.events)
            .expect("appending events");
        store
            .append_many(Table::Performances, &self.performances)
            .expect("appending performances");
    }

    /// The corpus row of the history school — observation one of three.
    fn history_row(&self) -> &CanonicalSchool {
        self.schools
            .iter()
            .find(|row| row.id == self.history_school)
            .expect("the corpus holds the history school")
    }
}

fn corpus() -> Corpus {
    let mut corpus = Corpus {
        schools: Vec::new(),
        teams: Vec::new(),
        coaches: Vec::new(),
        athletes: Vec::new(),
        meets: Vec::new(),
        events: Vec::new(),
        performances: Vec::new(),
        distinct_events: BTreeSet::new(),
        history_school: CanonicalSchool::mint(
            UsJurisdiction::Wisconsin,
            "Drill High School",
            "drill high school",
        ),
    };
    let history = add_school(
        &mut corpus,
        UsJurisdiction::Wisconsin,
        "Drill High School",
        0,
    );
    corpus.history_school = history;
    add_school(
        &mut corpus,
        UsJurisdiction::Minnesota,
        "Drill North High School",
        1,
    );
    add_school(
        &mut corpus,
        UsJurisdiction::Iowa,
        "Drill West High School",
        2,
    );
    corpus
}

/// One school with its team, coach, meet and two athletes. Returns the school id.
fn add_school(
    corpus: &mut Corpus,
    jurisdiction: UsJurisdiction,
    name: &str,
    index: usize,
) -> SchoolId {
    let (mut school, school_id) = CanonicalSchool::new(jurisdiction, name, normalize_name(name));
    school.evidence.push(observation(SOURCE, FIRST_DATE));
    let team = CanonicalTeam {
        id: Id::mint("team", &[school_id.as_str(), "outdoor", "2026"]),
        school: school_id.clone(),
        sport: Sport::OutdoorTrack,
        gender: Gender::Mixed,
        school_year: SchoolYear(2025),
        level: None,
        source_identities: Vec::new(),
        evidence: vec![observation(SOURCE, FIRST_DATE)],
    };
    let team_id = team.id.clone();
    let mut coach = CanonicalCoach::new(
        &school_id,
        format!("Drill Coach {index}"),
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    coach.professional_email = Some(format!("coach{index}@drill.k12.wi.us"));
    coach.evidence.push(observation(SOURCE, FIRST_DATE));
    let mut meet = CanonicalMeet::new(
        Some(jurisdiction),
        format!("Drill Invite {index}"),
        MEET_DATE,
        CompetitionLevel::Invitational,
    );
    meet.evidence.push(observation(SOURCE, MEET_DATE));
    let meet_id = meet.id.clone();
    corpus.schools.push(school);
    corpus.teams.push(team);
    corpus.coaches.push(coach);
    corpus.meets.push(meet);
    for slot in 0..2 {
        add_athlete(corpus, index, slot, &school_id, &team_id, &meet_id);
    }
    school_id
}

/// One athlete with one event and two performances of that event — the shape the best-mark
/// reduction needs to have something to reduce.
fn add_athlete(
    corpus: &mut Corpus,
    index: usize,
    slot: usize,
    school_id: &SchoolId,
    team_id: &TeamId,
    meet_id: &census_domain::model::MeetId,
) {
    let gender = if (index + slot).is_multiple_of(2) {
        Gender::Boys
    } else {
        Gender::Girls
    };
    let mut athlete = CanonicalAthlete::new(
        school_id,
        format!("Drill Runner {index}-{slot}"),
        GradYear::CO2027,
        gender,
    );
    athlete.sports.push(Sport::OutdoorTrack);
    athlete.observed_grades.push(ObservedGrade {
        grade: Grade::new(11).expect("grade 11 is a school grade"),
        school_year: SchoolYear(2025),
        source: SourceRef::id(SOURCE),
    });
    athlete.evidence.push(observation(SOURCE, MEET_DATE));

    let kind = if slot.is_multiple_of(2) {
        EventKind::Track800m
    } else {
        EventKind::Track1600m
    };
    let mut event = CanonicalEvent::new(meet_id, kind.clone(), gender, None, None);
    event.evidence.push(observation(SOURCE, MEET_DATE));
    corpus.distinct_events.insert(event.id.as_str().to_string());
    for attempt in 0..2 {
        let source_key = format!("drill-{index}-{slot}-{attempt}");
        corpus.performances.push(CanonicalPerformance {
            id: CanonicalPerformance::mint(&athlete.id, meet_id, &kind, MEET_DATE, &source_key),
            athlete: athlete.id.clone(),
            team: team_id.clone(),
            event: event.id.clone(),
            meet: meet_id.clone(),
            date: MEET_DATE.to_string(),
            mark: Mark::TimeSeconds(
                130.0
                    + f64::from(u32::try_from(index + slot).expect("small"))
                    + f64::from(u32::try_from(attempt).expect("small")),
            ),
            wind_mps: None,
            place: Some(u16::try_from(attempt + 1).expect("two attempts fit u16")),
            heat: None,
            round: None,
            timing: Some(TimingMethod::Fat),
            observed_grade: Some(Grade::new(11).expect("grade 11 is a school grade")),
            evidence: vec![observation(SOURCE, MEET_DATE)],
            source_key,
        });
    }
    corpus.events.push(event);
    corpus.athletes.push(athlete);
}

/// Observations two and three of the history school: the second fills a field the first left empty
/// (the merge has work to do), the third repeats the first source on a later day (the evidence union
/// has to keep both rows rather than dedupe them).
fn add_history(store: &Store, first: &CanonicalSchool) {
    let mut second = first.clone();
    second.city = Some("Drill City".to_string());
    second.enrollment = Some(420);
    second.evidence = vec![observation(SECOND_SOURCE, SECOND_DATE)];
    let mut third = first.clone();
    third.co_op = true;
    third.evidence = vec![observation(SOURCE, THIRD_DATE)];
    store
        .append(Table::Schools, &second)
        .expect("appending the second history observation");
    store
        .append(Table::Schools, &third)
        .expect("appending the third history observation");
}

// -------------------------------------------------------------------------------------------------
// Read model
// -------------------------------------------------------------------------------------------------

/// Everything the drill reads out of one store. Captured from the live store before the copy and
/// from the restored copy after it; every field must be identical.
struct ReadModel {
    tables: Vec<(String, u64)>,
    observations: u64,
    merged_schools: usize,
    history: CanonicalSchool,
    core: String,
    all_sources: String,
    bests: String,
    counts: Vec<(String, usize)>,
    snapshots: Vec<(String, String)>,
}

/// Steps 1-3 of the drill, in the order an operator runs them, plus the corpus the store was built
/// from and the root the backup was restored into.
///
/// 1. build the live store and stop the unit (the handle drops: Fjall holds an exclusive lock, so a
///    live store cannot be opened — let alone copied — by a second process),
/// 2. consistent backup: cold copy of the whole data directory,
/// 3. restore: copy the backup's `fjall/` database into a fresh data directory.
fn drill(root: &Path) -> (Corpus, PathBuf) {
    let live = root.join("live");
    let backup = root.join("backup");
    let restored = root.join("restored");
    let corpus = corpus();

    // Step 1: the census an operator would have collected, plus the resume journal that records
    // which units of work finished.
    {
        let store = Store::open(&live).expect("opening the live store");
        corpus.append(&store);
        add_history(&store, corpus.history_row());
        store
            .journal_done(
                "drill_phase",
                "wi:drill",
                &serde_json::json!({"schools": 3}),
            )
            .expect("recording the finished unit of work");
        store.flush().expect("flushing the live store");
    }

    // Step 2: consistent backup.
    copy_tree(&live, &backup);
    assert_eq!(
        tree_digest(&live.join("fjall")),
        tree_digest(&backup.join("fjall")),
        "the backup must be a byte image of the stopped store's database"
    );

    // Step 3: restore. Only `fjall/` is needed — `Store::open` recreates the cache and output
    // directories — so this is also the minimum restore set.
    copy_tree(&backup.join("fjall"), &restored.join("fjall"));
    assert_eq!(
        tree_digest(&restored.join("fjall")),
        tree_digest(&backup.join("fjall")),
        "the restored database must be a byte image of the backup"
    );

    (corpus, restored)
}

/// Read every surface a restored store has to reproduce.
fn capture(store: &Store, history_id: &SchoolId) -> ReadModel {
    let stats = store.stats().expect("store stats");
    let schools = store
        .scan::<CanonicalSchool>(Table::Schools)
        .expect("scanning schools");
    let history = schools
        .iter()
        .find(|row| &row.id == history_id)
        .expect("the merged history school")
        .clone();
    ReadModel {
        tables: stats.tables,
        observations: stats.observations,
        merged_schools: schools.len(),
        history,
        core: census_json(
            &report::build_census(store, Scope::Core).expect("core census"),
            store.root(),
        ),
        all_sources: census_json(
            &report::build_census(store, Scope::AllSources).expect("all-sources census"),
            store.root(),
        ),
        bests: serde_json::to_string_pretty(
            &bests::build(
                store,
                &bests::Options {
                    scope: Scope::Core,
                    grad_year: Some(2027),
                    limit: None,
                },
            )
            .expect("best marks"),
        )
        .expect("best marks serialize"),
        counts: census::consolidate(store).expect("consolidating the store"),
        snapshots: snapshots(&store.out_dir()),
    }
}

/// The consolidated snapshots the report and the workbook consume, as text.
fn snapshots(out: &Path) -> Vec<(String, String)> {
    ["schools", "coaches", "performances"]
        .iter()
        .map(|name| {
            let path = out.join(format!("{name}.jsonl"));
            let body = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("snapshot {} is missing: {error}", path.display()));
            ((*name).to_string(), body)
        })
        .collect()
}

/// A census document with the root path and the run date blanked out: `store_dir`, the output
/// directory the notes name, and `generated_on` all describe the *run*, not the data, and a restored
/// copy is read from a different root. Everything else must be identical.
fn census_json(census: &Census, root: &Path) -> String {
    let mut value = serde_json::to_value(census).expect("a census serializes");
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "generated_on".to_string(),
            serde_json::Value::String("<drill>".to_string()),
        );
    }
    let text = serde_json::to_string_pretty(&value).expect("a census value serializes");
    text.replace(&root.display().to_string(), "<drill>")
}

/// Compare two captured documents field by field. A restore that changes one number must say which
/// one — a raw string comparison of a whole census prints two walls of JSON and names nothing.
fn assert_same_json(label: &str, left: &str, right: &str) {
    let left_value: serde_json::Value = serde_json::from_str(left)
        .unwrap_or_else(|error| panic!("{label}: left is not JSON: {error}"));
    let right_value: serde_json::Value = serde_json::from_str(right)
        .unwrap_or_else(|error| panic!("{label}: right is not JSON: {error}"));
    if let Some((path, left_at, right_at)) = first_difference(&left_value, &right_value, "") {
        panic!("{label} differs at {path}: {left_at} != {right_at}");
    }
}

/// The first field path where two documents differ, with the two values at that path.
fn first_difference(
    left: &serde_json::Value,
    right: &serde_json::Value,
    path: &str,
) -> Option<(String, String, String)> {
    if left == right {
        return None;
    }
    match (left, right) {
        (serde_json::Value::Object(left_map), serde_json::Value::Object(right_map)) => {
            for key in left_map.keys().chain(right_map.keys()) {
                let step = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                match (left_map.get(key), right_map.get(key)) {
                    (Some(left_value), Some(right_value)) => {
                        if let Some(found) = first_difference(left_value, right_value, &step) {
                            return Some(found);
                        }
                    }
                    (left_value, right_value) => {
                        return Some((step, render(left_value), render(right_value)));
                    }
                }
            }
            None
        }
        (serde_json::Value::Array(left_rows), serde_json::Value::Array(right_rows))
            if left_rows.len() == right_rows.len() =>
        {
            for (index, (left_row, right_row)) in left_rows.iter().zip(right_rows).enumerate() {
                if let Some(found) =
                    first_difference(left_row, right_row, &format!("{path}[{index}]"))
                {
                    return Some(found);
                }
            }
            None
        }
        _ => Some((path.to_string(), render(Some(left)), render(Some(right)))),
    }
}

/// A value short enough to fit in a panic message.
fn render(value: Option<&serde_json::Value>) -> String {
    let text = match value {
        Some(value) => value.to_string(),
        None => "<absent>".to_string(),
    };
    let mut out: String = text.chars().take(160).collect();
    if out.len() < text.len() {
        out.push('…');
    }
    out
}

/// One count out of a captured census document, addressed by field path.
fn census_field(census: &str, path: &[&str]) -> u64 {
    let value: serde_json::Value =
        serde_json::from_str(census).expect("a captured census is valid JSON");
    let mut cursor = &value;
    for step in path {
        cursor = cursor
            .get(step)
            .unwrap_or_else(|| panic!("census has no {} field", path.join(".")));
    }
    cursor
        .as_u64()
        .unwrap_or_else(|| panic!("census field {} is not an integer", path.join(".")))
}

/// The per-table observation counts the corpus must produce, counted rather than scanned: a restore
/// that loses the same rows on both sides of the comparison must not pass.
fn expected_observations(corpus: &Corpus) -> Vec<(String, u64)> {
    let row = |table: Table, rows: usize| {
        let rows = u64::try_from(rows).expect("a corpus table holds fewer than 2^64 rows");
        (table.file().to_string(), rows)
    };
    // `stats()` reports every table in `Table::ALL` order, so both groups below are listed in that
    // order: the seven evidence tables the corpus writes, then the five derived index tables. The
    // drill's chain never runs the derivation, so the index tables must be empty here — they are
    // listed at zero rather than omitted, because a store that suddenly held derived rows would be a
    // different store and this assertion is what says so.
    let evidence = [
        row(
            Table::Schools,
            corpus.schools.len() + HISTORY_EXTRA_OBSERVATIONS,
        ),
        row(Table::Teams, corpus.teams.len()),
        row(Table::Coaches, corpus.coaches.len()),
        row(Table::Athletes, corpus.athletes.len()),
        row(Table::Meets, corpus.meets.len()),
        row(Table::Events, corpus.distinct_events.len()),
        row(Table::Performances, corpus.performances.len()),
    ];
    let derived = [
        Table::SourceIdentities,
        Table::Conflicts,
        Table::ReviewCases,
        Table::Coverage,
        Table::Snapshots,
        Table::SourceAccess,
        Table::IdentityVerdicts,
        // No meet census runs in this fixture, so the table that stage writes is empty here.
        Table::SourceMeets,
    ]
    .into_iter()
    .map(|table| row(table, 0));
    let mut expected: Vec<(String, u64)> = evidence.to_vec();
    expected.extend(derived);
    expected
}

/// The live store must hold exactly the corpus, merged where the corpus observes an entity twice.
fn assert_live_store_matches_corpus(before: &ReadModel, corpus: &Corpus) {
    assert_eq!(
        before.tables,
        expected_observations(corpus),
        "the live store's per-table observation counts"
    );
    assert_eq!(
        before.merged_schools,
        corpus.schools.len(),
        "three observations of the history school merge into one row"
    );
    assert_eq!(
        census_field(&before.core, &["totals", "schools"]),
        u64::try_from(corpus.schools.len()).expect("small"),
    );
    assert_eq!(
        census_field(&before.all_sources, &["totals", "athletes"]),
        u64::try_from(corpus.athletes.len()).expect("small"),
    );
    assert_eq!(
        census_field(&before.all_sources, &["totals", "class_of_2027"]),
        u64::try_from(corpus.athletes.len()).expect("small"),
    );
    assert_eq!(
        census_field(&before.all_sources, &["totals", "coaches"]),
        u64::try_from(corpus.coaches.len()).expect("small"),
    );
    let bests: Vec<serde_json::Value> =
        serde_json::from_str(&before.bests).expect("the best-mark reduction is a JSON array");
    assert_eq!(
        bests.len(),
        corpus.athletes.len(),
        "one best mark per athlete"
    );
}

/// The history school's merged evidence, checked against the corpus rather than against the other
/// store: a restore that drops a row on both sides must still fail here.
fn assert_history(school: &CanonicalSchool) {
    let observed: Vec<(String, String)> = school
        .evidence
        .iter()
        .map(|row| (row.source.id.clone(), row.observed_on.clone()))
        .collect();
    assert_eq!(
        observed,
        vec![
            (SOURCE.to_string(), FIRST_DATE.to_string()),
            (SECOND_SOURCE.to_string(), SECOND_DATE.to_string()),
            (SOURCE.to_string(), THIRD_DATE.to_string()),
        ],
        "the merged history keeps all three observations in append order"
    );
    assert_eq!(school.city.as_deref(), Some("Drill City"));
    assert_eq!(school.enrollment, Some(420));
    assert!(school.co_op, "the third observation set co_op");
}

// -------------------------------------------------------------------------------------------------
// Filesystem helpers
// -------------------------------------------------------------------------------------------------

/// Recursively copy `from` into a fresh `to`, byte for byte. Directories are walked in sorted order
/// so a failure names the same file on every run.
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap_or_else(|error| {
        panic!("creating {} failed: {error}", to.display());
    });
    let mut entries: Vec<PathBuf> = fs::read_dir(from)
        .unwrap_or_else(|error| panic!("reading {} failed: {error}", from.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|error| panic!("reading {} failed: {error}", from.display()))
                .path()
        })
        .collect();
    entries.sort();
    for path in entries {
        let name = path
            .file_name()
            .unwrap_or_else(|| panic!("{} has no file name", path.display()));
        let target = to.join(name);
        if path.is_dir() {
            copy_tree(&path, &target);
        } else {
            fs::copy(&path, &target).unwrap_or_else(|error| {
                panic!(
                    "copying {} to {} failed: {error}",
                    path.display(),
                    target.display()
                )
            });
        }
    }
}

/// Every file under `root` as `(relative path, sha256)`, sorted — the manifest an operator diffs to
/// prove a copy is faithful instead of trusting that the database opened.
fn file_digests(root: &Path) -> Vec<(String, String)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
        let mut entries: Vec<PathBuf> = fs::read_dir(dir)
            .unwrap_or_else(|error| panic!("reading {} failed: {error}", dir.display()))
            .map(|entry| {
                entry
                    .unwrap_or_else(|error| panic!("reading {} failed: {error}", dir.display()))
                    .path()
            })
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .unwrap_or_else(|_| {
                        panic!("{} is not under {}", path.display(), root.display())
                    })
                    .display()
                    .to_string();
                out.push((relative, sha256_file(&path)));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

fn sha256_file(path: &Path) -> String {
    let bytes =
        fs::read(path).unwrap_or_else(|error| panic!("reading {} failed: {error}", path.display()));
    format!("{:x}", Sha256::digest(&bytes))
}

/// One digest over the whole tree: the same manifest, folded into a single line.
fn tree_digest(root: &Path) -> String {
    let mut hasher = Sha256::new();
    for (path, digest) in file_digests(root) {
        hasher.update(path.as_bytes());
        hasher.update([0_u8]);
        hasher.update(digest.as_bytes());
        hasher.update(*b"\n");
    }
    format!("{:x}", hasher.finalize())
}

fn table_count(stats: &StoreStats, table: Table) -> u64 {
    stats
        .tables
        .iter()
        .find(|(name, _)| name == table.file())
        .map(|(_, count)| *count)
        .unwrap_or_else(|| panic!("stats reported no count for {}", table.file()))
}

/// The merged row of one school.
fn merged_school(store: &Store, id: &SchoolId) -> CanonicalSchool {
    store
        .scan::<CanonicalSchool>(Table::Schools)
        .expect("scanning schools")
        .into_iter()
        .find(|row| &row.id == id)
        .expect("the merged history school")
}

/// One row of the damaged-copy stores: batch `batch`, slot `slot`.
fn batch_row(batch: usize, slot: usize) -> CanonicalSchool {
    let index = batch * 2 + slot;
    let name = format!("Batch School {index}");
    CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        name.clone(),
        normalize_name(&name),
    )
    .0
}

/// The ids of the first `count` batch rows, minted exactly as [`batch_store`] mints them.
fn batch_ids(count: usize) -> BTreeSet<String> {
    (0..count)
        .map(|index| batch_row(index / 2, index % 2).id.as_str().to_string())
        .collect()
}

/// Append one batch of two schools and close the store again.
fn append_batch(root: &Path, batch: usize) {
    let store = Store::open(root).expect("opening the batch store");
    let rows: Vec<CanonicalSchool> = (0..2).map(|slot| batch_row(batch, slot)).collect();
    store
        .append_many(Table::Schools, &rows)
        .expect("appending a batch");
    store.flush().expect("flushing the batch store");
}

/// A store built for the damaged-copy tests: `batches` batches of two schools each, every batch
/// committed and the store closed again. Returns the observation count, and the journal length after
/// the *first* batch - a real frame edge, because closing and reopening a store settles the journal
/// to the frames that were actually written (fjall preallocates a fresh journal to 64 MiB,
/// `fjall-3.1.10/src/journal/writer.rs:19`, and only recovery truncates it back).
fn batch_store(root: &Path, batches: usize) -> (u64, u64) {
    let mut boundary = 0_u64;
    for batch in 0..batches {
        if batch == 1 {
            drop(Store::open(root).expect("settling the batch store"));
            boundary = journal_len(root);
        }
        append_batch(root, batch);
    }
    let store = Store::open(root).expect("reopening the batch store");
    let rows = store.stats().expect("batch store stats").observations;
    drop(store);
    (rows, boundary)
}

/// Length of a store's active journal, once a reopen has settled it to the recorded prefix.
fn journal_len(root: &Path) -> u64 {
    let journal = journal_file(root);
    fs::metadata(&journal)
        .unwrap_or_else(|error| panic!("reading {} failed: {error}", journal.display()))
        .len()
}

/// Cut the copy's journal short and read the copy back: the rows that survive, and the schools they
/// merge into.
fn cut_and_read(copy: &Path, full: &[u8], cut: usize) -> (u64, BTreeSet<String>) {
    let journal = journal_file(copy);
    fs::write(
        &journal,
        full.get(..cut).unwrap_or_else(|| {
            panic!(
                "cut {cut} is past the {} bytes of {}",
                full.len(),
                journal.display()
            )
        }),
    )
    .unwrap_or_else(|error| panic!("writing {} failed: {error}", journal.display()));
    let store = Store::open(copy).expect("a cut journal must not stop the store opening");
    let rows = store.stats().expect("copy stats").observations;
    (rows, school_ids(&store))
}

/// The active Fjall journal file under a store's `fjall/` directory.
fn journal_file(root: &Path) -> PathBuf {
    let fjall = root.join("fjall");
    let mut found: Vec<PathBuf> = fs::read_dir(&fjall)
        .unwrap_or_else(|error| panic!("reading {} failed: {error}", fjall.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|error| panic!("reading {} failed: {error}", fjall.display()))
                .path()
        })
        .filter(|path| path.extension().is_some_and(|extension| extension == "jnl"))
        .collect();
    found.sort();
    found
        .pop()
        .unwrap_or_else(|| panic!("no journal file under {}", fjall.display()))
}

/// The school ids a store holds, for a subset check that does not depend on counts.
fn school_ids(store: &Store) -> BTreeSet<String> {
    store
        .scan::<CanonicalSchool>(Table::Schools)
        .expect("scanning schools")
        .into_iter()
        .map(|row| row.id.as_str().to_string())
        .collect()
}

/// Every school a store holds, keyed by id, as the store serializes it: a restored or damaged copy
/// must reproduce these strings exactly or not have the row at all.
fn school_rows(store: &Store) -> BTreeMap<String, String> {
    store
        .scan::<CanonicalSchool>(Table::Schools)
        .expect("scanning schools")
        .into_iter()
        .map(|row| {
            (
                row.id.as_str().to_string(),
                serde_json::to_string(&row).expect("a school serializes"),
            )
        })
        .collect()
}

// -------------------------------------------------------------------------------------------------
// Drill
// -------------------------------------------------------------------------------------------------

#[test]
fn cold_copy_backup_restores_the_read_model_exactly() {
    let dir = tempfile::tempdir().expect("a temporary drill directory");

    // Steps 1-3: collect, stop, back up cold, restore into a fresh data directory.
    let (corpus, restored_root) = drill(dir.path());
    let before_root = dir.path().join("live");

    // Step 4: reopen the restored copy and read it back.
    let before = {
        let store = Store::open(&before_root).expect("reopening the stopped live store");
        capture(&store, &corpus.history_school)
    };
    let after = {
        let store = Store::open(&restored_root).expect("opening the restored store");
        capture(&store, &corpus.history_school)
    };

    // The corpus first, so a drill that loses the same rows on both sides still fails.
    assert_live_store_matches_corpus(&before, &corpus);
    assert_history(&before.history);

    // Then the restore claim: the restored copy is the live store, surface for surface.
    assert_eq!(
        after.tables, before.tables,
        "restored per-table observation counts"
    );
    assert_eq!(
        after.observations, before.observations,
        "restored observation total"
    );
    assert_eq!(
        after.merged_schools, before.merged_schools,
        "restored merged school count"
    );
    assert_eq!(
        after.history, before.history,
        "the history school's merged observation history"
    );
    assert_same_json("the core-scope census", &before.core, &after.core);
    assert_same_json(
        "the all-sources census",
        &before.all_sources,
        &after.all_sources,
    );
    assert_same_json("the best-mark reduction", &before.bests, &after.bests);
    assert_eq!(after.counts, before.counts, "the consolidated row counts");
    assert_eq!(
        after.snapshots, before.snapshots,
        "the consolidated snapshots"
    );
}

#[test]
fn the_restored_store_reopens_and_appends_without_overwriting() {
    let dir = tempfile::tempdir().expect("a temporary drill directory");
    let (corpus, restored_root) = drill(dir.path());

    let reopened = Store::open(&restored_root).expect("reopening the restored store");
    let restored_history = merged_school(&reopened, &corpus.history_school);
    assert_history(&restored_history);
    let before = reopened.stats().expect("restored store stats");

    // A write on the reopened copy: a restore that seeded its sequence counter wrong would hand out
    // a sequence number that is already stored and overwrite a restored observation instead of
    // appending one.
    let mut fourth = restored_history.clone();
    fourth.evidence = vec![observation(SECOND_SOURCE, AFTER_RESTORE_DATE)];
    reopened
        .append(Table::Schools, &fourth)
        .expect("appending after the restore");
    reopened.flush().expect("flushing the append");
    let after = reopened.stats().expect("stats after the append");
    assert_eq!(
        table_count(&after, Table::Schools),
        table_count(&before, Table::Schools) + 1,
        "the append is a new observation row"
    );
    let appended = merged_school(&reopened, &corpus.history_school);
    assert_eq!(
        appended.evidence.len(),
        4,
        "the appended observation joins the history instead of replacing it"
    );
    assert_eq!(
        appended.evidence.last().map(|row| row.observed_on.as_str()),
        Some(AFTER_RESTORE_DATE)
    );
    drop(reopened);

    // Reopen once more: the append has to survive the same durability boundary the restored rows
    // did, and the sequence has to resume from the restored store's high-water mark.
    let again = Store::open(&restored_root).expect("reopening after the append");
    let final_history = merged_school(&again, &corpus.history_school);
    assert_eq!(final_history.evidence.len(), 4);
    assert_eq!(
        table_count(&again.stats().expect("final stats"), Table::Schools),
        table_count(&before, Table::Schools) + 1
    );
}

#[test]
fn a_copy_taken_while_the_store_handle_is_open_keeps_every_committed_batch() {
    // The unit is stopped but the process is still up: `append_many` commits with `SyncData`, so the
    // journal on disk already holds every returned batch and a file copy needs no `Store::flush`
    // (SyncAll) to be complete. Regression this pins: a durability downgrade to `Buffer` would make
    // the copy lose the unpersisted tail — silently, which is the failure mode a backup cannot have.
    //
    // Nothing here protects a copy taken *while* a batch is mid-commit; that window is recorded as
    // not covered in `docs/FJALL_BACKUP.md`.
    let dir = tempfile::tempdir().expect("a temporary drill directory");
    let live = dir.path().join("live");
    let copy = dir.path().join("copy");
    let corpus = corpus();

    let store = Store::open(&live).expect("opening the live store");
    corpus.append(&store);
    add_history(&store, corpus.history_row());
    // Deliberately no `flush()`: the copy is taken from a live handle.
    let before = store.stats().expect("live store stats");
    copy_tree(&live.join("fjall"), &copy.join("fjall"));

    let restored = Store::open(&copy).expect("opening the copy taken from the open store");
    let after = restored.stats().expect("copy stats");
    assert_eq!(after.observations, before.observations);
    assert_eq!(after.tables, before.tables);
    assert_history(&merged_school(&restored, &corpus.history_school));
}

#[test]
fn a_second_open_of_a_live_store_is_refused() {
    // Why the drill's copy is cold: Fjall takes an exclusive lock on the database directory, so a
    // live store cannot be opened — and therefore cannot be read or copied by the census — a second
    // time. A backup run against a live unit fails here instead of copying a moving database.
    let dir = tempfile::tempdir().expect("a temporary drill directory");
    let live = Store::open(dir.path()).expect("opening the live store");
    let error = Store::open(dir.path())
        .err()
        .expect("a second open of a live store must be refused");
    assert!(
        matches!(error, StoreError::Open { .. }),
        "expected the typed open failure, got {error:?}"
    );
    drop(live);
    Store::open(dir.path()).expect("the path is openable once the live store is dropped");
}

#[test]
fn the_restored_store_does_not_re_import_the_legacy_journals_it_carries() {
    // A whole-root backup carries `entities/*.jsonl` and `journal/*.jsonl` next to the database, and
    // the import markers live in the `meta` keyspace. A restore that loses the markers re-imports
    // every legacy observation on the next open, doubling the counts in exactly the tables that
    // still have a log.
    let dir = tempfile::tempdir().expect("a temporary drill directory");
    let live = dir.path().join("live");
    let backup = dir.path().join("backup");
    let restored = dir.path().join("restored");
    write_legacy_journals(&live);

    let (tables, observations, keys) = {
        let store = Store::open(&live).expect("importing the legacy journals");
        let stats = store.stats().expect("store stats");
        let keys = store
            .journal_keys("drill_phase")
            .expect("the imported resume keys");
        (stats.tables, stats.observations, keys)
    };
    assert_eq!(
        tables
            .iter()
            .find(|(name, _)| name == "schools")
            .map(|(_, count)| *count),
        Some(2),
        "the legacy log holds one school under two observations"
    );
    assert_eq!(observations, 2);

    copy_tree(&live, &backup);
    copy_tree(&backup, &restored);

    for attempt in 1..=2 {
        let store = Store::open(&restored)
            .unwrap_or_else(|error| panic!("restored open {attempt} failed: {error}"));
        let stats = store.stats().expect("restored store stats");
        assert_eq!(
            stats.tables, tables,
            "restored open {attempt}: the markers in the copied `meta` keyspace must stop the import"
        );
        assert_eq!(stats.observations, observations);
        assert_eq!(
            store.journal_keys("drill_phase").expect("resume keys"),
            keys,
            "restored open {attempt}: the resume journal must not be imported twice"
        );
    }
}

#[test]
fn a_copy_whose_journal_stops_mid_batch_keeps_the_complete_prefix() {
    // A file copy that catches a *commit* in flight produces exactly this: a journal whose last frame
    // is incomplete. Fjall's reader drops the incomplete tail and keeps every complete batch
    // (`fjall-3.1.10/src/journal/reader.rs:56-79`), so the copy opens and reads back the prefix — a
    // hot copy loses the tail *silently*, which is why the drill states the shape of that loss
    // instead of pretending it cannot happen.
    let dir = tempfile::tempdir().expect("a temporary drill directory");
    let live = dir.path().join("live");
    let copy = dir.path().join("copy");
    let (source_rows, after_first_batch) = batch_store(&live, 3);
    assert_eq!(source_rows, 6, "three batches of two observations");
    assert!(
        after_first_batch > 0,
        "the first batch left no frames behind"
    );

    copy_tree(&live, &copy);
    let journal = journal_file(&copy);
    let full = fs::read(&journal)
        .unwrap_or_else(|error| panic!("reading {} failed: {error}", journal.display()));
    let settled = u64::try_from(full.len()).expect("the journal fits u64");
    assert!(
        after_first_batch + 1 < settled,
        "the cut points must sit inside the {settled}-byte journal"
    );

    // One byte into the second batch's first frame: the second and third batches are gone.
    let (rows, ids) = cut_and_read(
        &copy,
        &full,
        usize::try_from(after_first_batch + 1).expect("fits"),
    );
    assert_eq!(
        rows, 2,
        "the copy keeps the batches that finished and drops everything from the cut on"
    );
    assert_eq!(
        ids,
        batch_ids(2),
        "the survivors are exactly the first batch"
    );

    // One byte short of the journal: only the incomplete *last* batch is dropped.
    let (rows, ids) = cut_and_read(&copy, &full, usize::try_from(settled - 1).expect("fits"));
    assert_eq!(rows, 4, "the incomplete last batch is dropped");
    assert_eq!(
        ids,
        batch_ids(4),
        "the survivors are exactly the first two batches"
    );

    let source = Store::open(&live).expect("the source store is untouched by the copy's damage");
    assert_eq!(
        source.stats().expect("source stats").observations,
        source_rows
    );
}

#[test]
fn a_copy_with_a_torn_byte_inside_a_row_never_yields_an_invented_row() {
    // The other half of the racing-copy hazard: a frame whose bytes are complete but not the bytes
    // the writer computed. Fjall checksums every batch (`journal/batch_reader.rs:124-127`), so a torn
    // value costs the frame it sits in and everything after it - never a row the source never had.
    // The tear is aimed at a *value*: putting one in fjall's entry header trips its own debug
    // assertion instead of exercising recovery (reported in `docs/FJALL_BACKUP.md`, §3.5).
    let dir = tempfile::tempdir().expect("a temporary drill directory");
    let live = dir.path().join("live");
    let copy = dir.path().join("copy");
    let (source_rows, _) = batch_store(&live, 3);
    let source = school_rows(&Store::open(&live).expect("opening the source store"));

    copy_tree(&live, &copy);
    let journal = journal_file(&copy);
    let full = fs::read(&journal)
        .unwrap_or_else(|error| panic!("reading {} failed: {error}", journal.display()));
    // Row values are stored as uncompressed JSON, so a row's own name locates bytes that belong to
    // the second batch's frame rather than to a header.
    let needle = b"Batch School 2";
    let start = full
        .windows(needle.len())
        .position(|window| window == needle)
        .unwrap_or_else(|| panic!("the second batch's name is not stored verbatim"));

    let mut refused = 0_usize;
    let mut opened = 0_usize;
    let mut changed: Vec<(usize, String)> = Vec::new();
    for position in start..start.saturating_add(needle.len()) {
        let mut bytes = full.clone();
        let torn = bytes
            .get_mut(position)
            .unwrap_or_else(|| panic!("{} is shorter than {position}", journal.display()));
        *torn ^= 0xff;
        fs::write(&journal, &bytes)
            .unwrap_or_else(|error| panic!("writing {} failed: {error}", journal.display()));
        match Store::open(&copy) {
            Err(_) => refused = refused.saturating_add(1),
            Ok(store) => {
                opened = opened.saturating_add(1);
                for (id, row) in &school_rows(&store) {
                    if source.get(id) != Some(row) {
                        changed.push((position, id.clone()));
                    }
                }
            }
        }
    }
    eprintln!(
        "row-tear sweep over {} bytes: {refused} refused the open, {opened} opened with every row intact",
        needle.len()
    );
    assert_eq!(source_rows, 6, "three batches of two observations");
    assert_eq!(
        refused.saturating_add(opened),
        needle.len(),
        "every tear must be accounted for"
    );
    assert!(
        refused > 0,
        "a torn row value must be detected, not silently accepted"
    );
    assert!(
        changed.is_empty(),
        "a torn byte must never change or invent a row: {changed:?}"
    );
}

/// A pre-Fjall store: `<root>/entities/schools.jsonl` holds one school under two observations, and
/// `<root>/journal/drill_phase.jsonl` holds one finished unit of work. `Store::open` imports both
/// once and records the markers in the `meta` keyspace.
fn write_legacy_journals(root: &Path) {
    let (mut school, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Legacy High School",
        "legacy high school",
    );
    school.evidence.push(observation(SOURCE, FIRST_DATE));
    let mut later = school.clone();
    later.city = Some("Legacy City".to_string());
    later.evidence = vec![observation(SECOND_SOURCE, SECOND_DATE)];
    let rows = format!(
        "{}\n{}\n",
        serde_json::to_string(&school).expect("a school serializes"),
        serde_json::to_string(&later).expect("a school serializes")
    );
    let entities = root.join("entities");
    let journal = root.join("journal");
    fs::create_dir_all(&entities).expect("creating the legacy entities directory");
    fs::create_dir_all(&journal).expect("creating the legacy journal directory");
    fs::write(entities.join("schools.jsonl"), rows).expect("writing the legacy entity log");
    fs::write(
        journal.join("drill_phase.jsonl"),
        format!(
            "{}\n",
            serde_json::json!({"key": "wi:legacy", "at": FIRST_DATE, "payload": {"schools": 2}})
        ),
    )
    .expect("writing the legacy resume journal");
}
