//! The committed fixtures the `parse` group measures, and the parser each one goes through.
//!
//! Bodies are read from `tests/fixtures/**` once, before any measurement, and each case replays the
//! entry point the collection loop uses for that format: the Hy-Tek line front ends
//! (`lines_from_html` for the HTML release, `lines_from_text` for the plain-text report) feeding
//! `hytek::parse`, `raceday::parse` for a RaceDay grid, `milesplit::parse_roster` for a graded
//! roster page, and `plain_names::parse_nsaa_directory` for the NSAA directory export. The roster's
//! team is looked up in the committed team index the way `milesplit::collect` looks a team up,
//! rather than hard-coded.
//!
//! A case that parses to no published rows is a coverage hole, not a corpus entry, so
//! [`Corpus::build`] refuses to hand over a corpus in which one of them published nothing — and the
//! published row count is what the bench reports throughput against.

use anyhow::{bail, ensure, Context, Result};
use census_crawl::{hytek, milesplit, plain_names, raceday, wiaa_results};
use census_domain::model::SourceRef;
use std::fs;
use std::path::{Path, PathBuf};

/// The archive year the RaceDay export is stamped with: the format publishes no date of its own, so
/// the collection loop supplies the year the artifact was archived under. `2023` is the year of the
/// committed finish list, the same value `tests/parity_wisconsin.rs` stamps.
const RACEDAY_ARCHIVE_YEAR: i16 = 2023;
/// The MileSplit team the committed roster belongs to.
const ROSTER_TEAM_ID: &str = "52649";
/// The directory the classification corpus walks.
const ARCHIVE_DIR: &str = "wiaa_results";

/// The parse a bench case replays: fixture body in, rows published out.
type CaseParse = Box<dyn Fn(&str) -> Result<usize>>;

/// One fixture and the parse the bench replays for it.
pub struct Case {
    id: &'static str,
    file: &'static str,
    body: String,
    rows: usize,
    parse: CaseParse,
}

impl Case {
    /// The bench id this case reports under.
    pub fn id(&self) -> &'static str {
        self.id
    }

    /// The fixture file this case reads, for failure messages.
    pub fn file(&self) -> &'static str {
        self.file
    }

    /// The rows the parse publishes: this case's throughput element count.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Replay the parse on the committed body.
    pub fn parse(&self) -> Result<usize> {
        (self.parse)(&self.body)
    }
}

/// The parse corpus and the classification corpus.
pub struct Corpus {
    cases: Vec<Case>,
    archive: Vec<(String, String)>,
}

impl Corpus {
    /// Read every fixture and refuse the corpus unless each case publishes rows.
    pub fn build() -> Result<Self> {
        let cases = vec![
            hytek_html_case()?,
            hytek_text_case()?,
            raceday_case()?,
            milesplit_roster_case()?,
            nsaa_directory_case()?,
        ];

        let archive = archive()?;
        let corpus = Self { cases, archive };
        ensure!(
            corpus.artifacts() > 0,
            "no artifact under tests/fixtures/{ARCHIVE_DIR}: the classification case would measure nothing"
        );
        ensure!(
            corpus.classify_artifacts() > 0,
            "every artifact under tests/fixtures/{ARCHIVE_DIR} classified as unreadable"
        );
        Ok(corpus)
    }

    /// The fixture cases, in build order.
    pub fn cases(&self) -> &[Case] {
        &self.cases
    }

    /// Artifacts in the classification corpus.
    pub fn artifacts(&self) -> usize {
        self.archive.len()
    }

    /// Classify every committed archive artifact through `wiaa_results::artifact_format`, the way
    /// `wiaa_results::collect` decides which parser owns a body. Returns how many the classifier
    /// claims: an artifact it cannot read is not an error here, but zero claimed artifacts would be,
    /// so the count is both the return value and the guard.
    pub fn classify_artifacts(&self) -> usize {
        self.archive
            .iter()
            .filter(|(extension, body)| {
                wiaa_results::artifact_format(extension, Some(body.as_str()))
                    != wiaa_results::ArtifactFormat::Unparsed
            })
            .count()
    }
}

/// Read one fixture, run its parse once, and keep the body and the row count.
fn case(
    id: &'static str,
    dir: &str,
    file: &'static str,
    parse: impl Fn(&str) -> Result<usize> + 'static,
) -> Result<Case> {
    let body = fixture(dir, file)?;
    let rows = parse(&body).with_context(|| format!("{dir}/{file}"))?;
    if rows == 0 {
        bail!("{dir}/{file} publishes no rows: the case would report a rate for nothing");
    }
    Ok(Case {
        id,
        file,
        body,
        rows,
        parse: Box::new(parse),
    })
}

