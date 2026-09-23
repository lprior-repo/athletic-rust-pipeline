//! Golden-corpus parity harness for the whole publishing pipeline.
//!
//! One test drives the stages `midwest-census run` chains — `consolidate` → `report` (both scopes) →
//! `bests` (class of 2027) → `workbook` — over a corpus built from committed fixtures, and pins the
//! published bytes: both report JSONs, both per-state CSVs, the best-mark JSONL and its CSV
//! sidecar, the adapter's own run report, and the workbook's sheets, parts, cell count and every
//! non-empty cell (as one digest). It then rebuilds the corpus and the store — the directory is
//! deleted and re-opened — and requires the same values again, so a decomposition refactor that
//! moves code between functions or files cannot change a single published byte without failing here.
//!
//! Two published values legitimately differ between runs and are normalised before comparison:
//!
//! * `store_dir` — and the note and workbook cells that quote it — is the store's own path, which
//!   is a tempdir here and an operator's directory in production.
//! * `generated_on` is [`census_crawl::net::today_iso`], the wall clock.
//!
//! Everything else — every count, bucket, ordering, mark, CSV field and cell — is compared exactly:
//! the workbook's raw bytes, part CRC by part CRC, are required to be identical outside
//! `docProps/core.xml`, which `rust_xlsxwriter` stamps with the file's creation time.
//!
//! One further caveat, in the adapter's favour rather than the pipeline's: `wiaa_results::collect`
//! debug-prints three `HashMap`-backed tallies into its notes, whose key order varies per run, so
//! [`report_projection`] counts those and compares the rest of the report exactly.
//!
//! # Corpus
//!
//! | fixtures | read through | contributes |
//! |---|---|---|
//! | `wiaa_results/` (6) | the adapter's own `collect`, over a seeded HTTP cache | WI meets, events, teams, athletes, performances |
//! | `wiaa/` (4) | `parse_directory_letter` + `parse_school_page` + `school_entities` | WI association schools with coaches |
//! | `ohsaa/` (9) | `parse_search` + `parse_sports_table` + `parse_ad_page` + `school_entities` | OH schools with coaches |
//! | `milesplit/` (2) | `parse_team_index` + `parse_roster` + `roster_entities` | the Abbotsford roster and its graded athletes |
//! | `athleticlive/` (1) | `parse_meets_csv` + `build_meets` | IL meets (non-core evidence) |
//! | `athleticlive_athletes/` (1) | `meet_targets` + `build_entities` | KS schools, teams and graded athletes (non-core evidence) |
//!
//! Every file in those directories is an input or an asserted control, and every adapter is read
//! through its own public entry point — the result-file adapter through the `collect` it shares with
//! a real run, because it has no public entity-level function. Its two inputs are seeded into the
//! fetcher's cache: the four archive pages it enumerates (the archive listing is authored here,
//! since only the *result* files are committed) and the six committed result files, under the
//! archive URL each one was captured from. The adapter is then required to answer every request from
//! cache (`requests == 0`) and to parse all six artifacts, which is what makes the archive listing
//! and the seeds self-checking rather than decorative.
//!
//! The result files name their schools the way a timer types them, so the corpus mints a canonical
//! school for every published label, requires the adapter's own [`SchoolIndex`] to resolve each one,
//! and then requires the store to hold exactly the ids the adapter's key rules imply. No clock, no
//! randomness and no socket enters the corpus: every value is a literal or comes out of a fixture
//! through a public parser.
//!
//! # Seeding
//!
//! `GOLDEN_UPDATE=1 cargo nextest run -p midwest-census --test parity_pipeline` writes the golden
//! files; inspect the diff and commit it. Every later run is a byte comparison.

mod common;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ops::Range;
use std::path::Path;
use std::time::Duration;

use anyhow::{bail, ensure, Context, Result};
use calamine::{open_workbook_auto, Data, Reader};
use census_crawl::net::Fetcher;
use census_crawl::result_file::ParsedMeet;
use census_crawl::{
    athleticlive, athleticlive_athletes, hytek, milesplit, ohsaa, raceday, wiaa, wiaa_results,
    AdapterContext, AdapterReport,
};
use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet,
    CanonicalPerformance, CanonicalSchool, CanonicalTeam, CompetitionLevel, EventKind, Evidence,
    GradYear, Grade, SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use midwest_census::bests::{self, BestResult, Measure};
use midwest_census::report::{self, Census, Scope};
use midwest_census::{census, workbook};
use sha2::{Digest, Sha256};

/// Capture date the committed fixtures carry (the WIAA directory letter was taken 2026-09-19/20).
const OBSERVED_ON: &str = "2026-09-20";
/// School year the MileSplit roster capture belongs to (2026-27).
///
/// `new` is const, so an out-of-range season is a compile error (E0080) rather than a test-time
/// panic.
const SCHOOL_YEAR: SchoolYear = SchoolYear::new(2026).expect("2026 is a season");
/// The cohort the best-mark reduction and the workbook are published for.
const COHORT: i16 = 2027;
const COHORT_LABEL: &str = "co2027";
/// Adapter id the AthleticLIVE harvest is recorded under; its evidence is non-core.
const SOURCE_ATHLETICLIVE_MEETS: &str = "athleticlive_meets_csv";
/// The one xlsx part whose bytes may move between two writes: `rust_xlsxwriter` stamps the
/// workbook's creation time into it, so it is excluded from the byte-for-byte comparison rather
/// than asserted.
const CORE_PART: &str = "docProps/core.xml";

// -------------------------------------------------------------------------------------------------
// The test
// -------------------------------------------------------------------------------------------------

/// The whole chain, twice: the second run rebuilds the corpus and the store from scratch — the
/// directory is deleted and reopened — so equal output is a statement about the pipeline rather than
/// about one store instance.
///
/// The two runs share the store's *path* so their published bytes are comparable directly: the
/// workbook and the census both name the store directory they were built from, and comparing two
/// different directories would mean normalizing that path away in the middle of a zip part.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pipeline_publishes_the_same_bytes_from_a_rebuilt_store() -> Result<()> {
    let scratch = tempfile::tempdir().context("temp dir for the runs")?;
    let root = scratch.path().join("store");
    let first = run_pipeline(&root).await?;
    // The first run's store is closed by the time it returns, so the directory can go.
    std::fs::remove_dir_all(&root)
        .with_context(|| format!("clearing {} between runs", root.display()))?;
    let second = run_pipeline(&root).await?;

    // Published text, compared byte for byte. The CSV and JSONL goldens hold the published text of
    // that name rather than a re-serialization of it: a field rename or a reordered column is a
    // change to the artifact, not to a data structure.
    common::assert_golden("pipeline__consolidate-counts", &first.counts)?;
    common::assert_golden_json("pipeline__report-core", &first.report_core)?;
    common::assert_golden_json("pipeline__report-all_sources", &first.report_all_sources)?;
    common::assert_golden_json(
        "pipeline__census-by-state-core.csv",
        &first.census_by_state_core,
    )?;
    common::assert_golden_json(
        "pipeline__census-by-state-all_sources.csv",
        &first.census_by_state_all_sources,
    )?;
    common::assert_golden_json("pipeline__best-results-co2027.jsonl", &first.bests_jsonl)?;
    common::assert_golden_json("pipeline__best-results-co2027.csv", &first.bests_csv)?;
    // The workbook's shape: its sheets, its parts, how many cells it carries and what they say.
    common::assert_golden("pipeline__workbook-shape", &first.workbook.shape)?;
    // The adapter's own report is published evidence too, and it is what proves the cache answered.
    common::assert_golden(
        "pipeline__wiaa-results-report",
        &report_projection(&first.results_report),
    )?;

    // Every published value must survive rebuilding the corpus and the store.
    ensure!(
        first.report_text == second.report_text,
        "the census JSON changed when the store was rebuilt"
    );
    ensure!(
        first.census_by_state_core == second.census_by_state_core
            && first.census_by_state_all_sources == second.census_by_state_all_sources,
        "a per-state CSV changed when the store was rebuilt"
    );
    ensure!(
        first.bests_jsonl == second.bests_jsonl && first.bests_csv == second.bests_csv,
        "a best-mark sidecar changed when the store was rebuilt"
    );
    ensure!(
        first.counts == second.counts,
        "the consolidate counts changed when the store was rebuilt"
    );
    ensure!(
        report_projection(&first.results_report) == report_projection(&second.results_report),
        "the result-file adapter's report changed when the store was rebuilt"
    );

    // The workbook, at the byte level: every part identical once the one part that carries the
    // file's creation time is set aside. The container's raw size is deliberately not asserted:
    // `docProps/core.xml` holds `rust_xlsxwriter`'s creation stamp, whose text moves with the
    // wall clock, so the deflated size of an unchanged workbook drifts by a few bytes across runs
    // (observed 21293..21297). The per-part CRC map below is the stronger claim: it is identical
    // only if every part's uncompressed bytes are.
    anyhow::ensure!(
        first.workbook.parts == second.workbook.parts,
        "an xlsx part changed when the store was rebuilt — left={:?} right={:?}",
        &first.workbook.parts,
        &second.workbook.parts
    );
    ensure!(
        part_bytes(&first.workbook.bytes, CORE_PART)?
            == part_bytes(&second.workbook.bytes, CORE_PART)?,
        "the two workbooks differ outside {}, the one part rust_xlsxwriter stamps with the file's \
         creation time",
        CORE_PART
    );
    ensure!(
        first.workbook.shape == second.workbook.shape,
        "the workbook's sheets or cells changed when the store was rebuilt"
    );
    Ok(())
}

