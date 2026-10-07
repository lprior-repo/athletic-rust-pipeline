use super::input::Inputs;
use super::{io, preserve, Result};
use census_crawl::milesplit::{
    fetch_roster, fetch_team_index, roster_entities, RosterVerdict, Site,
};
use census_crawl::net::{FetchOptions, FetchStats};
use census_domain::model::{CanonicalSchool, SchoolYear, SourceNamespace};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::Path;

#[derive(Serialize)]
pub(super) struct Binding {
    pub school: CanonicalSchool,
    pub provider_team_id: u64,
    pub fetch_stats: FetchStats,
    pub offline: bool,
    pub provenance: Value,
}

#[tracing::instrument(skip(store, inputs))]
pub(super) async fn derive(store: &Store, inputs: &Inputs) -> Result<Binding> {
    let fetcher = preserve::fetcher(store)?;
    let site = Site::for_jurisdiction(UsJurisdiction::Alabama);
    let index = fetch_team_index(&fetcher, site, &FetchOptions::default()).await?;
    if index.len() > io::MAX_ROWS {
        return Err("team index exceeds smoke row bound".into());
    }
    let [_, _, _, original] = &inputs.captures;
    let mut matching = index
        .iter()
        .filter(|team| format!("{}/roster", team.url) == original.metadata.url);
    let team = matching
        .next()
        .ok_or("genuine roster URL absent from original index")?;
    if matching.next().is_some() {
        return Err("original index has ambiguous roster owner".into());
    }
    let outcome = fetch_roster(&fetcher, team, &FetchOptions::default(), None).await?;
    if outcome.capture.body != original.body
        || outcome.capture.fetched_at != original.metadata.fetched_at
        || outcome.capture.url != original.metadata.url
        || outcome.capture.status != 200
        || outcome.capture.content_digest != original.metadata.content_digest
    {
        return Err("offline roster differs from authentic original capture".into());
    }
    let stats = fetcher.stats().await;
    project(
        &outcome.verdict,
        site,
        original,
        stats,
        fetcher.is_offline(),
    )
}

fn project(
    verdict: &RosterVerdict,
    site: Site,
    original: &super::input::Capture,
    fetch_stats: FetchStats,
    offline: bool,
) -> Result<Binding> {
    let roster = verdict
        .roster()
        .ok_or("authentic roster quarantined by production parser")?;
    if roster.athletes.is_empty() || roster.athletes.len() > io::MAX_ROWS {
        return Err("authentic roster population absent or exceeds bound".into());
    }
    let acquired = chrono::DateTime::parse_from_rfc3339(&original.metadata.fetched_at)?;
    let roster_year = SchoolYear::new(acquired.format("%Y").to_string().parse()?)
        .ok_or("invalid roster school year")?;
    let (school, athletes, teams) =
        roster_entities(roster, roster_year, &original.metadata.fetched_at, &site);
    let provider_team_id = roster.team.id.parse::<u64>()?;
    if !school.source_identities.iter().any(|identity| {
        identity.namespace == SourceNamespace::MilesplitSchool && identity.id == roster.team.id
    }) || school.state != Some(site.jurisdiction())
    {
        return Err("production roster school lacks exact qualified provider identity".into());
    }
    Ok(Binding {
        school,
        provider_team_id,
        fetch_stats,
        offline,
        provenance: json!({"index_owner": {"id": roster.team.id, "name": roster.team.name,
            "url": roster.team.url, "city_state": roster.team.city_state},
            "roster_capture": original.metadata, "roster_school_year": roster_year,
            "roster_parse_status": match verdict { RosterVerdict::Complete { .. } => "complete",
                RosterVerdict::Partial { .. } => "partial", RosterVerdict::Quarantined { .. } => "quarantined" },
            "roster_rejections": verdict.rejections(), "projected_roster_athletes_not_ingested": athletes,
            "projected_roster_teams_not_ingested": teams,
            "role": "production roster_entities school binding only; no roster athlete/team population intake"}),
    })
}

pub(super) fn retain(store: &Store, root: &Path, binding: &Binding) -> Result<()> {
    io::json(&root.join("binding.json"), binding)?;
    store.append(Table::Schools, &binding.school)?;
    store.consolidate::<CanonicalSchool>(Table::Schools, &store.out_dir().join("schools.jsonl"))?;
    let schools: Vec<CanonicalSchool> = store.scan(Table::Schools)?;
    let consolidated: Vec<CanonicalSchool> =
        census_store::read::read_rows(&store.out_dir().join("schools.jsonl"))?;
    if schools.as_slice() != std::slice::from_ref(&binding.school) || consolidated != schools {
        return Err("retained qualified school and collector school input disagree".into());
    }
    Ok(())
}
