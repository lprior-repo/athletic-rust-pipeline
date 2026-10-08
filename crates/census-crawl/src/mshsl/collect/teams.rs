use super::run::MshslRun;
use crate::directory::acquisition::{fail, owe, text};
use crate::mshsl::map::coach_entities;
use crate::mshsl::teams::{parse_team_nodes, team_sport, CoachRecord, TeamCoaches, TeamNode};
use crate::mshsl::{COACH_API_PREFIX, TEAMS_VIEW_URL};
use crate::{CrawlError, CrawlResult};
use census_domain::model::SchoolId;
use futures::{stream, StreamExt, TryStreamExt};
use serde_json::Value;

pub(super) async fn collect(
    run: &mut MshslRun<'_>,
    subject: (&str, &SchoolId),
    domains: &[String],
) -> CrawlResult<()> {
    let url = format!("{TEAMS_VIEW_URL}?views-argument%5B%5D={}&fields%5Bnode--participant%5D=title,path,drupal_internal__nid", subject.0);
    let Some(capture) = fetch(run, &url).await? else {
        return Ok(());
    };
    let body = match text(&capture) {
        Ok(body) => body,
        Err(error) => return fail(&mut run.report, &url, error),
    };
    let parsed = serde_json::from_str::<Value>(body).ok();
    let Some(rows) = parsed
        .as_ref()
        .and_then(|value| value.get("data"))
        .and_then(Value::as_array)
    else {
        return fail(&mut run.report, &url, "missing team data array");
    };
    let nodes = parse_team_nodes(body);
    if nodes.len() != rows.len() {
        owe(&mut run.report, &url)?;
    }
    stream::iter(
        nodes
            .iter()
            .filter(|node| team_sport(&node.alias).is_some()),
    )
    .map(Ok::<_, CrawlError>)
    .try_fold(run, |run, node| async move {
        team(run, node, subject.1, domains).await?;
        Ok(run)
    })
    .await
    .map(|_| ())
}

async fn fetch(run: &mut MshslRun<'_>, url: &str) -> CrawlResult<Option<crate::net::FetchOutcome>> {
    match run
        .ctx
        .fetcher
        .get(url, &super::fetch_options(run.ctx, run.options))
        .await
    {
        Ok(capture) if capture.status == 200 => Ok(Some(capture)),
        Ok(capture) => {
            fail(&mut run.report, url, format!("HTTP {}", capture.status))?;
            Ok(None)
        }
        Err(error) => {
            fail(&mut run.report, url, error)?;
            Ok(None)
        }
    }
}

async fn team(
    run: &mut MshslRun<'_>,
    node: &TeamNode,
    school_id: &SchoolId,
    domains: &[String],
) -> CrawlResult<()> {
    let url = format!("{COACH_API_PREFIX}{}", node.nid);
    let Some(capture) = fetch(run, &url).await? else {
        return Ok(());
    };
    let parsed = text(&capture).and_then(|body| {
        serde_json::from_str::<Value>(body).map_err(|source| CrawlError::Decode {
            url: url.clone(),
            source,
        })
    });
    let rows = match parsed {
        Ok(Value::Array(rows)) => rows,
        Ok(_) => return fail(&mut run.report, &url, "missing coach array"),
        Err(error) => return fail(&mut run.report, &url, error),
    };
    rows.into_iter()
        .enumerate()
        .try_for_each(|(ordinal, value)| {
            let locator = format!("{url}#row={ordinal}");
            let record = match serde_json::from_value::<CoachRecord>(value) {
                Ok(record) if !record.name.trim().is_empty() && !record.level.trim().is_empty() => {
                    record
                }
                Ok(_) => return fail(&mut run.report, &locator, "missing coach name or level"),
                Err(error) => return fail(&mut run.report, &locator, error),
            };
            let teams = TeamCoaches {
                node: node.clone(),
                api_url: url.clone(),
                records: vec![record],
            };
            let coaches = coach_entities(
                std::slice::from_ref(&teams),
                school_id,
                domains,
                &capture.fetched_at,
            );
            run.publish_coaches(&locator, &coaches)
        })
}
