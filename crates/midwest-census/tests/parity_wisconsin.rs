//! Golden-corpus parity harness for the Wisconsin sources (`wiaa`, `wiaa_results`, `raceday`,
//! `xc`).
//!
//! Decomposition moves code between functions and files; this harness is the proof that no
//! published parse result moves with it. Every fixture under `tests/fixtures/wiaa/` and
//! `tests/fixtures/wiaa_results/` is parsed through the adapter's public entry point — the same
//! entry point, and the same line reader, that `wiaa_results::collect` uses for a body of that
//! format — and the parsed value is serialized into `tests/golden/<source>__<case>.json`. A
//! refactor that changes a field, a default, a row order or a normalised mark fails the byte
//! comparison.
//!
//! Coverage is closed from both ends:
//!
//! * the walk is `common::fixtures(dir)`, so every committed fixture in the directory is parsed;
//!   a file the harness cannot classify fails the walk instead of being skipped;
//! * `<source>__corpus` records one digest per fixture in that directory, so *dropping* a fixture
//!   file — which would remove its per-fixture assertion — changes the aggregate golden and
//!   fails. A silently shrinking corpus cannot pass.
//!
//! The RaceDay and cross-country cases are direct: the RaceDay finish list is read through
//! `raceday::parse`, the three cross-country layouts through `xc::parse`. The repository ships no
//! cross-country file under `tests/fixtures/` (`xc`'s own tests carry their captures inline), so
//! those three captures are transcribed verbatim from `src/sources/xc.rs` into the constants below
//! and their parsed output is pinned exactly like a file-backed fixture.
//!
//! Seeding: `GOLDEN_UPDATE=1 cargo nextest run -p midwest-census --test parity_wisconsin`, then
//! re-run without the variable. The green run is the contract.

mod common;

use anyhow::{bail, ensure, Context, Result};
use census_domain::model::{
    CanonicalCoach, CanonicalSchool, EventKind, Gender, Grade, Mark, SourceRef,
};
use midwest_census::sources::result_file::{ParsedEvent, ParsedMeet, ParsedRow, RelayLeg};
use midwest_census::sources::{hytek, raceday, wiaa, wiaa_results, xc};
use serde::Serialize;

/// Date stamped into canonical evidence; the capture date the module's own tests use.
const OBSERVED_ON: &str = "2026-09-20";
/// Provider slug the archive artifacts carry, as `wiaa_results::collect` mints it.
const SOURCE_ID: &str = "wiaa_results";

// -------------------------------------------------------------------------------------------------
// Serializable mirrors of the parsed shapes
//
// The adapters' hand-off shapes (`IndexEntry`, `SchoolPage`, `ParsedMeet`) are deliberately not
// `Serialize` — they are internal values, not wire formats. The mirrors below are this harness's
// own record of a published parse, so each one carries *every* field of the production shape:
// dropping a field here would let a refactor change it unnoticed.
// -------------------------------------------------------------------------------------------------

#[derive(Serialize)]
struct IndexRow {
    org_id: String,
    name: String,
    level: String,
    city: String,
    page_url: String,
}

impl IndexRow {
    fn of(entry: &wiaa::IndexEntry) -> Self {
        Self {
            org_id: entry.org_id.clone(),
            name: entry.name.clone(),
            level: entry.level.clone(),
            city: entry.city.clone(),
            page_url: entry.page_url(),
        }
    }
}

#[derive(Serialize)]
struct StaffRowView {
    role: String,
    name: String,
    email: Option<String>,
}

#[derive(Serialize)]
struct CoachRowView {
    sport: String,
    name: String,
    role: String,
    email: Option<String>,
}

#[derive(Serialize)]
struct SchoolPageView {
    name: String,
    level: Option<String>,
    city: Option<String>,
    conference: Option<String>,
    enrollment: Option<u32>,
    website: Option<String>,
    admins: Vec<StaffRowView>,
    coaches: Vec<CoachRowView>,
}

