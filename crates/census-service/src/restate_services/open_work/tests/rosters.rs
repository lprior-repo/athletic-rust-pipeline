use super::*;
use census_crawl::milesplit::Site;
use census_crawl::net::Fetcher;
use census_domain::model::CanonicalAthlete;
use std::path::Path;
use std::time::Duration;

fn seed_cache(cache: &Path, url: &str, body: &str) -> TestResult {
    let mut hasher = Sha256::new();
    hasher.update(b"GET\x1f");
    hasher.update(url.as_bytes());
    hasher.update(b"\x1f");
    let hash = format!("{:x}", hasher.finalize());
    let key = hash.get(..32).ok_or("invalid cache identity")?;
    let meta = serde_json::json!({ "url": url, "method": "GET", "status": 200,
        "content_digest": format!("{:x}", Sha256::digest(body.as_bytes())), "bytes": body.len(),
        "fetched_at": "2026-10-07T12:00:00Z", "content_type": "text/html" });
    std::fs::create_dir_all(cache)?;
    std::fs::write(cache.join(format!("{key}.body")), body)?;
    std::fs::write(
        cache.join(format!("{key}.meta.json")),
        serde_json::to_vec(&meta)?,
    )?;
    Ok(())
}

async fn partial_prefix(store: &Store) -> TestResult {
    let jurisdiction = UsJurisdiction::Wisconsin;
    let site = Site::for_jurisdiction(jurisdiction);
    let index = r#"<meta name="application-name" content="MileSplit"><table class="teams order-table table"><tbody><tr><td><a href="https://wi.milesplit.com/teams/52649-example">Example School</a></td><td>Madison, WI</td></tr></tbody></table>"#;
    let body = r#"<link rel="canonical" href="https://wi.milesplit.com/teams/52649-example/roster"><ul id="rosterDataset"><li class="athlete-row data-row"><a href="https://wi.milesplit.com/athletes/1-runner">Runner, Alice</a><div class="column-gender">m</div><div class="column-grad-year">2027</div><div data-season-id="2"><svg class="icon-yes"></svg></div></li><li class="athlete-row data-row"><a href="https://wi.milesplit.com/athletes/2-runner">Runner, Bob</a><div class="column-gender">m</div><div class="column-grad-year"></div></li></ul>"#;
    seed_cache(&store.http_cache_dir(), &site.teams_url(), index)?;
    seed_cache(
        &store.http_cache_dir(),
        "https://wi.milesplit.com/teams/52649-example/roster",
        body,
    )?;
    let fetcher = Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::ZERO,
        std::collections::HashMap::new(),
        vec!["wi.milesplit.com".to_string()],
    )?
    .with_offline(true);
    let teams = crate::census::collect_state_teams(&fetcher, store, jurisdiction, false).await?;
    let options = crate::census::CollectOptions {
        jurisdictions: vec![jurisdiction],
        limit_per_state: None,
        concurrency: 1,
        state_concurrency: 1,
        refresh: false,
        school_year: SchoolYear::DEFAULT,
        observed_on: "2026-10-07".to_string(),
        revision: std::num::NonZeroU32::MIN,
    };
    let progress =
        crate::census::collect_state_rosters(&fetcher, store, &teams, &options, jurisdiction)
            .await?;
    check!(eq; (progress.rosters_committed, progress.rosters_remaining, progress.athletes), (0, 1, 1));
    let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    check!(eq; athletes.into_iter().map(|athlete| athlete.canonical_name).collect::<Vec<_>>(), vec!["Alice Runner".to_string()]);
    Ok(())
}

#[tokio::test]
async fn cached_native_complete_cannot_hide_a_durable_partial_roster() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = bound_store(root.path())?;
    partial_prefix(&store).await?;
    let mut state = historical_state()?;
    state.rosters = Some(crate::census::StateProgress {
        jurisdiction: UsJurisdiction::Wisconsin,
        rosters_total: 1,
        rosters_committed: 1,
        rosters_remaining: 0,
        rosters_skipped: 0,
        athletes: 1,
        class_of_2027: 1,
        class_of_2027_boys: 1,
        class_of_2027_girls: 0,
        errors: Vec::new(),
        blocked: false,
        blocked_skipped: 0,
        teams: 1,
    });
    let evidence = inspect_store(&store, SchoolYear::DEFAULT, Revision(1))?;
    let stages = stages_of(
        &state,
        Disposition::Complete,
        Disposition::Complete,
        evidence.rosters(UsJurisdiction::Wisconsin),
    )
    .map_err(handler_error)?;
    check!(eq; stages.rosters, Disposition::Partial);
    check!(eq; stages.owed_rosters, 1);
    check!(stages.owing().contains(&"rosters"));
    check!(evidence.objects.iter().any(|row| row.endpoint
        == "https://wi.milesplit.com/teams/52649-example/roster"
        && row.disposition == Disposition::Partial));
    Ok(())
}
