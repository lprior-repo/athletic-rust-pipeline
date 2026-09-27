use crate::replay::{ensure_rows, unmapped, Capture};
use anyhow::{bail, Result};
use census_crawl::tfrrs;
use std::collections::BTreeSet;

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
