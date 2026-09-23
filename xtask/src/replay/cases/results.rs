//! The result-plane replay arms: result-set routes and published documents.
//!
//! These captures are documents rather than site pages - a Hy-Tek or RaceDay result file, a
//! MileSplit index, roster or `/raw` body, the AthleticLIVE meets CSV and one of its event
//! documents, Athletic.net's published JSON, a TFRRS list or team page - and the arms call the same
//! entry points the adapters' fixture tests call, so this is the same read rather than a second
//! parser beside them.
//!
//! Two inputs a result file cannot state about itself are supplied the way the harnesses supply
//! them: its format, decided from its extension and body by the adapter's own classifier, and its
//! archive year, read from the fixture's own record under `tests/golden/`.

use crate::replay::{ensure_rows, unmapped, Capture};
use anyhow::{bail, Context, Result};
use census_domain::model::SourceRef;
use midwest_census::sources::athleticlive_athletes::{AthleteHit, HitTeam};
use midwest_census::sources::athleticnet::{AllResults, EventDivisions, MeetData};
use midwest_census::sources::{athleticlive, milesplit, tfrrs, wiaa_results};
use std::collections::BTreeSet;

/// The capture date the corpus's harnesses stamp rows with (`parity_*::OBSERVED_ON`).
const OBSERVED_ON: &str = "2026-09-20";

/// The source label `athleticlive::build_meets` stamps its meets with, as its own fixtures do.
const MEETS_CSV_LABEL: &str = "athleticlive_meets_csv";

/// Replay one result-plane capture, returning the line the verb prints for it.
pub(super) fn replay(capture: &Capture<'_>) -> Result<String> {
    match capture.source {
        "wiaa_results" => wiaa_result_file(capture),
        "athleticlive_athletes" => athlete_hits(capture),
        "milesplit" => milesplit_capture(capture),
        "tfrrs" => tfrrs_capture(capture),
        "athleticlive_results" => athleticlive_event_document(capture),
        "athleticlive" => athleticlive_meets_csv(capture),
        "athleticnet" => athleticnet_document(capture),
        other => bail!("no result-plane replay arm for source `{other}`"),
    }
}

/// One AthleticLIVE athlete batch response: the recorded Elasticsearch envelope, decoded through
/// the adapter's own published row type.
///
/// The envelope unwrap (`hits.hits[]._source`) is the same two operations `batches::page_sources`
/// performs and `sources/athleticlive_athletes/tests.rs` repeats inline; the decode itself is
/// `AthleteHit`, the type the adapter's `page_hits` decodes, so this arm re-runs the adapter's read
/// rather than a second parser beside it. A capture whose envelope drifts decodes to nothing and
/// fails here by name.
///
/// Entity minting is out of reach for this fixture: `build_entities` needs each row's `MeetTarget`,
/// whose tenant, name, state and date come from the harvest CSV, and this fixture directory carries
/// none. The arm reports what the rows themselves publish.
fn athlete_hits(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    let envelope: serde_json::Value =
        serde_json::from_str(body).with_context(|| format!("{file} is not JSON"))?;
    let sources: Vec<serde_json::Value> = envelope
        .pointer("/hits/hits")
        .and_then(serde_json::Value::as_array)
        .map(|hits| {
            hits.iter()
                .map(|hit| {
                    hit.get("_source")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null)
                })
                .collect()
        })
        .unwrap_or_default();
    let hits: Vec<AthleteHit> = serde_json::from_value(serde_json::Value::Array(sources))
        .with_context(|| format!("{file}: the recorded hits do not decode"))?;
    ensure_rows(file, hits.len(), "athlete rows")?;
    let meets: BTreeSet<u64> = hits.iter().filter_map(AthleteHit::meet_id).collect();
    let athletes: BTreeSet<u64> = hits
        .iter()
        .filter_map(AthleteHit::athletic_net_athlete_id)
        .collect();
    let teams: BTreeSet<u64> = hits
        .iter()
        .filter_map(|hit| hit.t.as_ref()?.athletic_net_team_id())
        .collect();
    let cross_country = hits
        .iter()
        .filter(|hit| hit.t.as_ref().is_some_and(HitTeam::is_cross_country))
        .count();
    let grade_tokens = hits.iter().filter(|hit| hit.y.is_some()).count();
    Ok(format!(
        "athlete_hits rows={} meets={meets:?} athletic_net_athletes={} athletic_net_teams={} \
         cross_country={cross_country} grade_tokens={grade_tokens}",
        hits.len(),
        athletes.len(),
        teams.len()
    ))
}

/// One WIAA result file: the format from its extension and body, the season from its fixture
/// record, and the body through `wiaa_results::parse_result_body` - the call the crate's own tests
/// and `benches/core/fixtures.rs` make over these same captures.
fn wiaa_result_file(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    let extension = file.rsplit('.').next().unwrap_or_default();
    let format = wiaa_results::artifact_format(extension, Some(body));
    let year: i16 = capture
        .recorded("archive_year")?
        .parse()
        .context("the fixture record's `archive_year` is not a season")?;
    let source = SourceRef::new("wiaa_results", None);
    let Some(meet) = wiaa_results::parse_result_body(body.as_bytes(), format, source, year) else {
        bail!(
            "{file}: the {} reader published no meet: the body did not parse",
            format.as_str()
        );
    };
    Ok(format!(
        "{} year={year} meet={:?} date={} events={} rows={} skipped={}",
        format.as_str(),
        meet.name,
        meet.date,
        meet.events.len(),
        meet.rows_parsed,
        meet.rows_skipped
    ))
}