// -------------------------------------------------------------------------------------------------
// The pipeline
// -------------------------------------------------------------------------------------------------

/// The adapter's notes that are debug-printed maps, and whose key order therefore varies per run.
///
/// `wiaa_results::collect` ends with `report.note(format!("formats parsed: {:?}", map))` and the
/// same for `seasons` and `pdf layouts`, where the map is a `HashMap`: two runs of one unchanged
/// binary disagree. Observed, verbatim, across the two runs of this test:
///
/// ```text
/// A formats parsed: [("raceday", 1), ("hytek_html", 4), ("hytek_text", 1)]
/// B formats parsed: [("hytek_html", 4), ("hytek_text", 1), ("raceday", 1)]
/// A seasons parsed: [(2025, 5), (2023, 1)]
/// B seasons parsed: [(2023, 1), (2025, 5)]
/// ```
///
/// Every number the adapter reports is stable — the runs agree on the per-archive notes, on the
/// artifact tallies and on `rows`, `requests`, `from_cache` and `errors` — so this harness compares
/// and goldens those, counts the excluded notes (so dropping one is still a diff) and leaves making
/// them deterministic to the adapter, where it belongs. It is reported as a residual finding.
const MAP_ORDERED_NOTE_PREFIXES: [&str; 3] =
    ["pdf layouts: ", "formats parsed: ", "seasons parsed: "];

/// The adapter's report as published evidence: its scalars and its stable notes, with the
/// map-ordered notes counted rather than compared.
fn report_projection(report: &AdapterReport) -> serde_json::Value {
    let mut notes: Vec<&String> = report
        .notes
        .iter()
        .filter(|note| {
            !MAP_ORDERED_NOTE_PREFIXES
                .iter()
                .any(|prefix| note.starts_with(prefix))
        })
        .collect();
    notes.sort();
    serde_json::json!({
        "adapter": report.adapter,
        "unit": report.unit,
        "rows": report.rows,
        "requests": report.requests,
        "from_cache": report.from_cache,
        "errors": report.errors,
        "with_email": report.with_email,
        "map_ordered_notes": report.notes.len().saturating_sub(notes.len()),
        "notes": notes,
    })
}

/// Everything one pipeline run publishes.
struct Run {
    counts: Vec<(String, usize)>,
    /// The census JSON exactly as written, both scopes concatenated.
    report_text: String,
    /// The same two documents with the run-specific values normalized, for the goldens.
    report_core: String,
    report_all_sources: String,
    census_by_state_core: String,
    census_by_state_all_sources: String,
    bests_jsonl: String,
    bests_csv: String,
    results_report: AdapterReport,
    workbook: Workbook,
}

/// Build the corpus and a store under `root`, then run every publishing stage in the order
/// `run_cycle` chains them.
async fn run_pipeline(root: &Path) -> Result<Run> {
    let store =
        Store::open(root).with_context(|| format!("opening a store at {}", root.display()))?;
    let fetcher = Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::ZERO,
        HashMap::new(),
        Vec::new(),
    )
    .context("building a fetcher over the store's own cache")?;
    let context = AdapterContext {
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: SCHOOL_YEAR,
        observed_on: OBSERVED_ON.to_string(),
    };

    // The fixture-derived corpus, on its way into the store.
    let corpus = Corpus::build()?;
    corpus.append(&store)?;

    // The result-file adapter reads the *consolidated* school snapshot, so it runs after the first
    // consolidate — the same two-phase order a real run uses.
    let first_counts = census::consolidate(&store)?;
    assert_counts(&first_counts, &corpus.expected_counts(false))?;
    let results_report = collect_wiaa_results(&store, &fetcher, &context, &corpus).await?;
    assert_result_entities(&store, &corpus)?;

    let counts = {
        let mut counts = census::consolidate(&store)?;
        assert_counts(&counts, &corpus.expected_counts(true))?;
        counts.extend(corpus.merge_report());
        counts
    };

    // Both census scopes, written and read back.
    let core = report::build_census(&store, Scope::Core)?;
    let all_sources = report::build_census(&store, Scope::AllSources)?;
    assert_scope_split(&store, &core, &all_sources)?;
    let (core_json, core_csv) = report::write_census(&store, &core, Scope::Core)?;
    let (all_json, all_csv) = report::write_census(&store, &all_sources, Scope::AllSources)?;
    let core_text = read_file(&core_json)?;
    let all_text = read_file(&all_json)?;
    // The writer publishes the census it was handed, byte for byte, not a re-derived document.
    ensure!(
        core_text == serde_json::to_string_pretty(&core)?,
        "{} is not the census that was handed to it",
        core_json.display()
    );
    ensure!(
        all_text == serde_json::to_string_pretty(&all_sources)?,
        "{} is not the census that was handed to it",
        all_json.display()
    );

    // Class-of-2027 best marks, checked against the reduction the store's own rows imply.
    let rows = bests::build(
        &store,
        &bests::Options {
            scope: Scope::Core,
            grad_year: Some(COHORT),
            limit: None,
        },
    )?;
    assert_best_reduction(&rows, &store, Scope::Core)?;
    let (bests_jsonl, bests_csv) = bests::write(&store, &rows, COHORT_LABEL)?;
    let jsonl_text = read_file(&bests_jsonl)?;
    let csv_text = read_file(&bests_csv)?;

    // The workbook, which rewrites the same best-mark sidecars through its own code path.
    let workbook_path = root.join("parity-co2027.xlsx");
    let written = workbook::build(
        &store,
        &workbook::Options {
            grad_year: Some(COHORT),
            out: Some(workbook_path.clone()),
            limit: None,
            scope: Scope::Core,
        },
    )?;
    ensure!(
        written == workbook_path,
        "the workbook was written to {} instead of {}",
        written.display(),
        workbook_path.display()
    );
    ensure!(
        read_file(&bests_jsonl)? == jsonl_text && read_file(&bests_csv)? == csv_text,
        "the workbook rewrote the best-mark sidecars with different bytes"
    );

    Ok(Run {
        counts,
        report_text: format!("{core_text}\n{all_text}"),
        report_core: normalize_run_text(&core_text, root, &core.generated_on)?,
        report_all_sources: normalize_run_text(&all_text, root, &all_sources.generated_on)?,
        census_by_state_core: read_file(&core_csv)?,
        census_by_state_all_sources: read_file(&all_csv)?,
        bests_jsonl: jsonl_text,
        bests_csv: csv_text,
        results_report,
        workbook: Workbook::read(&workbook_path, root)?,
    })
}

/// The two values a census publishes that belong to the run rather than to the pipeline: the store's
/// own directory and the generation date.
fn normalize_run_text(text: &str, root: &Path, generated_on: &str) -> Result<String> {
    let directory = root.display().to_string();
    ensure!(
        text.contains(&directory),
        "the census no longer publishes its store directory, so this normalization is stale"
    );
    ensure!(
        text.contains(generated_on),
        "the census no longer publishes `generated_on`, so this normalization is stale"
    );
    Ok(text.replace(&directory, "<store>").replace(
        &format!("\"generated_on\": \"{generated_on}\""),
        "\"generated_on\": \"<date>\"",
    ))
}

// -------------------------------------------------------------------------------------------------
// The corpus
// -------------------------------------------------------------------------------------------------

