#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet,
    CanonicalPerformance, CanonicalSchool, CanonicalTeam, CoachRole, CompetitionLevel,
    EventIdentity, EventKind, EventSpecification, Evidence, ExactSeconds, Gender, GradYear, Grade,
    Id, Mark, ObservedGrade, SchoolId, SchoolYear, SourceIdentity, SourceNamespace, SourceRef,
    Sport, TeamId, TimingMethod,
};
use census_domain::UsJurisdiction;
use census_report::bests;
use census_report::export::ExportDataset;
use census_report::report::{self, Census, Derivation, Scope};
use census_service::census;
use census_store::{Store, StoreError, StoreStats, Table};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const SOURCE: &str = "wiaa_schools";
const SECOND_SOURCE: &str = "mshsl_schools";
const FIRST_DATE: &str = "2026-09-01";
const SECOND_DATE: &str = "2026-09-02";
const THIRD_DATE: &str = "2026-09-05";
const AFTER_RESTORE_DATE: &str = "2026-09-08";
const MEET_DATE: &str = "2026-05-02";
const HISTORY_EXTRA_OBSERVATIONS: usize = 2;

fn observation(source: &str, observed_on: &str) -> Evidence {
    Evidence::parsed(SourceRef::id(source), observed_on)
}

struct Corpus {
    schools: Vec<CanonicalSchool>,
    teams: Vec<CanonicalTeam>,
    coaches: Vec<CanonicalCoach>,
    athletes: Vec<CanonicalAthlete>,
    meets: Vec<CanonicalMeet>,
    events: Vec<CanonicalEvent>,
    performances: Vec<CanonicalPerformance>,
    distinct_events: BTreeSet<String>,
    history_school: SchoolId,
}

impl Corpus {
    fn append(&self, store: &Store) -> TestResult {
        store.append_many(Table::Schools, &self.schools)?;
        store.append_many(Table::Teams, &self.teams)?;
        store.append_many(Table::Coaches, &self.coaches)?;
        store.append_many(Table::Athletes, &self.athletes)?;
        store.append_many(Table::Meets, &self.meets)?;
        store.append_many(Table::Events, &self.events)?;
        store.append_many(Table::Performances, &self.performances)?;
        Ok(())
    }

    fn history_row(&self) -> TestResult<&CanonicalSchool> {
        self.schools
            .iter()
            .find(|row| row.id == self.history_school)
            .ok_or_else(|| "corpus carries no history school".into())
    }
}

fn corpus() -> TestResult<Corpus> {
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
            None,
        ),
    };
    let history = add_school(
        &mut corpus,
        UsJurisdiction::Wisconsin,
        "Drill High School",
        0,
    )?;
    corpus.history_school = history;
    add_school(
        &mut corpus,
        UsJurisdiction::Minnesota,
        "Drill North High School",
        1,
    )?;
    add_school(
        &mut corpus,
        UsJurisdiction::Iowa,
        "Drill West High School",
        2,
    )?;
    Ok(corpus)
}