impl SchoolPageView {
    fn of(page: &wiaa::SchoolPage) -> Self {
        Self {
            name: page.name.clone(),
            level: page.level.clone(),
            city: page.city.clone(),
            conference: page.conference.clone(),
            enrollment: page.enrollment,
            website: page.website.clone(),
            admins: page
                .admins
                .iter()
                .map(|row| StaffRowView {
                    role: row.role.clone(),
                    name: row.name.clone(),
                    email: row.email.clone(),
                })
                .collect(),
            coaches: page
                .coaches
                .iter()
                .map(|row| CoachRowView {
                    sport: row.sport.clone(),
                    name: row.name.clone(),
                    role: row.role.clone(),
                    email: row.email.clone(),
                })
                .collect(),
        }
    }
}

#[derive(Serialize)]
struct SchoolExtractView {
    school: CanonicalSchool,
    coaches: Vec<CanonicalCoach>,
    skipped_admin_roles: Vec<String>,
    skipped_coach_rows: usize,
}

impl SchoolExtractView {
    fn of(extract: &wiaa::SchoolExtract) -> Self {
        Self {
            school: extract.school.clone(),
            coaches: extract.coaches.clone(),
            skipped_admin_roles: extract.skipped_admin_roles.clone(),
            skipped_coach_rows: extract.skipped_coach_rows,
        }
    }
}

#[derive(Serialize)]
struct LegView {
    position: u8,
    name: String,
    grade: Option<Grade>,
}

impl LegView {
    fn of(leg: &RelayLeg) -> Self {
        Self {
            position: leg.position,
            name: leg.name.clone(),
            grade: leg.grade,
        }
    }
}

#[derive(Serialize)]
struct RowView {
    place: Option<u16>,
    name: String,
    grade: Option<Grade>,
    school: String,
    mark: Mark,
    wind_mps: Option<f64>,
    heat: Option<String>,
    points: Option<f64>,
    legs: Vec<LegView>,
}

impl RowView {
    fn of(row: &ParsedRow) -> Self {
        Self {
            place: row.place,
            name: row.name.clone(),
            grade: row.grade,
            school: row.school.clone(),
            mark: row.mark.clone(),
            wind_mps: row.wind_mps,
            heat: row.heat.clone(),
            points: row.points,
            legs: row.legs.iter().map(LegView::of).collect(),
        }
    }
}

#[derive(Serialize)]
struct EventView {
    label: String,
    kind: EventKind,
    gender: Gender,
    division: Option<String>,
    round: Option<String>,
    rows: Vec<RowView>,
}

impl EventView {
    fn of(event: &ParsedEvent) -> Self {
        Self {
            label: event.label.clone(),
            kind: event.kind.clone(),
            gender: event.gender,
            division: event.division.clone(),
            round: event.round.clone(),
            rows: event.rows.iter().map(RowView::of).collect(),
        }
    }
}

#[derive(Serialize)]
struct MeetView {
    name: String,
    date: String,
    end_date: Option<String>,
    timer: Option<String>,
    events: Vec<EventView>,
    rows_parsed: usize,
    rows_skipped: usize,
}

impl MeetView {
    fn of(meet: &ParsedMeet) -> Self {
        Self {
            name: meet.name.clone(),
            date: meet.date.clone(),
            end_date: meet.end_date.clone(),
            timer: meet.timer.clone(),
            events: meet.events.iter().map(EventView::of).collect(),
            rows_parsed: meet.rows_parsed,
            rows_skipped: meet.rows_skipped,
        }
    }
}

/// One `wiaa_results` artifact: how it was classified, and what the classified parser (and the
/// cross-country fallback) made of it.
#[derive(Serialize)]
struct ArtifactView {
    file: String,
    extension: String,
    format: String,
    archive_year: i16,
    meet: MeetView,
}

