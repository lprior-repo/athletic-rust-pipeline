//! The MileSplit arm: five capture shapes behind one source label, told apart by their file names.
//!
//! MileSplit needs the most context of any arm here - a roster page names no team, and a `/raw` body
//! names neither its meet nor its result set - so both readers resolve the missing half out of the
//! index captures sitting in the same corpus directory. The name grammar that decides which capture
//! is which ([`roster_fixture`], [`raw_fixture`]) lives beside the arms that read it, so a captured
//! file the verb cannot place is reported as unhandled rather than replayed as the wrong shape.

use crate::replay::{ensure_rows, unmapped, Capture};
use anyhow::{ensure, Context, Result};
use census_crawl::milesplit;

/// A MileSplit capture: a site's team index, a state results index, a roster (resolved through its
/// site's index), a meet's result-file page, or a `/raw` body.
pub(super) fn capture(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    if let Some(site) = file.strip_suffix("_teams_index.html") {
        let teams = milesplit::parse_team_index(body)?;
        ensure_rows(file, teams.len(), "teams")?;
        return Ok(format!("team_index site={site} teams={}", teams.len()));
    }
    if file.ends_with("_results_index.html") {
        let meets = milesplit::parse_meet_index(body)?;
        ensure_rows(file, meets.len(), "meets")?;
        return Ok(format!("meet_index meets={}", meets.len()));
    }
    if let Some((site, team_id)) = roster_fixture(file) {
        let team = index_team(&site, &team_id, capture)?;
        let roster = milesplit::parse_roster(body, team)?;
        ensure_rows(file, roster.athletes.len(), "athletes")?;
        return Ok(format!(
            "roster site={site} team={team_id} athletes={}",
            roster.athletes.len()
        ));
    }
    if let Some((site, meet_id, rsid)) = raw_fixture(file) {
        return raw(capture, &site, &meet_id, &rsid);
    }
    if let Some((site, meet_id, template)) = results_fixture(file) {
        // The page's own address is a label here, not a route: the reader is handed the capture's
        // corpus name, because a results page served from `www` has no jurisdiction host to name and
        // the rows come out of the body either way.
        let url = format!("corpus://milesplit/{file}");
        let files = milesplit::parse_meet_result_files(&url, body)?;
        ensure_rows(file, files.len(), "result files")?;
        if template != ResultsTemplate::Inline {
            return Ok(format!(
                "meet_result_files site={site} meet={meet_id} template={} files={}",
                template.name(),
                files.len()
            ));
        }
        // The inline template publishes no file list: the page is its own one result set, so its rows
        // are the page's `<pre>` block rather than a `/raw` route of their own.
        let set = files
            .first()
            .context("an inline results page publishes one result set")?;
        let set_url = set.raw_url(&url);
        ensure!(
            set_url == url,
            "{file}: an inline set is addressed by the page itself"
        );
        let page = milesplit::parse_raw(body, &set_url)?;
        ensure_rows(file, page.meet.rows_parsed, "result rows")?;
        return Ok(format!(
            "meet_result_files site={site} meet={meet_id} template=inline files={} rows={}",
            files.len(),
            page.meet.rows_parsed
        ));
    }
    unmapped("milesplit", file)
}

/// A `/raw` body: its URL is the one the meet's own results page lists for that result set, so the
/// reader is handed the URL `milesplit::tests` spells out for this same capture.
fn raw(capture: &Capture<'_>, site: &str, meet_id: &str, rsid: &str) -> Result<String> {
    let index_file = format!("{site}_results_index.html");
    let index = capture.corpus.get(&index_file).with_context(|| {
        format!("no `{index_file}` capture in the same directory to resolve the meet with")
    })?;
    let meet = milesplit::parse_meet_index(index)?
        .into_iter()
        .find(|meet| meet.meet_id == meet_id)
        .with_context(|| format!("meet {meet_id} is not in the {index_file} capture"))?;
    let list_file = format!("{site}_meet_{meet_id}_results.html");
    let listing = capture.corpus.get(&list_file).with_context(|| {
        format!("no `{list_file}` capture to list the meet's result files with")
    })?;
    let listed = milesplit::parse_meet_result_files(&meet.results_url, listing)?
        .into_iter()
        .find(|result| result.id.to_string() == rsid)
        .with_context(|| format!("result set {rsid} is not in the {list_file} capture"))?;
    let url = listed.raw_url(&meet.results_url);
    let page = milesplit::parse_raw(capture.body, &url)?;
    ensure_rows(capture.file, page.meet.rows_parsed, "result rows")?;
    Ok(format!(
        "raw meet={:?} date={} sport={:?} events={} rows={} skipped={}",
        page.meet.name,
        page.meet.date,
        page.sport,
        page.meet.events.len(),
        page.meet.rows_parsed,
        page.meet.rows_skipped
    ))
}