/// Every row the fixture corpus mints, plus the facts the assertions downstream need.
#[derive(Default)]
struct Corpus {
    schools: Vec<CanonicalSchool>,
    teams: Vec<CanonicalTeam>,
    coaches: Vec<CanonicalCoach>,
    athletes: Vec<CanonicalAthlete>,
    meets: Vec<CanonicalMeet>,
    events: Vec<CanonicalEvent>,
    performances: Vec<CanonicalPerformance>,
    /// The result files the corpus is published with, in archive order.
    artifacts: Vec<ResultArtifact>,
    /// Ids the result-file adapter's own key rules imply for those artifacts.
    expected: ExpectedEntities,
}

/// One committed result fixture, with the archive identity it is served under.
struct ResultArtifact {
    /// Fixture file name, for the seed's content type.
    file: String,
    /// URL the adapter discovers it by.
    url: String,
    /// Index into [`wiaa_results::ARCHIVES`].
    page: usize,
    /// Archive year the URL carries.
    year: i16,
    /// Anchor text the archive page publishes.
    label: &'static str,
}

impl Corpus {
    fn build() -> Result<Self> {
        let mut corpus = Corpus::default();
        wiaa_result_files(&mut corpus)?;
        wiaa_directory(&mut corpus)?;
        ohsaa_schools(&mut corpus)?;
        milesplit_roster(&mut corpus)?;
        athleticlive_meets(&mut corpus)?;
        athleticlive_athletes(&mut corpus)?;
        ensure!(
            !corpus.artifacts.is_empty() && !corpus.expected.performances.is_empty(),
            "the corpus carries no result artifact or no performance"
        );
        Ok(corpus)
    }

    /// Each table's name, how many rows the corpus holds and the distinct ids among them.
    ///
    /// The two numbers differ where the same entity is described twice — the WIAA directory and the
    /// MileSplit roster both describe Abbotsford, for instance — because the store merges rows that
    /// share an id at read time.
    fn tables(&self) -> [(&'static str, usize, BTreeSet<String>); 7] {
        [
            (
                "schools",
                self.schools.len(),
                ids_of(&self.schools, |row| row.id.as_str()),
            ),
            (
                "teams",
                self.teams.len(),
                ids_of(&self.teams, |row| row.id.as_str()),
            ),
            (
                "coaches",
                self.coaches.len(),
                ids_of(&self.coaches, |row| row.id.as_str()),
            ),
            (
                "athletes",
                self.athletes.len(),
                ids_of(&self.athletes, |row| row.id.as_str()),
            ),
            (
                "meets",
                self.meets.len(),
                ids_of(&self.meets, |row| row.id.as_str()),
            ),
            (
                "events",
                self.events.len(),
                ids_of(&self.events, |row| row.id.as_str()),
            ),
            (
                "performances",
                self.performances.len(),
                ids_of(&self.performances, |row| row.id.as_str()),
            ),
        ]
    }

    /// How much the read-time merge folds away per table. It is published in the golden counts, so a
    /// change in id minting that starts or stops a merge is a visible diff rather than a silent
    /// count shift.
    fn merge_report(&self) -> Vec<(String, usize)> {
        self.tables()
            .into_iter()
            .map(|(table, rows, ids)| (format!("{table}_merged"), rows.saturating_sub(ids.len())))
            .collect()
    }

    fn append(&self, store: &Store) -> Result<()> {
        store.append_many(Table::Schools, &self.schools)?;
        store.append_many(Table::Teams, &self.teams)?;
        store.append_many(Table::Coaches, &self.coaches)?;
        store.append_many(Table::Athletes, &self.athletes)?;
        store.append_many(Table::Meets, &self.meets)?;
        store.append_many(Table::Events, &self.events)?;
        store.append_many(Table::Performances, &self.performances)?;
        Ok(())
    }

    /// The row counts the store must report, with or without the result-file adapter's rows.
    fn expected_counts(&self, with_results: bool) -> Vec<(&'static str, usize)> {
        self.tables()
            .into_iter()
            .map(|(table, _, mut ids)| {
                if with_results {
                    match table {
                        "athletes" => ids.extend(self.expected.athletes.iter().cloned()),
                        "meets" => ids.extend(self.expected.meets.iter().cloned()),
                        "events" => ids.extend(self.expected.events.iter().cloned()),
                        "teams" => ids.extend(self.expected.teams.iter().cloned()),
                        "performances" => ids.extend(self.expected.performances.iter().cloned()),
                        _ => {}
                    }
                }
                (table, ids.len())
            })
            .collect()
    }
}

fn ids_of<T>(rows: &[T], id: impl Fn(&T) -> &str) -> BTreeSet<String> {
    rows.iter().map(|row| id(row).to_string()).collect()
}

/// Ids the result-file adapter's own key rules imply, so the corpus can prove the adapter kept every
/// row it was given rather than taking its own report's word for it.
#[derive(Default, Clone)]
struct ExpectedEntities {
    meets: BTreeSet<String>,
    events: BTreeSet<String>,
    teams: BTreeSet<String>,
    athletes: BTreeSet<String>,
    performances: BTreeSet<String>,
}

// -------------------------------------------------------------------------------------------------
// Corpus part 1: the WIAA result archive (driven through the adapter's own `collect`)
// -------------------------------------------------------------------------------------------------

/// Walk `wiaa_results/`: classify and parse every committed fixture exactly as the adapter does,
/// mint a school for every label the files publish, and record the ids the adapter must produce.
///
/// Each fixture is served under the archive URL it was captured from. The three `d1boysstateresults`
/// slices are slices of one release, so they are published under their own file names rather than a
/// single URL: the adapter deduplicates artifacts by URL, and collapsing them would leave two
/// fixtures unread. The RaceDay capture's URL is the one its own module documents.
fn wiaa_result_files(corpus: &mut Corpus) -> Result<()> {
    let mut parsed: Vec<(ResultArtifact, ParsedMeet, Sport)> = Vec::new();
    let mut schools: BTreeMap<String, CanonicalSchool> = BTreeMap::new();
    let mut labels: BTreeSet<String> = BTreeSet::new();

    for path in common::fixtures("wiaa_results")? {
        let file = common::file_name(&path)?;
        let (page, year, label) = archive_identity(&file)?;
        let body = read_file(&path)?;
        let (_, extension) = file
            .rsplit_once('.')
            .with_context(|| format!("{file} carries no extension"))?;
        let sport = wiaa_results::ARCHIVES[page].1;
        let url = format!(
            "https://www.wiaawi.org/Portals/0/PDF/Results/{}/{year}/{file}",
            archive_segment(sport)
        );
        // The adapter's own source label; it is a literal there rather than a constant.
        let source = SourceRef::new("wiaa_results", Some(url.clone()));
        let format = wiaa_results::artifact_format(extension, Some(&body));
        let meet = match format {
            wiaa_results::ArtifactFormat::HytekHtml => {
                hytek::parse(&hytek::lines_from_html(&body), source)
            }
            wiaa_results::ArtifactFormat::HytekText => {
                hytek::parse(&hytek::lines_from_text(&body), source)
            }
            wiaa_results::ArtifactFormat::RaceDay => Some(
                raceday::parse(&body, source, year)
                    .with_context(|| format!("{file} is not a RaceDay report"))?,
            ),
            other => bail!(
                "{file} is classified as {}, which this harness cannot read",
                other.as_str()
            ),
        }
        .with_context(|| format!("{file} yielded no meet as {}", format.as_str()))?;
        // Per-file coverage: a fixture that parses into nothing is a hole, not a corpus entry.
        // The events are walked rather than the `rows_parsed` counter, so the claim is about where
        // the rows live rather than how many were counted.
        ensure!(
            !meet.events.is_empty(),
            "{file}: the parsed meet carries no events"
        );
        ensure!(
            meet.events.iter().any(|event| !event.rows.is_empty()),
            "{file}: the parser accepted no row in any event"
        );
        ensure!(
            meet.date.starts_with(&year.to_string()),
            "{file}: the parsed date {:?} is not in the archive year {year} this harness serves it \
             under",
            meet.date
        );
        for event in &meet.events {
            for row in &event.rows {
                if !row.school.trim().is_empty() {
                    labels.insert(row.school.trim().to_string());
                }
            }
        }
        parsed.push((
            ResultArtifact {
                file,
                url,
                page,
                year,
                label,
            },
            meet,
            sport,
        ));
    }

    // A canonical school per published label: the adapter resolves labels through `SchoolIndex`, so
    // the corpus has to carry a school the index can find, and the check below proves it does.
    for label in &labels {
        let (school, id) =
            CanonicalSchool::new(UsJurisdiction::Wisconsin, label, normalize_name(label));
        schools.insert(id.as_str().to_string(), school);
    }
    let index = SchoolIndex::from_schools(&schools.values().cloned().collect::<Vec<_>>());
    for label in &labels {
        ensure!(
            index.resolve(UsJurisdiction::Wisconsin, label).is_some(),
            "the published school label {label:?} does not resolve against a school minted from it"
        );
    }

    let mut expected = ExpectedEntities::default();
    for (artifact, meet, sport) in &parsed {
        let graded = expected_ids_for(&index, artifact, meet, *sport, &mut expected)?;
        ensure!(
            graded > 0,
            "{}: no row carried a grade, so the fixture contributes no performance",
            artifact.file
        );
    }
    corpus.schools.extend(schools.into_values());
    corpus.artifacts = parsed
        .iter()
        .map(|(artifact, ..)| ResultArtifact {
            file: artifact.file.clone(),
            url: artifact.url.clone(),
            page: artifact.page,
            year: artifact.year,
            label: artifact.label,
        })
        .collect();
    expected.absorb_into(&mut corpus.expected);
    Ok(())
}

/// The archive page, year and anchor text each committed result fixture is published under.
fn archive_identity(file: &str) -> Result<(usize, i16, &'static str)> {
    match file {
        "d1boysstateresults-dash.htm" => Ok((0, 2025, "Division 1 - Dash")),
        "d1boysstateresults-dash.txt" => Ok((0, 2025, "Division 1 - Dash (text)")),
        "d1boysstateresults-sections.htm" => Ok((0, 2025, "Division 1 - Sections")),
        "seed-column-regional.htm" => Ok((0, 2025, "Division 3 Colfax Regional")),
        "trackside-regional.htm" => Ok((1, 2025, "Division 1 Badger Regional")),
        "racinesectionalb-finish-list.htm" => Ok((2, 2023, "Division 2 Racine Sectional")),
        other => bail!("{other} is not a known wiaa_results fixture"),
    }
}

/// The URL segment each archive publishes its result files under.
fn archive_segment(sport: Sport) -> &'static str {
    match sport {
        Sport::CrossCountry => "Cross_Country",
        _ => "Track",
    }
}