/// A TFRRS capture: an instance home page, a performance-list page, or a team page with its roster.
fn tfrrs_capture(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    if file.ends_with("_home_teams.html") {
        return tfrrs_home_routes(capture);
    }
    if file.contains("_list_") {
        let list = tfrrs::parse_list_page(body);
        let rows: usize = list.sections.iter().map(|section| section.rows.len()).sum();
        ensure_rows(file, rows, "list rows")?;
        let labels: Vec<&str> = list
            .sections
            .iter()
            .map(|section| section.label.as_str())
            .collect();
        return Ok(format!(
            "list sections={} rows={rows} labels={labels:?}",
            list.sections.len()
        ));
    }
    if file.ends_with("_team.html") {
        let roster = tfrrs::parse_team_page(body);
        ensure_rows(file, roster.athletes.len(), "roster athletes")?;
        return Ok(format!(
            "team_page school={:?} season={:?} athletes={}",
            roster.school,
            roster.season,
            roster.athletes.len()
        ));
    }
    unmapped("tfrrs", file)
}

/// The instance home page: the `/teams/tf/<School>_m.html` route family it publishes, each href
/// resolved through `parse_team_path` - the read `sources/tfrrs/tests.rs` makes of this same
/// capture. The page's production walk reads hrefs through a helper inside the module's private
/// `parse::html`; the split below is the one the module's own test performs inline, and the route
/// parse is the published `parse_team_path`, so a page that stops publishing the family fails here
/// by name rather than reporting an empty route list.
fn tfrrs_home_routes(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    let hrefs: Vec<&str> = body
        .split("href=\"")
        .skip(1)
        .filter_map(|piece| piece.split('"').next())
        .filter(|href| href.contains("/teams/tf/"))
        .collect();
    ensure_rows(file, hrefs.len(), "team routes")?;
    let routes: Vec<tfrrs::TeamPath> = hrefs
        .iter()
        .filter_map(|href| tfrrs::parse_team_path(href))
        .collect();
    if routes.len() != hrefs.len() {
        bail!(
            "{file}: {} of {} published team routes do not parse",
            hrefs.len().saturating_sub(routes.len()),
            hrefs.len()
        );
    }
    let slugs: BTreeSet<&str> = routes.iter().map(|path| path.slug.as_str()).collect();
    Ok(format!(
        "home_teams team_routes={} distinct_schools={}",
        routes.len(),
        slugs.len()
    ))
}

/// One AthleticLIVE event document: the reader takes the URL the document was published at, which
/// the adapter's own route builder derives from the event id the capture's file name carries.
fn athleticlive_event_document(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    let Some(event_id) = event_id(file) else {
        return unmapped("athleticlive_results", file);
    };
    let doc = athleticlive::parse_event_document(&athleticlive::event_doc_url(event_id), body)?;
    ensure_rows(file, doc.rows.len(), "result rows")?;
    Ok(format!(
        "event_doc event_id={:?} meet_id={:?} label={:?} kind={:?} rows={}",
        doc.event_id(),
        doc.meet_id(),
        doc.label(),
        doc.kind(),
        doc.rows.len()
    ))
}

/// A MileSplit capture: a site's team index, a state results index, a roster (resolved through its
/// site's index), a meet's result-file page, or a `/raw` body.
fn milesplit_capture(capture: &Capture<'_>) -> Result<String> {
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
        return milesplit_raw(capture, &site, &meet_id, &rsid);
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
fn milesplit_raw(capture: &Capture<'_>, site: &str, meet_id: &str, rsid: &str) -> Result<String> {
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

/// The AthleticLIVE meets CSV: its rows, then the canonical meets the adapter builds from them.
fn athleticlive_meets_csv(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    let rows = athleticlive::parse_meets_csv(body)?;
    ensure_rows(file, rows.len(), "meet rows")?;
    let meets = athleticlive::build_meets(&rows, OBSERVED_ON, MEETS_CSV_LABEL);
    Ok(format!(
        "meets_csv rows={} meets={}",
        rows.len(),
        meets.len()
    ))
}

/// An Athletic.net document, decoded with the adapter's own published wire types - the read
/// `athleticnet_meet_parity` makes of these captures.
fn athleticnet_document(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    if file.ends_with("_allresults.json") {
        let results: AllResults = decode(file, "results document", body)?;
        let rows: usize = results.blocks.iter().map(|block| block.results.len()).sum();
        return Ok(format!(
            "all_results blocks={} rows={} legs={} teams={}",
            results.blocks.len(),
            rows,
            results.legs.len(),
            results.teams.len()
        ));
    }
    if file.ends_with("_eventdiv.json") {
        let divisions: EventDivisions = decode(file, "divisions document", body)?;
        return Ok(format!("event_divisions events={}", divisions.events.len()));
    }
    if file.contains("_meetdata") {
        let data: MeetData = decode(file, "meet document", body)?;
        return Ok(format!(
            "meet_data id={} name={:?} date={} divisions={}",
            data.meet.id,
            data.meet.name,
            data.meet.date,
            data.divisions.len()
        ));
    }
    unmapped("athleticnet", file)
}

/// Decode one published document into its wire type; a body that is not that document is an error
/// naming the file, never an empty parse.
fn decode<T: serde::de::DeserializeOwned>(file: &str, what: &str, body: &str) -> Result<T> {
    serde_json::from_str(body).with_context(|| format!("{file}: the {what} did not decode"))
}

/// The event id a capture's file name carries (`event-doc-2150205.json` → `2150205`).
fn event_id(file: &str) -> Option<u64> {
    file.strip_suffix(".json")?.rsplit('-').next()?.parse().ok()
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
