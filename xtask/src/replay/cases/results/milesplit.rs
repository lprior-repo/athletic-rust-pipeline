//! The MileSplit arm: five capture shapes behind one source label, told apart by their file names.
//!
//! MileSplit needs the most context of any arm here - a roster page names no team, and a `/raw` body
//! names neither its meet nor its result set - so both readers resolve the missing half out of the
//! index captures sitting in the same corpus directory. The name grammar that decides which capture
//! is which ([`roster_fixture`], [`raw_fixture`]) lives beside the arms that read it, so a captured
//! file the verb cannot place is reported as unhandled rather than replayed as the wrong shape.

use crate::replay::{ensure_rows, unmapped, Capture};
use anyhow::{Context, Result};
use midwest_census::sources::milesplit;

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
    if file.ends_with("_results.html") {
        let files = milesplit::parse_meet_result_files(body)?;
        ensure_rows(file, files.len(), "result files")?;
        return Ok(format!("meet_result_files files={}", files.len()));
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
    let listed = milesplit::parse_meet_result_files(listing)?
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