/// Ids the adapter's key rules imply for one result file, appended to `expected`, returning how many
/// graded members the file publishes.
///
/// This is the corpus's own reading of the file, built from the *public* primitives the adapter uses
/// — the school index, the model's constructors and `school_year_for` — so the counts asserted after
/// `collect` are independent of the adapter's private accumulator.
fn expected_ids_for(
    index: &SchoolIndex,
    artifact: &ResultArtifact,
    meet: &ParsedMeet,
    sport: Sport,
    expected: &mut ExpectedEntities,
) -> Result<usize> {
    let expected_meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        &meet.name,
        &meet.date,
        wiaa_results::level_of(&meet.name),
    );
    expected.meets.insert(expected_meet.id.as_str().to_string());
    let school_year = wiaa_results::school_year_for(&meet.date, sport, artifact.year)
        .with_context(|| {
            format!(
                "{} (archive {}) names no school season",
                meet.date, artifact.year
            )
        })?;
    let stem = artifact
        .url
        .rsplit('/')
        .next()
        .and_then(|file| file.rsplit_once('.'))
        .map(|(stem, _)| stem)
        .context("the artifact URL carries no file stem")?;

    let mut graded = 0usize;
    for event in &meet.events {
        let expected_event = CanonicalEvent::new(
            &expected_meet.id,
            event.kind.clone(),
            event.gender,
            event.division.as_deref(),
            event.round.as_deref(),
        );
        expected
            .events
            .insert(expected_event.id.as_str().to_string());

        for (row_index, row) in event.rows.iter().enumerate() {
            let label = row.school.trim();
            if label.is_empty() {
                continue;
            }
            let Some((school_id, _)) = index.resolve(UsJurisdiction::Wisconsin, label) else {
                bail!(
                    "{}: the published school label {label:?} has no canonical school",
                    artifact.file
                );
            };
            expected.teams.insert(
                CanonicalTeam::mint(&school_id, sport, event.gender, school_year)
                    .as_str()
                    .to_string(),
            );
            // Individual rows name an athlete; relay rows name a school and list their legs.
            let members: Vec<(Option<u8>, &str, Option<Grade>)> = if row.legs.is_empty() {
                vec![(None, row.name.as_str(), row.grade)]
            } else {
                row.legs
                    .iter()
                    .map(|leg| (Some(leg.position), leg.name.as_str(), leg.grade))
                    .collect()
            };
            for (leg_position, member, grade) in members {
                let Some(grade) = grade else { continue };
                if member.trim().is_empty() {
                    continue;
                }
                graded = graded.saturating_add(1);
                let grad_year = GradYear::of(grade, school_year);
                let athlete_id =
                    CanonicalAthlete::mint(&school_id, member, grad_year, event.gender);
                expected.athletes.insert(athlete_id.as_str().to_string());
                let round = event.round.as_deref().unwrap_or("final");
                let source_key = match leg_position {
                    Some(position) => {
                        format!("{stem}:{}:{round}:{row_index}:leg{position}", event.label)
                    }
                    None => format!("{stem}:{}:{round}:{row_index}", event.label),
                };
                expected.performances.insert(
                    CanonicalPerformance::mint(
                        &athlete_id,
                        &expected_meet.id,
                        &event.kind,
                        &expected_meet.date,
                        &source_key,
                    )
                    .as_str()
                    .to_string(),
                );
            }
        }
    }
    Ok(graded)
}

impl ExpectedEntities {
    fn absorb_into(&self, target: &mut ExpectedEntities) {
        target.meets.extend(self.meets.iter().cloned());
        target.events.extend(self.events.iter().cloned());
        target.teams.extend(self.teams.iter().cloned());
        target.athletes.extend(self.athletes.iter().cloned());
        target
            .performances
            .extend(self.performances.iter().cloned());
    }
}

/// Run the result-file adapter over the seeded cache and require it to have used it.
async fn collect_wiaa_results(
    _store: &Store,
    fetcher: &Fetcher,
    context: &AdapterContext<'_>,
    corpus: &Corpus,
) -> Result<AdapterReport> {
    seed_archives(fetcher, corpus)?;
    let report = wiaa_results::collect(
        context,
        &wiaa_results::Options {
            limit: None,
            refresh: false,
            observed_on: OBSERVED_ON.to_string(),
            seasons: Vec::new(),
            states: Vec::new(),
            school_names: Vec::new(),
        },
    )
    .await
    .context("the result-file adapter failed")?;
    ensure!(
        report.requests == 0,
        "the result-file adapter made {} live requests; every URL must be answered from the seeded \
         cache",
        report.requests
    );
    ensure!(
        report.rows == u64::try_from(corpus.artifacts.len())?,
        "the adapter parsed {} of {} artifacts",
        report.rows,
        corpus.artifacts.len()
    );
    Ok(report)
}

/// Seed the four archive pages the adapter enumerates and the six result files they publish.
fn seed_archives(fetcher: &Fetcher, corpus: &Corpus) -> Result<()> {
    for (page, (url, _)) in wiaa_results::ARCHIVES.iter().enumerate() {
        let listing: Vec<&ResultArtifact> = corpus
            .artifacts
            .iter()
            .filter(|artifact| artifact.page == page)
            .collect();
        seed(fetcher.cache_dir(), url, &archive_page(url, &listing))?;
    }
    for artifact in &corpus.artifacts {
        seed(
            fetcher.cache_dir(),
            &artifact.url,
            &common::fixture("wiaa_results", &artifact.file)?,
        )?;
    }
    Ok(())
}

/// The archive listing the adapter reads: one anchor per committed result file, in the URL shape the
/// association publishes (`/Portals/0/PDF/Results/<sport>/<year>/<file>`).
fn archive_page(url: &str, artifacts: &[&ResultArtifact]) -> String {
    let mut body = format!("<h3><strong>{url}</strong></h3>\n<ul>\n");
    for artifact in artifacts {
        let href = artifact
            .url
            .strip_prefix("https://www.wiaawi.org")
            .unwrap_or(&artifact.url);
        let ResultArtifact { year, label, .. } = artifact;
        body.push_str(&format!(
            "  <li>{year} - <a href=\"{href}\">{label}</a></li>\n"
        ));
    }
    body.push_str("</ul>\n");
    body
}