/// A fixture file and the digest of its parsed view.
#[derive(Serialize)]
struct CorpusEntry {
    file: String,
    digest: String,
}

/// The whole fixture directory: a dropped file changes the list and fails the golden.
#[derive(Serialize)]
struct Corpus {
    source: &'static str,
    files: Vec<CorpusEntry>,
}

// -------------------------------------------------------------------------------------------------
// Corpus facts
// -------------------------------------------------------------------------------------------------

/// Archive year each `wiaa_results` fixture was published under, from the URL the WIAA archive
/// filed it under (`/Results/Track/2025/…`, `/Results/Cross_Country/2023/…`).
///
/// The RaceDay layout publishes no date of its own, so `raceday::parse` stamps the meet with the
/// year the artifact was archived under; that year belongs to the corpus, not to the file. A
/// fixture missing from this table is a coverage hole, not a default.
const ARCHIVE_YEARS: [(&str, i16); 6] = [
    ("d1boysstateresults-dash.htm", 2025),
    ("d1boysstateresults-dash.txt", 2025),
    ("d1boysstateresults-sections.htm", 2025),
    ("racinesectionalb-finish-list.htm", 2023),
    ("seed-column-regional.htm", 2025),
    ("trackside-regional.htm", 2025),
];

fn extension_of(file: &str) -> &str {
    file.rsplit_once('.').map_or("", |(_, extension)| extension)
}

/// File name without its extension, used for golden names.
fn stem_of(file: &str) -> &str {
    file.rsplit_once('.').map_or(file, |(stem, _)| stem)
}

fn archive_year(file: &str) -> Result<i16> {
    ARCHIVE_YEARS
        .iter()
        .find(|(name, _)| *name == file)
        .map(|(_, year)| *year)
        .with_context(|| format!("no archive year recorded for {file}: add it to ARCHIVE_YEARS"))
}

fn source() -> SourceRef {
    SourceRef::new(SOURCE_ID, None)
}

/// Parse one `wiaa_results` artifact exactly the way `wiaa_results::collect` does: classify by
/// extension and body, then hand the body to the parser that owns that format.
///
/// A PDF or an unrecognised extension yields `None` here — the walk rejects a `None` result, so a
/// PDF added to this corpus fails loudly instead of being quietly streamed around.
fn dispatch_artifact(file: &str, body: &str, year: i16) -> Result<Option<ParsedMeet>> {
    match wiaa_results::artifact_format(extension_of(file), Some(body)) {
        wiaa_results::ArtifactFormat::HytekHtml => {
            Ok(hytek::parse(&hytek::lines_from_html(body), source()))
        }
        wiaa_results::ArtifactFormat::HytekText => {
            Ok(hytek::parse(&hytek::lines_from_text(body), source()))
        }
        wiaa_results::ArtifactFormat::RaceDay => raceday::parse(body, source(), year)
            .map(Some)
            .with_context(|| format!("the RaceDay parser rejected {file}")),
        wiaa_results::ArtifactFormat::Pdf | wiaa_results::ArtifactFormat::Unparsed => Ok(None),
    }
}

/// What the cross-country parser makes of a body the archive classified otherwise.
///
/// `xc` is reached only from the PDF arm of the collection loop, and each of its layouts states a
/// header the report must carry; a Track & Field artifact must therefore come back without a meet.
/// The body is split with the reader that produced this format's report lines.
fn xc_claim(file: &str, body: &str, year: i16) -> Option<MeetView> {
    let lines = if extension_of(file) == "txt" {
        hytek::lines_from_text(body)
    } else {
        hytek::lines_from_html(body)
    };
    xc::parse(&lines, source(), year).as_ref().map(MeetView::of)
}

// -------------------------------------------------------------------------------------------------
// wiaa: the school directory
// -------------------------------------------------------------------------------------------------

