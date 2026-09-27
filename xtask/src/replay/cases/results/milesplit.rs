use crate::replay::{ensure_rows, unmapped, Capture};
use anyhow::{ensure, Context, Result};
use census_crawl::milesplit;

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
        let parsed = milesplit::parse_roster(body, team)?;
        let roster = parsed
            .roster()
            .context("roster quarantined: no readable athletes")?;
        ensure_rows(file, roster.athletes.len(), "athletes")?;
        return Ok(format!(
            "roster site={site} team={team_id} athletes={} rejected={}",
            roster.athletes.len(),
            parsed.rejections().len()
        ));
    }
    if let Some((site, meet_id, rsid)) = raw_fixture(file) {
        return raw(capture, &site, &meet_id, &rsid);
    }
    if let Some((site, meet_id, template)) = results_fixture(file) {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResultsTemplate {
    Literal,
    Legacy,
    Inline,
}

impl ResultsTemplate {
    fn name(self) -> &'static str {
        match self {
            Self::Literal => "literal",
            Self::Legacy => "legacy",
            Self::Inline => "inline",
        }
    }
}

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