/// Serve `body` for a GET of `url` out of the fetcher's own cache, so an adapter that fetches still
/// runs against committed bytes with no socket ever opening: the fetcher is cache-first and returns
/// a cached 200 before it parses the URL, checks robots or takes a host turn.
///
/// The key mirrors the cache's on-disk contract — the leading 16 bytes of SHA-256 over `method`,
/// `url` and the request body joined by `0x1f`, in hex. A drift in that key turns the fetch into a
/// live request, which the caller detects through `FetchStats::requests`.
fn seed(cache: &Path, url: &str, body: &str) -> Result<()> {
    let mut hasher = Sha256::new();
    hasher.update(b"GET");
    hasher.update([0x1f]);
    hasher.update(url.as_bytes());
    hasher.update([0x1f]);
    let key = hex_prefix(hasher)?;
    let body_path = cache.join(format!("{key}.body"));
    let meta_path = cache.join(format!("{key}.meta.json"));
    std::fs::write(&body_path, body.as_bytes())
        .with_context(|| format!("seeding {}", body_path.display()))?;
    let mut body_hasher = Sha256::new();
    body_hasher.update(body.as_bytes());
    let meta = serde_json::json!({
        "url": url,
        "method": "GET",
        "status": 200,
        "sha256": hex_prefix(body_hasher)?,
        "bytes": body.len(),
        "fetched_at": "2026-09-20T00:00:00Z",
        "content_type": if url.ends_with(".txt") { "text/plain" } else { "text/html" },
    });
    std::fs::write(&meta_path, serde_json::to_vec_pretty(&meta)?)
        .with_context(|| format!("seeding {}", meta_path.display()))?;
    Ok(())
}

/// Hex of the leading 16 bytes of a SHA-256 digest, the cache-key form `net` uses.
fn hex_prefix(hasher: Sha256) -> Result<String> {
    let digest = hasher.finalize();
    let head = digest
        .get(..16)
        .context("a sha256 digest is shorter than its 16-byte prefix")?;
    Ok(head.iter().map(|byte| format!("{byte:02x}")).collect())
}

// -------------------------------------------------------------------------------------------------
// Corpus part 2: the association and roster adapters
// -------------------------------------------------------------------------------------------------

/// Walk `wiaa/`: the directory letter is the index, each `school_org<id>_…` page is one school.
fn wiaa_directory(corpus: &mut Corpus) -> Result<()> {
    let files = common::fixtures("wiaa")?;
    let letter = files
        .iter()
        .find(|path| {
            common::file_name(path)
                .map(|name| name.starts_with("directory_"))
                .unwrap_or(false)
        })
        .context("the wiaa corpus carries no directory letter")?;
    let index = wiaa::parse_directory_letter(&read_file(letter)?);
    ensure!(
        !index.is_empty(),
        "{}: the directory letter yielded no rows",
        letter.display()
    );
    let mut pages = 0usize;
    for path in &files {
        if path == letter {
            continue;
        }
        let name = common::file_name(path)?;
        let Some(org_id) = name
            .strip_prefix("school_org")
            .and_then(|rest| rest.split('_').next())
        else {
            bail!("{name} is not a known wiaa fixture: the letter or a school page");
        };
        let page = wiaa::parse_school_page(&read_file(path)?);
        // The index row supplies the org id and page URL; a page whose row is not in the captured
        // letter falls back to the id in its own file name, exactly as that module's tests do.
        let entry = index
            .iter()
            .find(|entry| entry.org_id == org_id)
            .cloned()
            .unwrap_or_else(|| wiaa::IndexEntry {
                org_id: org_id.to_string(),
                ..wiaa::IndexEntry::default()
            });
        let extract = wiaa::school_entities(&entry, &page, OBSERVED_ON)
            .with_context(|| format!("{name}: the page yielded no school"))?;
        ensure!(
            !extract.coaches.is_empty(),
            "{name}: the page yielded no coach"
        );
        corpus.schools.push(extract.school);
        corpus.coaches.extend(extract.coaches);
        pages = pages.saturating_add(1);
    }
    ensure!(
        pages >= 3,
        "the wiaa corpus carries only {pages} school pages"
    );
    Ok(())
}

/// Walk `ohsaa/`: three searches, three sports pages and three AD pages.
///
/// The two searches that name a school become rows — Dublin Coffman with its own pages, Mentor with
/// the malformed pair (a school the directory knows and whose sport page says nothing). The
/// Centerville pair has no search capture, so its school header supplies the name, the OHSAA id and
/// the city. `search_no_results.html` is the control: an empty result set, asserted empty.
fn ohsaa_schools(corpus: &mut Corpus) -> Result<()> {
    let mut searches: BTreeMap<String, Vec<ohsaa::SearchResult>> = BTreeMap::new();
    let mut sports: BTreeMap<String, String> = BTreeMap::new();
    let mut ads: BTreeMap<String, String> = BTreeMap::new();
    for path in common::fixtures("ohsaa")? {
        let name = common::file_name(&path)?;
        let body = read_file(&path)?;
        if name.starts_with("search_") {
            searches.insert(name, ohsaa::parse_search(&body));
        } else if name.starts_with("sports_") {
            sports.insert(name, body);
        } else if name.starts_with("ad_") {
            ads.insert(name, body);
        } else {
            bail!("{name} is not a known ohsaa fixture");
        }
    }
    ensure!(
        searches.len() == 3 && sports.len() == 3 && ads.len() == 3,
        "the ohsaa corpus is nine fixtures: three searches, three sports pages, three AD pages"
    );

    // Dublin Coffman: one search row, and its own sports and AD pages. The search row and the
    // school's own header have to agree on the school, or the pair is not one school.
    let dublin = &searches["search_dublin_coffman.html"];
    ensure!(
        dublin.len() == 1,
        "the Dublin Coffman search fixture yields {} rows",
        dublin.len()
    );
    let dublin_sports = &sports["sports_dublin_coffman.html"];
    let header = ohsaa_school_from_page(dublin_sports)?;
    ensure!(
        dublin[0].name == header.name && dublin[0].ohsaa_id == header.ohsaa_id,
        "the search row {} ({}) and the school page header {} ({}) disagree",
        dublin[0].name,
        dublin[0].ohsaa_id,
        header.name,
        header.ohsaa_id
    );
    ensure!(
        ohsaa::parse_sports_table(dublin_sports)
            .iter()
            .find(|(label, ..)| label == "Cross Country")
            .is_some_and(|(_, boys, _)| boys.is_some()),
        "the Dublin Coffman sports fixture no longer publishes a boys cross-country coach"
    );
    let dublin_ad = ohsaa::parse_ad_page(&ads["ad_dublin_coffman.html"]);
    let (director, _) = dublin_ad
        .director
        .clone()
        .context("the Dublin Coffman AD fixture publishes no director")?;
    ensure!(
        !dublin_ad.office_roles.is_empty(),
        "the Dublin Coffman AD fixture publishes no office role, so nothing proves they stay out \
         of the coach list"
    );
    let extract = ohsaa::school_entities(
        &dublin[0],
        dublin_sports,
        &ads["ad_dublin_coffman.html"],
        OBSERVED_ON,
    );
    ensure!(
        !extract.coaches.is_empty(),
        "the Dublin Coffman pair yields no coach"
    );
    // The AD is published as a coach; the office roles are not. This is the row-level half of that
    // rule — the report's role counts pin the classification itself. Both sides are compared after
    // the adapter's own honorific stripping.
    let expected_director = ohsaa::strip_honorific(&director);
    ensure!(
        extract
            .coaches
            .iter()
            .any(|coach| coach.name == expected_director),
        "the Dublin Coffman director {expected_director:?} is missing from the coach rows"
    );
    for (_, role_holder) in &dublin_ad.office_roles {
        let role_holder = ohsaa::strip_honorific(role_holder);
        ensure!(
            !extract
                .coaches
                .iter()
                .any(|coach| coach.name == role_holder),
            "the office role {role_holder:?} became a coach row"
        );
    }
    corpus.schools.push(extract.school);
    corpus.coaches.extend(extract.coaches);

    // Mentor: the duplicate-row search collapses to one school, and the malformed pair proves the
    // degrade path — a school with no coach rows, not an error.
    let mentor = &searches["search_duplicate_rows.html"];
    ensure!(
        mentor.len() == 1 && !mentor[0].name.is_empty() && !mentor[0].ohsaa_id.is_empty(),
        "the duplicate-row search fixture no longer collapses to one school"
    );
    ensure!(
        ohsaa::parse_sports_table(&sports["sports_malformed.html"]).is_empty(),
        "the malformed sports fixture stopped parsing as empty"
    );
    ensure!(
        ohsaa::parse_ad_page(&ads["ad_malformed.html"])
            .director
            .is_none(),
        "the malformed AD fixture stopped parsing as empty"
    );
    let extract = ohsaa::school_entities(
        &mentor[0],
        &sports["sports_malformed.html"],
        &ads["ad_malformed.html"],
        OBSERVED_ON,
    );
    ensure!(
        extract.coaches.is_empty(),
        "a school with malformed sport and AD pages yields {} coaches",
        extract.coaches.len()
    );
    corpus.schools.push(extract.school);

    // Centerville: the sports page publishes the school's own header and address, and its
    // `Track & Field` row reads one of the two cells.
    let centerville_sports = &sports["sports_centerville.html"];
    let track = ohsaa::parse_sports_table(centerville_sports);
    let track = track
        .iter()
        .find(|(label, ..)| label == "Track & Field")
        .context("the Centerville sports fixture no longer publishes a Track & Field row")?;
    ensure!(
        !(track.1.is_none() && track.2.is_none()),
        "the Centerville Track & Field row names no coach at all"
    );
    let result = ohsaa_school_from_page(centerville_sports)?;
    // The same school's AD page carries the same header: a capture pair, not two schools.
    let ad_header = ohsaa_school_from_page(&ads["ad_centerville.html"])?;
    ensure!(
        result.name == ad_header.name && result.ohsaa_id == ad_header.ohsaa_id,
        "the Centerville sports and AD fixtures name different schools: {} ({}) against {} ({})",
        result.name,
        result.ohsaa_id,
        ad_header.name,
        ad_header.ohsaa_id
    );
    let extract = ohsaa::school_entities(
        &result,
        centerville_sports,
        &ads["ad_centerville.html"],
        OBSERVED_ON,
    );
    ensure!(
        !extract.coaches.is_empty(),
        "the Centerville pair yields no coach"
    );
    corpus.schools.push(extract.school);
    corpus.coaches.extend(extract.coaches);

    ensure!(
        searches["search_no_results.html"].is_empty(),
        "the empty search fixture no longer parses as empty"
    );
    Ok(())
}

