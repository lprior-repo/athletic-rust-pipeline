use anyhow::{bail, ensure, Context, Result};
use census_crawl::{hytek, milesplit, plain_names, raceday, wiaa_results};
use census_domain::model::SourceRef;
use std::fs;
use std::path::{Path, PathBuf};

const RACEDAY_ARCHIVE_YEAR: i16 = 2023;
const ROSTER_TEAM_ID: &str = "52649";
const ARCHIVE_DIR: &str = "wiaa_results";

type CaseParse = Box<dyn Fn(&str) -> Result<usize>>;

pub struct Case {
    id: &'static str,
    file: &'static str,
    body: String,
    rows: usize,
    parse: CaseParse,
}

impl Case {
    pub fn id(&self) -> &'static str {
        self.id
    }

    pub fn file(&self) -> &'static str {
        self.file
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn parse(&self) -> Result<usize> {
        (self.parse)(&self.body)
    }
}

pub struct Corpus {
    cases: Vec<Case>,
    archive: Vec<(String, String)>,
}

impl Corpus {
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

    pub fn cases(&self) -> &[Case] {
        &self.cases
    }

    pub fn artifacts(&self) -> usize {
        self.archive.len()
    }

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

fn milesplit_roster_case() -> Result<Case> {
    let team = roster_team()?;
    case(
        "milesplit_roster",
        "milesplit",
        "wi_roster_52649.html",
        move |body| {
            let parsed = milesplit::parse_roster(body, team.clone())
                .context("the graded roster page was rejected")?;
            Ok(parsed
                .roster()
                .context("the roster was quarantined")?
                .athletes
                .len())
        },
    )
}

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

fn roster_team() -> Result<milesplit::TeamRef> {
    let index = fixture("milesplit", "wi_teams_index.html")?;
    let teams = milesplit::parse_team_index(&index).context("the team index was rejected")?;
    teams
        .iter()
        .find(|team| team.id == ROSTER_TEAM_ID)
        .cloned()
        .with_context(|| format!("the team index holds no team {ROSTER_TEAM_ID}"))
}

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

fn fixtures_root() -> Result<PathBuf> {
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../census-crawl/tests/fixtures"))
}

fn fixture(dir: &str, file: &str) -> Result<String> {
    let path = fixtures_root()?.join(dir).join(file);
    fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))
}

fn extension_of(file: &str) -> String {
    Path::new(file)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_string()
}

fn source() -> SourceRef {
    SourceRef::new("wiaa_results", None)
}