/// `parse_directory_letter` over every directory fixture, and `parse_school_page` +
/// `school_entities` over every school fixture.
#[test]
fn wiaa_corpus_matches_its_goldens() -> Result<()> {
    let index = index_entries()?;
    let mut files = Vec::new();
    let mut schools = 0usize;
    for path in common::fixtures("wiaa")? {
        let file = common::file_name(&path)?;
        let body = common::fixture("wiaa", &file)?;
        if file.starts_with("directory_letter_") {
            let entries: Vec<IndexRow> = wiaa::parse_directory_letter(&body)
                .iter()
                .map(IndexRow::of)
                .collect();
            ensure!(
                !entries.is_empty(),
                "{file} is a directory fixture that yielded no rows"
            );
            common::assert_golden(&format!("wiaa__{}", stem_of(&file)), &entries)?;
            files.push(CorpusEntry {
                file,
                digest: common::digest(&entries)?,
            });
            continue;
        }
        if file.starts_with("school_org") {
            let org_id = org_id_of(&file)?;
            let page = wiaa::parse_school_page(&body);
            ensure!(
                !page.name.is_empty(),
                "{file} is a school fixture whose page carries no name"
            );
            let page_view = SchoolPageView::of(&page);
            common::assert_golden(&format!("wiaa__{}", stem_of(&file)), &page_view)?;

            let entry = entry_for(org_id, &index);
            let extract = wiaa::school_entities(&entry, &page, OBSERVED_ON)
                .with_context(|| format!("{file} yields no canonical school"))?;
            let extract_view = SchoolExtractView::of(&extract);
            common::assert_golden(&format!("wiaa__{}__extract", stem_of(&file)), &extract_view)?;

            files.push(CorpusEntry {
                file,
                digest: common::digest(&(page_view, extract_view))?,
            });
            schools = schools.saturating_add(1);
            continue;
        }
        bail!("unrecognised wiaa fixture {file}: teach this harness how to parse it");
    }
    ensure!(
        schools > 0,
        "the wiaa corpus carries no school page, so nothing exercises the entity mapping"
    );
    common::assert_golden(
        "wiaa__corpus",
        &Corpus {
            source: "wiaa",
            files,
        },
    )?;
    Ok(())
}

/// The parsed per-letter directory index, keyed for the school pages.
///
/// The school-page fixtures are keyed by the index row their own letter fragment publishes, exactly
/// as the module's tests do; a school whose letter fragment is not part of the corpus falls back to
/// an entry that carries only its `orgID`.
fn index_entries() -> Result<Vec<wiaa::IndexEntry>> {
    for path in common::fixtures("wiaa")? {
        let file = common::file_name(&path)?;
        if file.starts_with("directory_letter_") {
            let body = common::fixture("wiaa", &file)?;
            return Ok(wiaa::parse_directory_letter(&body));
        }
    }
    bail!("no directory_letter_* fixture under tests/fixtures/wiaa")
}

fn entry_for(org_id: &str, index: &[wiaa::IndexEntry]) -> wiaa::IndexEntry {
    index
        .iter()
        .find(|entry| entry.org_id == org_id)
        .cloned()
        .unwrap_or_else(|| wiaa::IndexEntry {
            org_id: org_id.to_string(),
            ..wiaa::IndexEntry::default()
        })
}

/// `school_org135_gale_ettrick_trempealeau.html` → `135`.
fn org_id_of(file: &str) -> Result<&str> {
    let rest = file
        .strip_prefix("school_org")
        .with_context(|| format!("{file} does not follow school_org<orgID>_<name>.html"))?;
    let org_id = rest.split('_').next().unwrap_or_default();
    ensure!(!org_id.is_empty(), "{file} carries no orgID");
    Ok(org_id)
}

// -------------------------------------------------------------------------------------------------
// wiaa_results: the result archive
// -------------------------------------------------------------------------------------------------