/// The school row a school's own pages publish: `<h2>NAME (id)</h2>` plus the address line the same
/// page repeats (`Centerville, OH 45459`).
fn ohsaa_school_from_page(html: &str) -> Result<ohsaa::SearchResult> {
    let header = html
        .split_once("<h2>")
        .and_then(|(_, rest)| rest.split_once("</h2>"))
        .map(|(header, _)| header.trim())
        .context("the page carries no <h2> school header")?;
    let (name, id) = header
        .split_once(" (")
        .with_context(|| format!("the school header {header:?} carries no OHSAA id"))?;
    let id = id.strip_suffix(')').unwrap_or(id).trim();
    ensure!(
        !name.is_empty() && !id.is_empty(),
        "the school header {header:?} is incomplete"
    );
    let city = html
        .split_once(", OH ")
        .and_then(|(head, _)| head.lines().last())
        .map(str::trim)
        .filter(|city| !city.is_empty())
        .context("the page carries no city on its address line")?;
    Ok(ohsaa::SearchResult {
        name: name.to_string(),
        city: city.to_string(),
        ohsaa_id: id.to_string(),
    })
}

/// Walk `milesplit/`: the team index is the roster's own team row, and the roster is the school and
/// its graded athletes — the same call that adapter's own tests make.
fn milesplit_roster(corpus: &mut Corpus) -> Result<()> {
    let mut index_body = None;
    let mut roster: Option<(String, String)> = None;
    for path in common::fixtures("milesplit")? {
        let name = common::file_name(&path)?;
        let body = read_file(&path)?;
        if name == "wi_teams_index.html" {
            index_body = Some(body);
        } else if let Some(team_id) = name
            .strip_prefix("wi_roster_")
            .and_then(|rest| rest.strip_suffix(".html"))
        {
            roster = Some((team_id.to_string(), body));
        } else if name.starts_with("oh_") {
            // The rank-4 route captures — the OH team index, the graded OH roster and the two
            // result-set bodies. The corpus this walk builds models the WI school, its athletes and
            // its teams; the OH captures are asserted by the adapter's own tests, and folding them
            // in here would move the goldens without adding a case the WI pair does not already
            // cover. They are named, not unknown.
            continue;
        } else {
            bail!("{name} is not a known milesplit fixture");
        }
    }
    let index_body = index_body.context("the milesplit corpus carries no team index")?;
    let (team_id, roster_body) = roster.context("the milesplit corpus carries no roster")?;
    let teams = milesplit::parse_team_index(&index_body)?;
    let team = teams
        .iter()
        .find(|team| team.id == team_id)
        .with_context(|| format!("the team index does not list team {team_id}"))?
        .clone();
    let parsed = milesplit::parse_roster(&roster_body, team)?;
    ensure!(
        !parsed.athletes.is_empty(),
        "the roster fixture parses to no athletes"
    );
    let site = milesplit::Site::for_jurisdiction(UsJurisdiction::Wisconsin);
    let (school, athletes, school_teams) =
        milesplit::roster_entities(&parsed, SCHOOL_YEAR, OBSERVED_ON, &site);
    corpus.schools.push(school);
    corpus.athletes.extend(athletes);
    corpus.teams.extend(school_teams);
    Ok(())
}

/// Walk `athleticlive/`: the harvest rows become meets. They carry non-core evidence, which is what
/// the all-sources scope adds and the core scope has to do without.
fn athleticlive_meets(corpus: &mut Corpus) -> Result<()> {
    for path in common::fixtures("athleticlive")? {
        let name = common::file_name(&path)?;
        ensure!(
            name == "meets-sample.csv",
            "{name} is not a known athleticlive fixture"
        );
        let rows = athleticlive::parse_meets_csv(&read_file(&path)?)?;
        ensure!(!rows.is_empty(), "{name}: no meet rows");
        let meets = athleticlive::build_meets(&rows, OBSERVED_ON, SOURCE_ATHLETICLIVE_MEETS);
        ensure!(!meets.is_empty(), "{name}: no canonical meet");
        corpus.meets.extend(meets);
    }
    Ok(())
}

/// Walk `athleticlive_athletes/`: the athlete rows become schools, teams and graded athletes, keyed
/// by the timer meet id the adapter queries — the same call that adapter's own tests make.
fn athleticlive_athletes(corpus: &mut Corpus) -> Result<()> {
    for path in common::fixtures("athleticlive_athletes")? {
        let name = common::file_name(&path)?;
        ensure!(
            name == "athlete-list-sample.json",
            "{name} is not a known athleticlive_athletes fixture"
        );
        let document: serde_json::Value = serde_json::from_str(&read_file(&path)?)?;
        let sources: Vec<serde_json::Value> = document["hits"]["hits"]
            .as_array()
            .context("the fixture carries no `hits.hits` array")?
            .iter()
            .map(|hit| hit["_source"].clone())
            .collect();
        let hits: Vec<athleticlive_athletes::AthleteHit> =
            serde_json::from_value(serde_json::Value::Array(sources))
                .context("the fixture's hits decode as athlete rows")?;
        ensure!(!hits.is_empty(), "{name}: no athlete rows");
        let meet_ids: BTreeSet<u64> = hits.iter().filter_map(|hit| hit.meet_id()).collect();
        ensure!(
            meet_ids.len() == 1,
            "{name}: the rows name {} meets; the capture is a single meet",
            meet_ids.len()
        );
        let meet_id = *meet_ids.iter().next().context("no AthleticLIVE meet id")?;

        // The adapter queries athlete rows through the meet's timer identity, so the corpus mints
        // the meet its own tests mint and hands the same selection to `build_entities`.
        let mut meet = CanonicalMeet::new(
            Some(UsJurisdiction::Kansas),
            "Abilene Invitational",
            "2025-04-25",
            CompetitionLevel::Invitational,
        );
        meet.source_identities.push(SourceIdentity::new(
            SourceNamespace::TimerMeet {
                provider: "reddirt".to_string(),
            },
            meet_id.to_string(),
        ));
        meet.evidence.push(Evidence::parsed(
            SourceRef::new(SOURCE_ATHLETICLIVE_MEETS, None),
            OBSERVED_ON,
        ));
        let selection =
            athleticlive_athletes::meet_targets(&[meet.clone()], &[UsJurisdiction::Kansas]);
        let by_id: HashMap<u64, &athleticlive_athletes::MeetTarget> = selection
            .targets
            .iter()
            .map(|target| (target.athleticlive_meet_id, target))
            .collect();
        ensure!(
            by_id.contains_key(&meet_id),
            "the corpus holds no target for AthleticLIVE meet {meet_id}"
        );
        let entities =
            athleticlive_athletes::build_entities(&hits, &by_id, OBSERVED_ON, SCHOOL_YEAR);
        ensure!(
            entities.rows == hits.len(),
            "the adapter examined {} of {} athlete rows",
            entities.rows,
            hits.len()
        );
        ensure!(
            !entities.athletes.is_empty(),
            "{name}: the rows mint no athlete"
        );
        ensure!(
            entities.rows_with_grade > 0,
            "{name}: no row carries a grade"
        );
        corpus.meets.push(meet);
        corpus.schools.extend(entities.schools);
        corpus.teams.extend(entities.teams);
        corpus.athletes.extend(entities.athletes);
    }
    Ok(())
}

