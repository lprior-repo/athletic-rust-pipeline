use super::artifacts;
use anyhow::{ensure, Context, Result};
use census_crawl::milesplit::{
    parse_roster, parse_team_index, roster_entities, RosterVerdict, Site,
};
use census_domain::{model::SchoolYear, UsJurisdiction};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct Input {
    jurisdiction: String,
    team_id: String,
    index: Capture,
    roster: Capture,
}

#[derive(Deserialize)]
struct Capture {
    body: PathBuf,
    metadata: PathBuf,
}

pub fn prepare(root: &Path, input: &Path) -> Result<Value> {
    let input: Input = serde_json::from_slice(&artifacts::read(input)?)?;
    let jurisdiction =
        UsJurisdiction::from_code(&input.jurisdiction).context("unknown jurisdiction")?;
    let site = Site::for_jurisdiction(jurisdiction);
    let (index, index_meta) = load(&input.index, &site.teams_url())?;
    let mut matching = parse_team_index(std::str::from_utf8(&index)?)?
        .into_iter()
        .filter(|team| team.id == input.team_id);
    let team = matching
        .next()
        .context("capture contains no requested source team")?;
    ensure!(matching.next().is_none(), "ambiguous source team owner");
    let (roster_bytes, roster_meta) = load(&input.roster, &format!("{}/roster", team.url))?;
    let verdict = parse_roster(std::str::from_utf8(&roster_bytes)?, team)?;
    let roster = match verdict {
        RosterVerdict::Complete { roster } => roster,
        other => anyhow::bail!("qualification requires complete authentic roster: {other:?}"),
    };
    let captured = roster_meta
        .get("fetched_at")
        .and_then(Value::as_str)
        .context("capture date absent")?;
    let (_, athletes, _) = roster_entities(
        &roster,
        SchoolYear::new(2026).context("season invalid")?,
        captured,
        &site,
    );
    let rows = athletes
        .into_iter()
        .filter(|athlete| athlete.grad_year.get() == 2027)
        .map(serde_json::to_value)
        .collect::<serde_json::Result<Vec<_>>>()?;
    ensure!(
        !rows.is_empty() && rows.len() <= 100,
        "require 1..100 genuine Class-of-2027 rows"
    );
    let payload = json!({"table":"athletes", "rows":rows, "operation_id":"vm_fixture_replay_2026_r1:athletes:0", "cursor":input.team_id});
    [("index.body", index), ("roster.body", roster_bytes)]
        .into_iter()
        .try_for_each(|(name, bytes)| artifacts::write(&root.join(name), &bytes))?;
    artifacts::publish(&root.join("index.meta.json"), &index_meta)?;
    artifacts::publish(&root.join("roster.meta.json"), &roster_meta)?;
    artifacts::publish(&root.join("request.json"), &payload)?;
    Ok(
        json!({"request":payload, "captures":[index_meta,roster_meta], "season":2026, "cohort":2027, "revision":1, "fresh_public_acquisition":false, "transport":"SSH fixture replay", "date_semantics":"immutable supplied source capture dates; not replay installation time"}),
    )
}

fn load(capture: &Capture, expected_url: &str) -> Result<(Vec<u8>, Value)> {
    let body = artifacts::read(&capture.body)?;
    let metadata = artifacts::json(&capture.metadata)?;
    ensure!(
        metadata.get("url").and_then(Value::as_str) == Some(expected_url),
        "capture source URL mismatch"
    );
    ensure!(
        metadata.get("content_digest").and_then(Value::as_str)
            == Some(artifacts::sha(&body).as_str()),
        "capture digest mismatch"
    );
    ensure!(
        metadata.get("status").and_then(Value::as_u64) == Some(200),
        "capture is not successful public response"
    );
    ensure!(
        metadata.get("bytes").and_then(Value::as_u64) == Some(u64::try_from(body.len())?),
        "capture byte length mismatch"
    );
    let date = metadata
        .get("fetched_at")
        .and_then(Value::as_str)
        .context("original fetched_at required")?;
    chrono::DateTime::parse_from_rfc3339(date)
        .context("original fetched_at must be honest RFC3339")?;
    ensure!(!body.is_empty(), "capture body empty");
    Ok((body, metadata))
}