/// Every artifact in the archive corpus, classified and parsed the way the collection loop does.
#[test]
fn wiaa_results_corpus_matches_its_goldens() -> Result<()> {
    let mut files = Vec::new();
    let mut formats: Vec<String> = Vec::new();
    for path in common::fixtures("wiaa_results")? {
        let file = common::file_name(&path)?;
        let body = common::fixture("wiaa_results", &file)?;
        let year = archive_year(&file)?;
        let format = wiaa_results::artifact_format(extension_of(&file), Some(&body));
        let meet = dispatch_artifact(&file, &body, year)?.with_context(|| {
            format!(
                "{file} ({}): the classified parser found no meet",
                format.as_str()
            )
        })?;
        let view = ArtifactView {
            file: file.clone(),
            extension: extension_of(&file).to_string(),
            format: format.as_str().to_string(),
            archive_year: year,
            meet: MeetView::of(&meet),
        };
        // The golden key is the whole file name, not the stem: `d1boysstateresults-dash.htm` and
        // `d1boysstateresults-dash.txt` are two artifacts of the same release read by two
        // different line readers, and a stem-keyed golden would silently keep only one of them.
        common::assert_golden(&format!("wiaa_results__{file}"), &view)?;
        formats.push(format.as_str().to_string());
        files.push(CorpusEntry {
            file,
            digest: common::digest(&view)?,
        });
    }
    // The corpus is only a parity corpus if it still covers every reader the archive needs: track
    // HTML and text through the Hy-Tek reader, cross-country through the RaceDay reader.
    for required in ["hytek_html", "hytek_text", "raceday"] {
        ensure!(
            formats.iter().any(|format| format == required),
            "the corpus no longer covers the {required} reader: {formats:?}"
        );
    }
    common::assert_golden(
        "wiaa_results__corpus",
        &Corpus {
            source: "wiaa_results",
            files,
        },
    )?;
    Ok(())
}

/// The RaceDay finish list, read directly through `raceday::parse`.
///
/// This is the module-level entry point the archive calls for a `data-display` export; the corpus
/// test above reaches the same fixture through the classifier.
#[test]
fn raceday_finish_list_matches_its_golden() -> Result<()> {
    const FILE: &str = "racinesectionalb-finish-list.htm";
    let body = common::fixture("wiaa_results", FILE)?;
    ensure!(
        wiaa_results::artifact_format("htm", Some(&body)) == wiaa_results::ArtifactFormat::RaceDay,
        "{FILE} is no longer classified as a RaceDay export"
    );
    let meet = raceday::parse(&body, source(), 2023)
        .context("the RaceDay finish-list fixture no longer parses")?;
    ensure!(
        !meet.events.is_empty(),
        "{FILE} parsed without a single race"
    );
    common::assert_golden(
        "wiaa_results__raceday__racinesectionalb-finish-list",
        &MeetView::of(&meet),
    )
}

// -------------------------------------------------------------------------------------------------
// xc: the cross-country result layouts
// -------------------------------------------------------------------------------------------------

/// The three cross-country layouts the WIAA archive publishes, read through `xc::parse`.
///
/// A capture is the report as the timer published it; `lines_from_pdf_text` is the reader the PDF
/// arm of the archive uses before handing the lines to this parser.
#[test]
fn xc_layouts_match_their_goldens() -> Result<()> {
    for (name, capture, year) in [
        ("state_blocks", XC_STATE_BLOCKS, 2025_i16),
        ("padded_grade_table", XC_PADDED_GRADE_TABLE, 2025),
        ("accurace_rule_lined", XC_ACCURACE_RULE_LINED, 2025),
    ] {
        let meet = xc::parse(&hytek::lines_from_pdf_text(capture), source(), year)
            .with_context(|| format!("the {name} capture yielded no cross-country meet"))?;
        ensure!(
            !meet.events.is_empty(),
            "the {name} capture parsed without a single race"
        );
        common::assert_golden(&format!("wiaa_results__xc__{name}"), &MeetView::of(&meet))?;
    }
    Ok(())
}