// -------------------------------------------------------------------------------------------------
// Corpus assertions
// -------------------------------------------------------------------------------------------------

/// The store must hold exactly the ids the corpus (and, once it has run, the result-file adapter)
/// implies: no row lost to a merge, no row invented.
fn assert_counts(counts: &[(String, usize)], expected: &[(&str, usize)]) -> Result<()> {
    for (table, rows) in expected {
        let observed = counts
            .iter()
            .find(|(name, _)| name == table)
            .map(|(_, count)| *count)
            .with_context(|| format!("consolidate reported no count for {table}"))?;
        ensure!(
            observed == *rows,
            "the store holds {observed} rows for {table}; the corpus implies {rows}"
        );
    }
    Ok(())
}

/// What the result-file adapter wrote must be exactly what its key rules imply for the six fixtures.
fn assert_result_entities(store: &Store, corpus: &Corpus) -> Result<()> {
    let meets: BTreeSet<String> = store
        .scan::<CanonicalMeet>(Table::Meets)?
        .iter()
        .map(|row| row.id.as_str().to_string())
        .collect();
    let events: BTreeSet<String> = store
        .scan::<CanonicalEvent>(Table::Events)?
        .iter()
        .map(|row| row.id.as_str().to_string())
        .collect();
    let teams: BTreeSet<String> = store
        .scan::<CanonicalTeam>(Table::Teams)?
        .iter()
        .map(|row| row.id.as_str().to_string())
        .collect();
    let athletes: BTreeSet<String> = store
        .scan::<CanonicalAthlete>(Table::Athletes)?
        .iter()
        .map(|row| row.id.as_str().to_string())
        .collect();
    let performances: BTreeSet<String> = store
        .scan::<CanonicalPerformance>(Table::Performances)?
        .iter()
        .map(|row| row.id.as_str().to_string())
        .collect();

    let mut expected_meets = ids_of(&corpus.meets, |row| row.id.as_str());
    expected_meets.extend(corpus.expected.meets.iter().cloned());
    let mut expected_teams = ids_of(&corpus.teams, |row| row.id.as_str());
    expected_teams.extend(corpus.expected.teams.iter().cloned());
    let mut expected_athletes = ids_of(&corpus.athletes, |row| row.id.as_str());
    expected_athletes.extend(corpus.expected.athletes.iter().cloned());

    for (table, observed, expected) in [
        ("meets", &meets, &expected_meets),
        ("events", &events, &corpus.expected.events),
        ("teams", &teams, &expected_teams),
        ("athletes", &athletes, &expected_athletes),
        ("performances", &performances, &corpus.expected.performances),
    ] {
        ensure!(
            observed == expected,
            "the store holds {} {table} rows against the {} the fixtures imply; first difference: \
             {:?}",
            observed.len(),
            expected.len(),
            observed.symmetric_difference(expected).next()
        );
    }
    Ok(())
}

/// The core scope must be reachable without the Athletic.net-derived adapters: exactly the rows
/// whose evidence is non-core only are dropped, and nothing else is.
fn assert_scope_split(store: &Store, core: &Census, all_sources: &Census) -> Result<()> {
    ensure!(
        core.scope == "core" && all_sources.scope == "all_sources",
        "the censuses report scopes {:?} and {:?}",
        core.scope,
        all_sources.scope
    );
    let mut athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    let dropped_athletes = report::retain_core(&mut athletes);
    let mut meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
    let dropped_meets = report::retain_core(&mut meets);
    ensure!(
        dropped_athletes > 0 && dropped_meets > 0,
        "the store holds no non-core-only row, so the scope split proves nothing"
    );
    ensure!(
        all_sources.totals.athletes == core.totals.athletes + dropped_athletes,
        "the core scope serves {} athletes against {} all-sources and {dropped_athletes} \
         non-core-only athletes",
        core.totals.athletes,
        all_sources.totals.athletes
    );
    ensure!(
        all_sources.meets.total == core.meets.total + dropped_meets,
        "the core scope reports {} meets against {} all-sources and {dropped_meets} non-core-only \
         meets",
        core.meets.total,
        all_sources.meets.total
    );
    ensure!(
        core.totals.athletes > 0 && core.meets.total > 0,
        "the core scope serves no athlete or no meet at all"
    );
    Ok(())
}

/// The best-mark reduction must carry one row per `(athlete, event kind)` the store holds in scope,
/// holding the extreme value — computed here from the store's own rows with the reduction's own
/// comparison primitives, not read back from `bests`.
fn assert_best_reduction(rows: &[BestResult], store: &Store, scope: Scope) -> Result<()> {
    let mut athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    let mut meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
    let mut events: Vec<CanonicalEvent> = store.scan(Table::Events)?;
    let mut performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
    if scope == Scope::Core {
        report::retain_core(&mut athletes);
        report::retain_core(&mut meets);
        report::retain_core(&mut events);
        report::retain_core(&mut performances);
    }

    let cohort: BTreeMap<&str, i16> = athletes
        .iter()
        .map(|athlete| (athlete.id.as_str(), athlete.grad_year.get()))
        .collect();
    let kinds: BTreeMap<&str, &EventKind> = events
        .iter()
        .map(|event| (event.id.as_str(), &event.kind))
        .collect();

    let mut expected: BTreeMap<(String, String), (f64, usize)> = BTreeMap::new();
    for performance in &performances {
        let Some(kind) = kinds.get(performance.event.as_str()) else {
            continue;
        };
        if cohort.get(performance.athlete.as_str()) != Some(&COHORT) || bests::is_relay(kind) {
            continue;
        }
        let Some(measure) = Measure::of(&performance.mark) else {
            continue;
        };
        let Some(value) = measure.value(&performance.mark) else {
            continue;
        };
        let entry = expected
            .entry((
                performance.athlete.as_str().to_string(),
                format!("{kind:?}"),
            ))
            .or_insert((value, 0));
        entry.1 = entry.1.saturating_add(1);
        if measure.better(value, entry.0) {
            entry.0 = value;
        }
    }
    ensure!(
        !expected.is_empty(),
        "the store holds no class-of-{COHORT} mark in scope, so the reduction proves nothing"
    );
    ensure!(
        rows.len() == expected.len(),
        "the reduction published {} rows for the {} pairs the store implies",
        rows.len(),
        expected.len()
    );
    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
    for row in rows {
        let key = (row.athlete_id.clone(), row.event.clone());
        ensure!(
            seen.insert(key.clone()),
            "the reduction published {} twice for one athlete",
            row.event
        );
        let Some((value, count)) = expected.get(&key) else {
            bail!(
                "the reduction published {} for {}, which the store does not imply",
                row.event,
                row.athlete_id
            );
        };
        ensure!(
            row.marks_in_event == *count,
            "{} rests on {} marks, the store holds {count}",
            row.event,
            row.marks_in_event
        );
        ensure!(
            row.best_value == *value,
            "{} of {} is {}, the store's best is {value}",
            row.event,
            row.athlete_id,
            row.best_value
        );
    }
    Ok(())
}

// -------------------------------------------------------------------------------------------------
// The workbook
// -------------------------------------------------------------------------------------------------

/// The written workbook: its shape (golden-comparable, run-independent) and its raw bytes (compared
/// between the two runs, which share a store path).
struct Workbook {
    bytes: Vec<u8>,
    /// CRC-32 of every xlsx part except [`CORE_PART`], keyed by part name.
    parts: BTreeMap<String, u32>,
    /// Sheets (name, cell count and a per-sheet digest), xlsx parts, the total cell count and a
    /// digest of every non-empty cell, with the run-specific cells normalized so the golden holds
    /// across machines. Per-sheet digests are what make a failure name the sheet that moved.
    shape: serde_json::Value,
}