/// The Hy-Tek HTML release, replayed through `lines_from_html` and `hytek::parse`.
fn hytek_html_case() -> Result<Case> {
    case(
        "hytek_html",
        "wiaa_results",
        "d1boysstateresults-sections.htm",
        |body| {
            let lines = hytek::lines_from_html(body);
            let meet = hytek::parse(&lines, source())
                .context("the Hy-Tek HTML release parsed to no meet")?;
            Ok(meet.rows_parsed)
        },
    )
}

/// The Hy-Tek plain-text report, replayed through `lines_from_text` and `hytek::parse`.
fn hytek_text_case() -> Result<Case> {
    case(
        "hytek_text",
        "wiaa_results",
        "d1boysstateresults-dash.txt",
        |body| {
            let lines = hytek::lines_from_text(body);
            let meet = hytek::parse(&lines, source())
                .context("the Hy-Tek plain-text report parsed to no meet")?;
            Ok(meet.rows_parsed)
        },
    )
}

/// The RaceDay finish list, parsed under the year the collection loop stamps it with.
fn raceday_case() -> Result<Case> {
    case(
        "raceday_html",
        "wiaa_results",
        "racinesectionalb-finish-list.htm",
        |body| {
            let meet = raceday::parse(body, source(), RACEDAY_ARCHIVE_YEAR)
                .context("the RaceDay finish list was rejected")?;
            Ok(meet.rows_parsed)
        },
    )
}

/// The graded MileSplit roster page, parsed for the team the committed index lists.
fn milesplit_roster_case() -> Result<Case> {
    let team = roster_team()?;
    case(
        "milesplit_roster",
        "milesplit",
        "wi_roster_52649.html",
        move |body| {
            let roster = milesplit::parse_roster(body, team.clone())
                .context("the graded roster page was rejected")?;
            Ok(roster.athletes.len())
        },
    )
}

/// The NSAA directory export, whose published row count is the schools it lists.
fn nsaa_directory_case() -> Result<Case> {
    case(
        "nsaa_directory",
        "plain_names",
        "nsaa_directory_export.html",
        |body| {
            let schools = plain_names::parse_nsaa_directory(body)
                .context("the NSAA directory export was rejected")?;
            Ok(schools.len())
        },
    )
}

/// The MileSplit team the committed roster belongs to, read out of the committed team index through
/// `milesplit::parse_team_index` — the same lookup `milesplit::collect` performs.
fn roster_team() -> Result<milesplit::TeamRef> {
    let index = fixture("milesplit", "wi_teams_index.html")?;
    let teams = milesplit::parse_team_index(&index).context("the team index was rejected")?;
    teams
        .iter()
        .find(|team| team.id == ROSTER_TEAM_ID)
        .cloned()
        .with_context(|| format!("the team index holds no team {ROSTER_TEAM_ID}"))
}

/// Every committed artifact under `tests/fixtures/<dir>`, sorted by file name, as
/// `(extension, body)`.
fn archive() -> Result<Vec<(String, String)>> {
    let root = fixtures_root()?.join(ARCHIVE_DIR);
    let entries = fs::read_dir(&root).with_context(|| format!("listing {}", root.display()))?;
    let mut files = Vec::new();
    for entry in entries {
        let path = entry
            .with_context(|| format!("reading an entry of {}", root.display()))?
            .path();
        if !path.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .with_context(|| format!("a file name under {} is not UTF-8", root.display()))?
            .to_string();
        files.push(name);
    }
    files.sort();
    files
        .into_iter()
        .map(|name| {
            let body = fixture(ARCHIVE_DIR, &name)?;
            Ok((extension_of(&name), body))
        })
        .collect()
}

/// `<crawl crate>/tests/fixtures`: the corpus lives with the adapters that read it.
fn fixtures_root() -> Result<PathBuf> {
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../census-crawl/tests/fixtures"))
}

/// `<crate>/tests/fixtures/<dir>/<file>`, read as UTF-8.
fn fixture(dir: &str, file: &str) -> Result<String> {
    let path = fixtures_root()?.join(dir).join(file);
    fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))
}

/// The extension `artifact_format` classifies by, without the dot.
fn extension_of(file: &str) -> String {
    Path::new(file)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_string()
}

/// The source every fixture is parsed as evidence for; the archive walk uses the same id.
fn source() -> SourceRef {
    SourceRef::new("wiaa_results", None)
}
