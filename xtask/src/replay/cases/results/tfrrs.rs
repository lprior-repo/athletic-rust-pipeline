//! The TFRRS arm: an instance home page, a performance-list page, or a team page with its roster.
//!
//! The home page is the one capture here the production walk does not read through a published
//! entry point - the adapter's own bare regex helpers are private - so [`home_routes`] performs the
//! split `sources/tfrrs/tests.rs` performs inline over this same capture, and then resolves each
//! href through the published `parse_team_path`. A page that stops publishing the `/teams/tf/`
//! family therefore fails here by name rather than reporting an empty route list.

use crate::replay::{ensure_rows, unmapped, Capture};
use anyhow::{bail, Result};
use census_crawl::tfrrs;
use std::collections::BTreeSet;

/// A TFRRS capture: a performance-list page, a team page with its roster, or the instance home page.
pub(super) fn capture(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    if file.ends_with("_home_teams.html") {
        return home_routes(capture);
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
fn home_routes(capture: &Capture<'_>) -> Result<String> {
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