impl Workbook {
    fn read(path: &Path, root: &Path) -> Result<Self> {
        let bytes = read_bytes(path)?;
        let mut book = open_workbook_auto(path)
            .with_context(|| format!("opening {} with calamine", path.display()))?;
        let sheet_names = book.sheet_names().to_vec();
        ensure!(!sheet_names.is_empty(), "the workbook carries no sheets");

        // Every non-empty cell, rendered `sheet!row:column=value`, grouped per sheet so a mismatch
        // names the sheet that moved rather than one opaque line.
        let mut sheets: Vec<serde_json::Value> = Vec::with_capacity(sheet_names.len());
        let mut cells: Vec<String> = Vec::new();
        for name in &sheet_names {
            let range = book
                .worksheet_range(name)
                .with_context(|| format!("reading sheet {name}"))?;
            let mut sheet_cells: Vec<String> = Vec::new();
            for (row_index, row) in range.rows().enumerate() {
                let label = row.first().map(cell_text).unwrap_or_default();
                for (column, cell) in row.iter().enumerate() {
                    let text = cell_text(cell);
                    if text.is_empty() {
                        continue;
                    }
                    sheet_cells.push(format!(
                        "{name}!{row_index}:{column}={}",
                        volatile_cell(&label, column, text, root)
                    ));
                }
            }
            ensure!(
                !sheet_cells.is_empty(),
                "the workbook's {name} sheet carries no cell"
            );
            cells.extend(sheet_cells.iter().cloned());
            sheets.push(serde_json::json!({
                "name": name,
                "cells": sheet_cells.len(),
                "digest": common::digest(&sheet_cells)?,
            }));
        }
        ensure!(
            cells.len() > 100,
            "the workbook carries only {} non-empty cells, so the digest would be vacuous",
            cells.len()
        );

        let parts = part_crcs(&bytes, path)?;
        let shape = serde_json::json!({
            "sheets": sheets,
            "parts": parts.keys().collect::<Vec<_>>(),
            "cells": cells.len(),
            "content_digest": common::digest(&cells)?,
        });
        Ok(Self {
            parts,
            shape,
            bytes,
        })
    }
}

/// One cell as text; a blank cell contributes nothing to the digest.
fn cell_text(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(value) => value.clone(),
        Data::Float(value) => format!("{value}"),
        Data::Int(value) => format!("{value}"),
        Data::Bool(value) => format!("{value}"),
        other => other.to_string(),
    }
}

/// The workbook cells that quote a run-specific value: the generation date, the store's own
/// directory, and the census note that repeats it. Everything else is the pipeline's own output.
fn volatile_cell(label: &str, column: usize, text: String, root: &Path) -> String {
    if column != 1 {
        return text;
    }
    match label {
        // Both meta sheets name the day the run wrote them; that day is the wall clock, so it is
        // replaced on both rather than pinned to the day the golden was captured.
        "Core report generated" | "Workbook generated on" => "<date>".to_string(),
        // The store root is the run's own temp path; the coverage notes quote it the way the
        // `Store` row does, so the same substitution applies to every row that can carry it.
        "Store" | "Core note" | "Note" => text.replace(&root.display().to_string(), "<store>"),
        _ => text,
    }
}

/// One part of the xlsx (zip) container, from its central-directory record.
struct Part {
    name: String,
    crc32: u32,
    /// The bytes this part occupies: local header, compressed payload and data descriptor.
    region: Range<usize>,
}

/// CRC-32 of every xlsx part except [`CORE_PART`].
fn part_crcs(bytes: &[u8], path: &Path) -> Result<BTreeMap<String, u32>> {
    let parts = zip_parts(bytes)?;
    ensure!(
        parts.iter().any(|part| part.name == CORE_PART),
        "{} carries no {CORE_PART} part",
        path.display()
    );
    ensure!(
        parts.len() >= 8,
        "{} carries only {} parts, which is not a workbook",
        path.display(),
        parts.len()
    );
    Ok(parts
        .into_iter()
        .filter(|part| part.name != CORE_PART)
        .map(|part| (part.name, part.crc32))
        .collect())
}

/// The bytes of every part except `skip`, concatenated — the byte-level half of the "two runs wrote
/// the same workbook" claim.
fn part_bytes(bytes: &[u8], skip: &str) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    for part in zip_parts(bytes)? {
        if part.name == skip {
            continue;
        }
        out.extend_from_slice(
            bytes
                .get(part.region)
                .context("a part range runs past the end of the file")?,
        );
    }
    Ok(out)
}

/// Read the zip central directory: part names, their uncompressed CRC-32, and the byte range each
/// part occupies. Streamed writers put the sizes in a data descriptor, so the central directory is
/// the only place the layout is stated.
fn zip_parts(bytes: &[u8]) -> Result<Vec<Part>> {
    const CENTRAL: u32 = 0x0201_4b50;
    let eocd =
        find_eocd(bytes).context("the file carries no zip end-of-central-directory record")?;
    let count = usize::from(le_u16(bytes, eocd.saturating_add(10))?);
    let directory = usize::try_from(le_u32(bytes, eocd.saturating_add(16))?)
        .context("the central-directory offset does not fit this platform")?;
    let mut entries: Vec<(String, u32, usize)> = Vec::with_capacity(count);
    let mut at = directory;
    for _ in 0..count {
        ensure!(
            le_u32(bytes, at)? == CENTRAL,
            "the central directory at {at} does not start a header"
        );
        let crc32 = le_u32(bytes, at.saturating_add(16))?;
        let name_len = usize::from(le_u16(bytes, at.saturating_add(28))?);
        let extra_len = usize::from(le_u16(bytes, at.saturating_add(30))?);
        let comment_len = usize::from(le_u16(bytes, at.saturating_add(32))?);
        let local = usize::try_from(le_u32(bytes, at.saturating_add(42))?)
            .context("a part offset does not fit this platform")?;
        let name_at = at.saturating_add(46);
        let name_end = name_at.saturating_add(name_len);
        let name = std::str::from_utf8(
            bytes
                .get(name_at..name_end)
                .context("a part name is out of bounds")?,
        )?
        .to_string();
        entries.push((name, crc32, local));
        at = name_end
            .saturating_add(extra_len)
            .saturating_add(comment_len);
    }

    // Parts are laid out in local-header order: each owns everything up to the next part's header,
    // and the last owns everything up to the central directory.
    let mut order: Vec<usize> = (0..entries.len()).collect();
    order.sort_by_key(|index| entries[*index].2);
    let mut parts: Vec<Part> = Vec::with_capacity(entries.len());
    for (position, index) in order.iter().enumerate() {
        let start = entries[*index].2;
        let end = order
            .get(position.saturating_add(1))
            .map_or(directory, |next| entries[*next].2);
        ensure!(
            start < end && end <= bytes.len(),
            "part {} claims the byte range {start}..{end}",
            entries[*index].0
        );
        parts.push(Part {
            name: entries[*index].0.clone(),
            crc32: entries[*index].1,
            region: start..end,
        });
    }
    parts.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(parts)
}

/// Offset of the end-of-central-directory record, whose trailing comment is bounded at 64 KiB.
fn find_eocd(bytes: &[u8]) -> Option<usize> {
    const EOCD: [u8; 4] = [0x50, 0x4b, 0x05, 0x06];
    const MAX_COMMENT: usize = 65_535;
    let floor = bytes.len().saturating_sub(MAX_COMMENT.saturating_add(22));
    bytes
        .windows(EOCD.len())
        .enumerate()
        .rev()
        .find(|(index, window)| *index >= floor && *window == EOCD.as_slice())
        .map(|(index, _)| index)
}

fn le_u16(bytes: &[u8], at: usize) -> Result<u16> {
    let end = at.checked_add(2).context("a zip offset overflowed")?;
    let field: [u8; 2] = bytes
        .get(at..end)
        .context("the zip header is truncated")?
        .try_into()
        .context("the zip header field is the wrong width")?;
    Ok(u16::from_le_bytes(field))
}

fn le_u32(bytes: &[u8], at: usize) -> Result<u32> {
    let end = at.checked_add(4).context("a zip offset overflowed")?;
    let field: [u8; 4] = bytes
        .get(at..end)
        .context("the zip header is truncated")?
        .try_into()
        .context("the zip header field is the wrong width")?;
    Ok(u32::from_le_bytes(field))
}

// -------------------------------------------------------------------------------------------------
// File helpers
// -------------------------------------------------------------------------------------------------

/// Read a fixture by path; `common::fixture` takes a source and file name, which a walk does not
/// have.
fn read_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
}

fn read_bytes(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).with_context(|| format!("reading {}", path.display()))
}