/// The cross-country parser declines every Track & Field artifact in the archive corpus.
///
/// `xc` is the third layout of the PDF fallback chain, so a report it wrongly claims would be
/// parsed as a race. The corpus golden records `xc: null` per artifact; this test states the claim
/// once, with the fixture list, so a regression names the file it happened on.
#[test]
fn xc_declines_every_archive_fixture() -> Result<()> {
    for path in common::fixtures("wiaa_results")? {
        let file = common::file_name(&path)?;
        let body = common::fixture("wiaa_results", &file)?;
        let year = archive_year(&file)?;
        ensure!(
            xc_claim(&file, &body, year).is_none(),
            "{file} is a Track & Field report the cross-country parser now claims"
        );
    }
    Ok(())
}

// -------------------------------------------------------------------------------------------------
// Cross-country captures, transcribed verbatim from `src/sources/xc.rs`
//
// The repository has no cross-country result file under `tests/fixtures/`: the parser's own tests
// carry these three captures inline. They are reproduced byte for byte so the goldens pin what the
// parser publishes for the real layouts, and so a decomposition that moves the layouts around is
// measured against the same input.
// -------------------------------------------------------------------------------------------------

/// State meet: team score blocks that print each scorer's place, grade and time.
const XC_STATE_BLOCKS: &str = r#"
11/1/25, 1:38 PM                                                     WIAA State Cross Country Championships
                                                     WIAA State Cross Country Championships
                                                  The Ridges Golf Course, Wisconsin Rapids, WI
                                                                  11/1/2025
                                                      ========== BOYS TEAM SCORE ==========
                                                                  Division 1
    1.    69 SPASH                             (16:09.3 80:46.1 0:43.4)
  ===============================================
    1      6 Cooper Erickson                 12 15:50.2     5     28 Bennett Story               12   16:33.6
    2      9 Garrett Strong                  10 15:59.9     6   ( 32) Alex Dziak                 11   16:41.3
    3     10 Fisher Carroll                  9    16:03.1   7   ( 58) Donald Voetberg            12   17:06.6
"#;

/// Sectional: a padded table whose header carries a grade column.
const XC_PADDED_GRADE_TABLE: &str = r#"
WIAA D3 Sectional @ Sheboygan Lutheran
Overall Results
Place   Points   Bib   Name                        School                        Gender   Grade   Time      Pace
Boys Varsity
1       1        574   Wyatt See                   Poynette                      M        12      16:44.1   5:23
2       2        546   Nicholas Schubert           Ozaukee                       M        11      16:55.5   5:26
3       3        621   Eddy Giebler                Sheboygan Area Lutheran       M        10      17:00.7   5:28
4       4        573   Paceler Moll                Poynette                      M        10      17:18.1   5:34
"#;

/// AccuRace: columns stated by a `====` rule line rather than by a labelled header.
const XC_ACCURACE_RULE_LINED: &str = r#"
                           WIAA Division 3 Sectional Championship Meet
                   Baertschi & Keepers Property - Hosted by Albany High School
                                        Albany, Wisconsin
                                        October 25, 2025
                           Results provided by AccuRace Timing Services
                                      www.accuracetiming.com
                                  **** Boys' 5000 Meter Run ****
      Team Team                                                                         Avg   State
Place Pts Place Bib#    Name                  Gr   Team                         Time    Mile Qual
===== ==== ===== ====   ===================== ==   ============================ ======= ===== =====
    1    1 1/7 8223     Jonathan Simon        10   St. Ambrose/Abundant Life    16:21.6 5:16 t
    2    2 1/7 8156     Will Rzentkowski      11   Madison Country Day          16:45.7 5:24 t
    3    3 2/7 8222     David Simon           11   St. Ambrose/Abundant Life    16:55.4 5:27 t
"#;