fn add_school(
    corpus: &mut Corpus,
    jurisdiction: UsJurisdiction,
    name: &str,
    index: usize,
) -> TestResult<SchoolId> {
    let (mut school, school_id) =
        CanonicalSchool::new(jurisdiction, name, normalize_name(name), None);
    school.evidence.push(observation(SOURCE, FIRST_DATE));
    let team = CanonicalTeam {
        id: Id::mint("team", &[school_id.as_str(), "outdoor", "2026"]),
        school: school_id.clone(),
        sport: Sport::OutdoorTrack,
        gender: Gender::Mixed,
        school_year: SchoolYear::new(2025).ok_or("invalid fixture season")?,
        level: None,
        source_identities: Vec::new(),
        evidence: vec![observation(SOURCE, FIRST_DATE)],
        retained_conflicts: Vec::new(),
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
    meet.sports = vec![Sport::OutdoorTrack];
    meet.evidence.push(observation(SOURCE, MEET_DATE));
    let meet_id = meet.id.clone();
    corpus.schools.push(school);
    corpus.teams.push(team);
    corpus.coaches.push(coach);
    corpus.meets.push(meet);
    for slot in 0..2 {
        add_athlete(corpus, index, slot, &school_id, &team_id, &meet_id)?;
    }
    Ok(school_id)
}

fn add_athlete(
    corpus: &mut Corpus,
    index: usize,
    slot: usize,
    school_id: &SchoolId,
    team_id: &TeamId,
    meet_id: &census_domain::model::MeetId,
) -> TestResult {
    let gender = if (index + slot).is_multiple_of(2) {
        Gender::Boys
    } else {
        Gender::Girls
    };
    let source = SourceIdentity::new(
        SourceNamespace::Other("fixture".to_string()),
        format!("drill-athlete-{index}-{slot}"),
    );
    let mut athlete = CanonicalAthlete::new(
        school_id,
        format!("Drill Runner {index}-{slot}"),
        GradYear::CO2027,
        gender,
        source.clone(),
    );
    athlete.sports.push(Sport::OutdoorTrack);
    athlete.observed_grades.push(ObservedGrade {
        grade: Grade::new(11).ok_or("invalid fixture grade")?,
        school_year: SchoolYear::new(2025).ok_or("invalid fixture season")?,
        source: SourceRef::id(SOURCE),
    });
    athlete.evidence.push(observation(SOURCE, MEET_DATE));

    let kind = if slot.is_multiple_of(2) {
        EventKind::Track800m
    } else {
        EventKind::Track1600m
    };
    let mut event = CanonicalEvent::new(
        EventIdentity {
            meet: meet_id,
            kind,
            gender,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?;
    event.evidence.push(observation(SOURCE, MEET_DATE));
    corpus.distinct_events.insert(event.id.as_str().to_string());
    for attempt in 0..2 {
        let source_key = format!("drill-{index}-{slot}-{attempt}");
        corpus.performances.push(CanonicalPerformance {
            id: CanonicalPerformance::mint(&athlete.id, meet_id, &event.id, MEET_DATE, &source_key),
            athlete: athlete.id.clone(),
            team: team_id.clone(),
            event: event.id.clone(),
            meet: meet_id.clone(),
            date: MEET_DATE.to_string(),
            mark: Mark::TimeSeconds(ExactSeconds::parse(
                &(130.0
                    + f64::from(u32::try_from(index + slot)?)
                    + f64::from(u32::try_from(attempt)?))
                .to_string(),
            )?),
            wind_mps: None,
            place: Some(u16::try_from(attempt + 1)?),
            heat: Some(format!("attempt-{}", attempt + 1)),
            round: None,
            timing: Some(TimingMethod::Fat),
            observed_grade: Some(Grade::new(11).ok_or("invalid fixture grade")?),
            evidence: vec![observation(SOURCE, MEET_DATE)],
            source_key,
            source_athlete: Some(source.clone()),
            retained_conflicts: Vec::new(),
        });
    }
    corpus.events.push(event);
    corpus.athletes.push(athlete);
    Ok(())
}

fn add_history(store: &Store, first: &CanonicalSchool) -> TestResult {
    let mut second = first.clone();
    second.city = Some("Drill City".to_string());
    second.enrollment = Some(420);
    second.evidence = vec![observation(SECOND_SOURCE, SECOND_DATE)];
    let mut third = first.clone();
    third.co_op = true;
    third.evidence = vec![observation(SOURCE, THIRD_DATE)];
    store.append(Table::Schools, &second)?;
    store.append(Table::Schools, &third)?;
    Ok(())
}

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

fn drill(root: &Path) -> TestResult<(Corpus, PathBuf)> {
    let live = root.join("live");
    let backup = root.join("backup");
    let restored = root.join("restored");
    let corpus = corpus()?;

    {
        let store = Store::open(&live)?;
        corpus.append(&store)?;
        add_history(&store, corpus.history_row()?)?;
        store.journal_done(
            "drill_phase",
            "wi:drill",
            &serde_json::json!({"schools": 3}),
        )?;
        store.flush()?;
    }

    copy_tree(&live, &backup)?;
    check!(eq; tree_digest(&live.join("fjall"))?,
    tree_digest(&backup.join("fjall"))?,
    "the backup must be a byte image of the stopped store's database");

    copy_tree(&backup.join("fjall"), &restored.join("fjall"))?;
    check!(eq; tree_digest(&restored.join("fjall"))?,
    tree_digest(&backup.join("fjall"))?,
    "the restored database must be a byte image of the backup");

    Ok((corpus, restored))
}

fn capture(store: &Store, history_id: &SchoolId) -> TestResult<ReadModel> {
    let stats = store.stats()?;
    let schools = store.scan::<CanonicalSchool>(Table::Schools)?;
    let history = schools
        .iter()
        .find(|row| &row.id == history_id)
        .ok_or("missing merged history school")?
        .clone();
    let dataset = ExportDataset::load(store)?;
    let core = Derivation::of(&dataset, Scope::Core, None);
    let all_sources = Derivation::of(&dataset, Scope::AllSources, None);
    Ok(ReadModel {
        tables: stats.tables,
        observations: stats.observations,
        merged_schools: schools.len(),
        history,
        core: census_json(&report::build_census(&core, &store.out_dir()), store.root())?,
        all_sources: census_json(
            &report::build_census(&all_sources, &store.out_dir()),
            store.root(),
        )?,
        bests: serde_json::to_string_pretty(&bests::build_from_dataset(
            &dataset,
            &bests::Options {
                scope: Scope::Core,
                grad_year: Some(2027),
                limit: None,
            },
        ))?,
        counts: census::consolidate(store)?,
        snapshots: snapshots(&store.out_dir())?,
    })
}

fn snapshots(out: &Path) -> TestResult<Vec<(String, String)>> {
    ["schools", "coaches", "performances"]
        .iter()
        .map(|name| -> TestResult<_> {
            let path = out.join(format!("{name}.jsonl"));
            let body = fs::read_to_string(&path)?;
            Ok(((*name).to_string(), body))
        })
        .collect()
}

fn census_json(census: &Census, root: &Path) -> TestResult<String> {
    let mut value = serde_json::to_value(census)?;
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "generated_on".to_string(),
            serde_json::Value::String("<drill>".to_string()),
        );
    }
    let text = serde_json::to_string_pretty(&value)?;
    Ok(text.replace(&root.display().to_string(), "<drill>"))
}

fn assert_same_json(label: &str, left: &str, right: &str) -> TestResult {
    let left_value: serde_json::Value = serde_json::from_str(left)?;
    let right_value: serde_json::Value = serde_json::from_str(right)?;
    if let Some((path, left_at, right_at)) = first_difference(&left_value, &right_value, "") {
        return Err(format!("{label} differs at {path}: {left_at} != {right_at}").into());
    }
    Ok(())
}

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

fn census_field(census: &str, path: &[&str]) -> TestResult<u64> {
    let value: serde_json::Value = serde_json::from_str(census)?;
    let mut cursor = &value;
    for step in path {
        cursor = cursor
            .get(step)
            .ok_or_else(|| format!("census has no {} field", path.join(".")))?;
    }
    cursor
        .as_u64()
        .ok_or_else(|| format!("census field {} is not an integer", path.join(".")).into())
}

fn expected_observations(corpus: &Corpus) -> TestResult<Vec<(String, u64)>> {
    let row = |table: Table, rows: usize| -> TestResult<_> {
        let rows = u64::try_from(rows)?;
        Ok((table.file().to_string(), rows))
    };
    let evidence = [
        row(
            Table::Schools,
            corpus.schools.len() + HISTORY_EXTRA_OBSERVATIONS,
        )?,
        row(Table::Teams, corpus.teams.len())?,
        row(Table::Coaches, corpus.coaches.len())?,
        row(Table::Athletes, corpus.athletes.len())?,
        row(Table::Meets, corpus.meets.len())?,
        row(Table::Events, corpus.distinct_events.len())?,
        row(Table::Performances, corpus.performances.len())?,
    ];
    let derived = [
        Table::SourceIdentities,
        Table::Conflicts,
        Table::ReviewCases,
        Table::Coverage,
        Table::Snapshots,
        Table::SourceAccess,
        Table::IdentityVerdicts,
        Table::SourceMeets,
        Table::SourceObservations,
        Table::AthleteIdentityDecisions,
    ]
    .into_iter()
    .map(|table| row(table, 0))
    .collect::<TestResult<Vec<_>>>()?;
    let mut expected: Vec<(String, u64)> = evidence.to_vec();
    expected.extend(derived);
    Ok(expected)
}

fn assert_live_store_matches_corpus(before: &ReadModel, corpus: &Corpus) -> TestResult {
    check!(eq; before.tables,
    expected_observations(corpus)?,
    "the live store's per-table observation counts");
    check!(eq; before.merged_schools,
    corpus.schools.len(),
    "three observations of the history school merge into one row");
    check!(eq; census_field(&before.core, &["totals", "schools"])?,
    u64::try_from(corpus.schools.len())?,);
    check!(eq; census_field(&before.all_sources, &["totals", "athletes"])?,
    u64::try_from(corpus.athletes.len())?,);
    check!(eq; census_field(&before.all_sources, &["totals", "class_of_2027"])?,
    u64::try_from(corpus.athletes.len())?,);
    check!(eq; census_field(&before.all_sources, &["totals", "coaches"])?,
    u64::try_from(corpus.coaches.len())?,);
    let bests: Vec<serde_json::Value> = serde_json::from_str(&before.bests)?;
    check!(eq; bests.len(),
    corpus.athletes.len(),
    "one best mark per athlete");
    Ok(())
}

fn assert_history(school: &CanonicalSchool) -> TestResult {
    let observed: Vec<(String, String)> = school
        .evidence
        .iter()
        .map(|row| (row.source.id.clone(), row.observed_on.clone()))
        .collect();
    check!(eq; observed,
    vec![
        (SOURCE.to_string(), FIRST_DATE.to_string()),
        (SECOND_SOURCE.to_string(), SECOND_DATE.to_string()),
        (SOURCE.to_string(), THIRD_DATE.to_string()),
    ],
    "the merged history keeps all three observations in append order");
    check!(eq; school.city.as_deref(), Some("Drill City"));
    check!(eq; school.enrollment, Some(420));
    check!(school.co_op, "the third observation set co_op");
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) -> TestResult {
    fs::create_dir_all(to)?;
    let mut entries: Vec<PathBuf> = fs::read_dir(from)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<_>>()?;
    entries.sort();
    for path in entries {
        let name = path
            .file_name()
            .ok_or_else(|| format!("{} has no file name", path.display()))?;
        let target = to.join(name);
        if path.is_dir() {
            copy_tree(&path, &target)?;
        } else {
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

fn file_digests(root: &Path) -> TestResult<Vec<(String, String)>> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) -> TestResult {
        let mut entries: Vec<PathBuf> = fs::read_dir(dir)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<_>>()?;
        entries.sort();
        for path in entries {
            if path.is_dir() {
                walk(root, &path, out)?;
            } else {
                let relative = path.strip_prefix(root)?.display().to_string();
                out.push((relative, sha256_file(&path)?));
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(root, root, &mut out)?;
    out.sort();
    Ok(out)
}

fn sha256_file(path: &Path) -> TestResult<String> {
    let bytes = fs::read(path)?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

fn tree_digest(root: &Path) -> TestResult<String> {
    let mut hasher = Sha256::new();
    for (path, digest) in file_digests(root)? {
        hasher.update(path.as_bytes());
        hasher.update([0_u8]);
        hasher.update(digest.as_bytes());
        hasher.update(*b"\n");
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn table_count(stats: &StoreStats, table: Table) -> TestResult<u64> {
    stats
        .tables
        .iter()
        .find(|(name, _)| name == table.file())
        .map(|(_, count)| *count)
        .ok_or_else(|| format!("stats reported no count for {}", table.file()).into())
}

fn merged_school(store: &Store, id: &SchoolId) -> TestResult<CanonicalSchool> {
    store
        .scan::<CanonicalSchool>(Table::Schools)?
        .into_iter()
        .find(|row| &row.id == id)
        .ok_or_else(|| "missing merged history school".into())
}

fn batch_row(batch: usize, slot: usize) -> CanonicalSchool {
    let index = batch * 2 + slot;
    let name = format!("Batch School {index}");
    CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        name.clone(),
        normalize_name(&name),
        None,
    )
    .0
}

fn batch_ids(count: usize) -> BTreeSet<String> {
    (0..count)
        .map(|index| batch_row(index / 2, index % 2).id.as_str().to_string())
        .collect()
}

fn append_batch(root: &Path, batch: usize) -> TestResult {
    let store = Store::open(root)?;
    let rows: Vec<CanonicalSchool> = (0..2).map(|slot| batch_row(batch, slot)).collect();
    store.append_many(Table::Schools, &rows)?;
    store.flush()?;
    Ok(())
}

fn batch_store(root: &Path, batches: usize) -> TestResult<(u64, u64)> {
    let mut boundary = 0_u64;
    for batch in 0..batches {
        if batch == 1 {
            drop(Store::open(root)?);
            boundary = journal_len(root)?;
        }
        append_batch(root, batch)?;
    }
    let store = Store::open(root)?;
    let rows = store.stats()?.observations;
    drop(store);
    Ok((rows, boundary))
}

fn journal_len(root: &Path) -> TestResult<u64> {
    let journal = journal_file(root)?;
    Ok(fs::metadata(&journal)?.len())
}

fn cut_and_read(copy: &Path, full: &[u8], cut: usize) -> TestResult<(u64, BTreeSet<String>)> {
    let journal = journal_file(copy)?;
    let prefix = full
        .get(..cut)
        .ok_or_else(|| format!("cut {cut} exceeds journal length {}", full.len()))?;
    fs::write(&journal, prefix)?;
    let store = Store::open(copy)?;
    let rows = store.stats()?.observations;
    Ok((rows, school_ids(&store)?))
}

fn journal_file(root: &Path) -> TestResult<PathBuf> {
    let fjall = root.join("fjall");
    let mut found: Vec<PathBuf> = fs::read_dir(&fjall)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .filter(|path| path.extension().is_some_and(|extension| extension == "jnl"))
        .collect();
    found.sort();
    found
        .pop()
        .ok_or_else(|| format!("no journal under {}", fjall.display()).into())
}

fn school_ids(store: &Store) -> TestResult<BTreeSet<String>> {
    Ok(store
        .scan::<CanonicalSchool>(Table::Schools)?
        .into_iter()
        .map(|row| row.id.as_str().to_string())
        .collect())
}

fn school_rows(store: &Store) -> TestResult<BTreeMap<String, String>> {
    store
        .scan::<CanonicalSchool>(Table::Schools)?
        .into_iter()
        .map(|row| -> TestResult<_> {
            Ok((row.id.as_str().to_string(), serde_json::to_string(&row)?))
        })
        .collect()
}

#[test]
fn cold_copy_backup_restores_the_read_model_exactly() -> TestResult {
    let dir = tempfile::tempdir()?;

    let (corpus, restored_root) = drill(dir.path())?;
    let before_root = dir.path().join("live");

    let before = {
        let store = Store::open(&before_root)?;
        capture(&store, &corpus.history_school)?
    };
    let after = {
        let store = Store::open(&restored_root)?;
        capture(&store, &corpus.history_school)?
    };

    assert_live_store_matches_corpus(&before, &corpus)?;
    assert_history(&before.history)?;

    check!(eq; after.tables, before.tables,
    "restored per-table observation counts");
    check!(eq; after.observations, before.observations,
    "restored observation total");
    check!(eq; after.merged_schools, before.merged_schools,
    "restored merged school count");
    check!(eq; after.history, before.history,
    "the history school's merged observation history");
    assert_same_json("the core-scope census", &before.core, &after.core)?;
    assert_same_json(
        "the all-sources census",
        &before.all_sources,
        &after.all_sources,
    )?;
    assert_same_json("the best-mark reduction", &before.bests, &after.bests)?;
    check!(eq; after.counts, before.counts, "the consolidated row counts");
    check!(eq; after.snapshots, before.snapshots,
    "the consolidated snapshots");
    Ok(())
}

#[test]
fn the_restored_store_reopens_and_appends_without_overwriting() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (corpus, restored_root) = drill(dir.path())?;

    let reopened = Store::open(&restored_root)?;
    let restored_history = merged_school(&reopened, &corpus.history_school)?;
    assert_history(&restored_history)?;
    let before = reopened.stats()?;

    let mut fourth = restored_history.clone();
    fourth.evidence = vec![observation(SECOND_SOURCE, AFTER_RESTORE_DATE)];
    reopened.append(Table::Schools, &fourth)?;
    reopened.flush()?;
    let after = reopened.stats()?;
    check!(eq; table_count(&after, Table::Schools)?,
    table_count(&before, Table::Schools)? + 1,
    "the append is a new observation row");
    let appended = merged_school(&reopened, &corpus.history_school)?;
    check!(eq; appended.evidence.len(),
    4,
    "the appended observation joins the history instead of replacing it");
    check!(eq; appended.evidence.last().map(|row| row.observed_on.as_str()),
    Some(AFTER_RESTORE_DATE));
    drop(reopened);

    let again = Store::open(&restored_root)?;
    let final_history = merged_school(&again, &corpus.history_school)?;
    check!(eq; final_history.evidence.len(), 4);
    check!(eq; table_count(&again.stats()?, Table::Schools)?,
    table_count(&before, Table::Schools)? + 1);
    Ok(())
}

#[test]
fn a_copy_taken_while_the_store_handle_is_open_keeps_every_committed_batch() -> TestResult {
    let dir = tempfile::tempdir()?;
    let live = dir.path().join("live");
    let copy = dir.path().join("copy");
    let corpus = corpus()?;

    let store = Store::open(&live)?;
    corpus.append(&store)?;
    add_history(&store, corpus.history_row()?)?;
    let before = store.stats()?;
    copy_tree(&live.join("fjall"), &copy.join("fjall"))?;

    let restored = Store::open(&copy)?;
    let after = restored.stats()?;
    check!(eq; after.observations, before.observations);
    check!(eq; after.tables, before.tables);
    assert_history(&merged_school(&restored, &corpus.history_school)?)?;
    Ok(())
}

#[test]
fn a_second_open_of_a_live_store_is_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let live = Store::open(dir.path())?;
    let error = match Store::open(dir.path()) {
        Err(error) => error,
        Ok(_) => return Err("second open accepted while store is live".into()),
    };
    check!(
        matches!(error, StoreError::Open { .. }),
        "expected the typed open failure, got {error:?}"
    );
    drop(live);
    Store::open(dir.path())?;
    Ok(())
}

#[test]
fn a_copy_whose_journal_stops_mid_batch_keeps_the_complete_prefix() -> TestResult {
    let dir = tempfile::tempdir()?;
    let live = dir.path().join("live");
    let copy = dir.path().join("copy");
    let (source_rows, after_first_batch) = batch_store(&live, 3)?;
    check!(eq; source_rows, 6, "three batches of two observations");
    check!(
        after_first_batch > 0,
        "the first batch left no frames behind"
    );

    copy_tree(&live, &copy)?;
    let journal = journal_file(&copy)?;
    let full = fs::read(&journal)?;
    let settled = u64::try_from(full.len())?;
    check!(
        after_first_batch + 1 < settled,
        "the cut points must sit inside the {settled}-byte journal"
    );

    let (rows, ids) = cut_and_read(&copy, &full, usize::try_from(after_first_batch + 1)?)?;
    check!(eq; rows, 2,
    "the copy keeps the batches that finished and drops everything from the cut on");
    check!(eq; ids,
    batch_ids(2),
    "the survivors are exactly the first batch");

    let (rows, ids) = cut_and_read(&copy, &full, usize::try_from(settled - 1)?)?;
    check!(eq; rows, 4, "the incomplete last batch is dropped");
    check!(eq; ids,
    batch_ids(4),
    "the survivors are exactly the first two batches");

    let source = Store::open(&live)?;
    check!(eq; source.stats()?.observations,
    source_rows);
    Ok(())
}

#[test]
fn a_copy_with_a_torn_byte_inside_a_row_never_yields_an_invented_row() -> TestResult {
    let dir = tempfile::tempdir()?;
    let live = dir.path().join("live");
    let copy = dir.path().join("copy");
    let (source_rows, _) = batch_store(&live, 3)?;
    let source = school_rows(&Store::open(&live)?)?;

    copy_tree(&live, &copy)?;
    let journal = journal_file(&copy)?;
    let full = fs::read(&journal)?;
    let needle = b"Batch School 2";
    let start = full
        .windows(needle.len())
        .position(|window| window == needle)
        .ok_or("second batch name not stored verbatim")?;

    let mut refused = 0_usize;
    let mut opened = 0_usize;
    let mut changed: Vec<(usize, String)> = Vec::new();
    for position in start..start.saturating_add(needle.len()) {
        let mut bytes = full.clone();
        let torn = bytes
            .get_mut(position)
            .ok_or_else(|| format!("journal is shorter than {position}"))?;
        *torn ^= 0xff;
        fs::write(&journal, &bytes)?;
        match Store::open(&copy) {
            Err(_) => refused = refused.saturating_add(1),
            Ok(store) => {
                opened = opened.saturating_add(1);
                for (id, row) in &school_rows(&store)? {
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
    check!(eq; source_rows, 6, "three batches of two observations");
    check!(eq; refused.saturating_add(opened),
    needle.len(),
    "every tear must be accounted for");
    check!(
        refused > 0,
        "a torn row value must be detected, not silently accepted"
    );
    check!(
        changed.is_empty(),
        "a torn byte must never change or invent a row: {changed:?}"
    );
    Ok(())
}
