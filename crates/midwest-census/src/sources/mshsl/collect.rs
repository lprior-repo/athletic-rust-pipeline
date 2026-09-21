mod run;

use crate::net::FetchOptions;
use crate::sources::{AdapterContext, AdapterReport};
use anyhow::Result;
use census_domain::model::{CanonicalCoach, SchoolId};

use self::run::MshslRun;
use super::map::coach_entities;
use super::teams::{parse_coach_records, parse_team_nodes, select_team_nodes, TeamCoaches};
use super::{Options, COACH_API_PREFIX, SOURCE_ID, TEAMS_VIEW_URL};

/// Fetch one school's team nodes and their coach records.
///
/// Returns the entities plus notes for surfaces that were unavailable; the caller has already written the
/// AD rows, so a failure here never discards work.
async fn collect_team_coaches(
    ctx: &AdapterContext<'_>,
    fetch_options: &FetchOptions,
    school_key: &str,
    school_id: &SchoolId,
    domains: &[String],
    observed_on: &str,
) -> (Vec<CanonicalCoach>, Vec<String>) {
    let teams_url = format!(
        "{TEAMS_VIEW_URL}?views-argument%5B%5D={school_key}&fields%5Bnode--participant%5D=title,path,drupal_internal__nid"
    );
    let outcome = match ctx.fetcher.get(&teams_url, fetch_options).await {
        Ok(outcome) => outcome,
        Err(error) => {
            return (
                Vec::new(),
                vec![format!("team list {teams_url}: {error:#}")],
            )
        }
    };
    let selected = select_team_nodes(&parse_team_nodes(&outcome.text()));
    let mut teams: Vec<TeamCoaches> = Vec::with_capacity(selected.len());
    let mut notes: Vec<String> = Vec::new();
    for node in selected {
        let api_url = format!("{COACH_API_PREFIX}{}", node.nid);
        match ctx.fetcher.get(&api_url, fetch_options).await {
            Ok(outcome) => teams.push(TeamCoaches {
                node,
                api_url,
                records: parse_coach_records(&outcome.text()),
            }),
            Err(error) => notes.push(format!("coach list {api_url}: {error:#}")),
        }
    }
    (
        coach_entities(&teams, school_id, domains, observed_on),
        notes,
    )
}

/// Fetch options for this run: the run-level refresh flag or the adapter's own.
fn fetch_options(ctx: &AdapterContext<'_>, options: &Options) -> FetchOptions {
    FetchOptions {
        refresh: options.refresh || ctx.refresh,
        allow_not_found: false,
        headers: Vec::new(),
    }
}

/// `u64` view of a `usize` count: lossless on every supported target, saturating otherwise.
fn count(value: usize) -> u64 {
    // `clippy::manual_unwrap_or` (a `-D warnings` error) requires this over a `match`, and the
    // fallback is unreachable on every supported target.
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// Collect this provider's schools and coach/AD contacts into the canonical store.
///
/// Per school: one school page (facts + AD rows), one team-node list and up to
/// `MAX_TEAMS_PER_SCHOOL` coach requests. Progress is journalled per school, so a re-run resumes
/// without re-fetching finished schools.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> Result<AdapterReport> {
    let stats_before = ctx.fetcher.stats().await;
    let Some(mut run) = MshslRun::start(ctx, options)? else {
        let mut report = AdapterReport::new(SOURCE_ID, "schools");
        report.note(
            "MSHSL covers Minnesota only; requested states do not include MN, so nothing was fetched",
        );
        return Ok(report);
    };
    run.walk().await?;
    Ok(run.finish(stats_before).await)
}