/// The team a roster capture belongs to, resolved out of its site's own index capture: a roster
/// page publishes no team id, exactly as `parity_national::roster_fixture` resolves it.
fn index_team(site: &str, team_id: &str, capture: &Capture<'_>) -> Result<milesplit::TeamRef> {
    let index_file = format!("{site}_teams_index.html");
    let index = capture.corpus.get(&index_file).with_context(|| {
        format!("no `{index_file}` capture in the same directory to resolve the team with")
    })?;
    milesplit::parse_team_index(index)?
        .into_iter()
        .find(|team| team.id == team_id)
        .with_context(|| format!("team {team_id} is not in the {index_file} capture"))
}

/// A roster capture's site prefix and team id: `wi_roster_52649.html` → `("wi", "52649")`.
///
/// A three-part name is the bare form; a fourth part is the slug the site publishes after the id
/// (`oh_roster_10002_mason.html`). Anything else is no roster capture at all.
fn roster_fixture(file: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = file
        .strip_suffix(".html")?
        .split('_')
        .filter(|part| !part.is_empty())
        .collect();
    match (parts.first(), parts.get(1), parts.get(2), parts.len()) {
        (Some(site), Some(&"roster"), Some(team_id), 3 | 4) => {
            Some(((*site).to_string(), (*team_id).to_string()))
        }
        _ => None,
    }
}

/// The results-page template a capture's name declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResultsTemplate {
    /// `_results.html`: the `meetResultFiles` literal.
    Literal,
    /// `_results_legacy.html`: the `<select id="ddResultsPage">` select.
    Legacy,
    /// `_results_inline.html`: no file list, the page is its own one result set.
    Inline,
}

impl ResultsTemplate {
    /// The tag the capture's name carries for this template.
    fn name(self) -> &'static str {
        match self {
            Self::Literal => "literal",
            Self::Legacy => "legacy",
            Self::Inline => "inline",
        }
    }
}

/// A captured results page's site prefix, meet id and the template its name declares:
/// `oh_meet_770621_results.html` → the literal, `dc_meet_735841_results_legacy.html` → the legacy
/// select, `dc_meet_764735_results_inline.html` → the inline page.
///
/// The site prefix is the name the capture was filed under, which for a `www`-served page is the
/// jurisdiction its meet belongs to rather than the host that answered.
fn results_fixture(file: &str) -> Option<(String, String, ResultsTemplate)> {
    let stem = file.strip_suffix(".html")?;
    let (stem, template) = if let Some(stem) = stem.strip_suffix("_results") {
        (stem, ResultsTemplate::Literal)
    } else if let Some(stem) = stem.strip_suffix("_results_legacy") {
        (stem, ResultsTemplate::Legacy)
    } else {
        (
            stem.strip_suffix("_results_inline")?,
            ResultsTemplate::Inline,
        )
    };
    let (site, meet_id) = stem.split_once("_meet_")?;
    (!meet_id.is_empty() && meet_id.chars().all(|digit| digit.is_ascii_digit()))
        .then(|| (site.to_string(), meet_id.to_string(), template))
}

/// A `/raw` capture's site prefix, meet id and result-set id:
/// `oh_meet_770621_rs1321880_raw.html` → `("oh", "770621", "1321880")`.
fn raw_fixture(file: &str) -> Option<(String, String, String)> {
    let parts: Vec<&str> = file
        .strip_suffix("_raw.html")?
        .split('_')
        .filter(|part| !part.is_empty())
        .collect();
    match (
        parts.first(),
        parts.get(1),
        parts.get(2),
        parts.get(3),
        parts.len(),
    ) {
        (Some(site), Some(&"meet"), Some(meet_id), Some(rsid), 4) => Some((
            (*site).to_string(),
            (*meet_id).to_string(),
            rsid.strip_prefix("rs")?.to_string(),
        )),
        _ => None,
    }
}
