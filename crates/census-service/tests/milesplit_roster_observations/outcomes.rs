use super::*;

fn team() -> TestResult<milesplit::TeamRef> {
    milesplit::parse_team_index(WI_TEAMS_FIXTURE)?
        .teams
        .into_iter()
        .next()
        .ok_or_else(|| "index fixture lists no teams".into())
}

fn fetcher(store: &Store) -> TestResult<Fetcher> {
    Ok(Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::ZERO,
        std::collections::HashMap::new(),
        Vec::new(),
    )?
    .with_offline(true))
}

async fn collect(
    store: &Store,
    fetcher: &Fetcher,
    team: &milesplit::TeamRef,
) -> TestResult<census::StateProgress> {
    Ok(census::collect_state_rosters(
        fetcher,
        store,
        std::slice::from_ref(team),
        &collect_options(),
        UsJurisdiction::Wisconsin,
    )
    .await?)
}

fn accepted_names(body: &str, team: &milesplit::TeamRef) -> TestResult<Vec<String>> {
    let verdict = milesplit::parse_roster(body, team.clone())?;
    let mut names = verdict
        .roster()
        .map(|roster| {
            roster
                .athletes
                .iter()
                .map(|athlete| athlete.name.clone())
                .collect::<Vec<_>>()
        })
        .map_or_else(Vec::new, core::convert::identity);
    names.sort();
    Ok(names)
}

fn observed_names(store: &Store) -> TestResult<Vec<String>> {
    let observations: Vec<SourceObservation> = store.scan(Table::SourceObservations)?;
    let mut names = observations
        .into_iter()
        .filter_map(|row| match row {
            SourceObservation::Athlete(athlete) => Some(athlete.observed_name),
            SourceObservation::School(_) => None,
        })
        .collect::<Vec<_>>();
    names.sort();
    Ok(names)
}

#[tokio::test]
async fn partial_and_quarantined_rosters_preserve_source_facts_and_remain_resumable() -> TestResult
{
    let partial =
        WI_ROSTER_FIXTURE.replacen("column-grad-year\">2027", "column-grad-year\">invalid", 1);
    for body in [partial.as_str(), "<html>unrecognized page</html>"] {
        let root = tempfile::tempdir()?;
        let store = Store::open(root.path())?;
        let team = team()?;
        let document = synthetic_owned_roster_document(&team, body);
        seed_cache(
            &store.http_cache_dir(),
            &format!("{}/roster", team.url),
            &document,
        )?;
        let fetcher = fetcher(&store)?;
        configure_inventory(&store, &fetcher, &team).await?;
        let first = collect(&store, &fetcher, &team).await?;
        check!(!first.is_terminal());
        check!(eq; (first.rosters_committed, first.rosters_remaining), (0, 1));
        let names = accepted_names(&document, &team)?;
        check!(eq; observed_names(&store)?, names);
        let before = replay::digest(&store)?;
        drop(store);
        let reopened = Store::open(root.path())?;
        let resumed = collect(&reopened, &fetcher, &team).await?;
        check!(!resumed.is_terminal());
        check!(eq; resumed.errors, first.errors);
        check!(eq; resumed.rosters_remaining, 1);
        check!(eq; observed_names(&reopened)?, names);
        check!(eq; replay::digest(&reopened)?, before);
        let corrected = synthetic_owned_roster_document(&team, WI_ROSTER_FIXTURE);
        seed_cache(
            &reopened.http_cache_dir(),
            &format!("{}/roster", team.url),
            &corrected,
        )?;
        let complete = collect(&reopened, &fetcher, &team).await?;
        check!(complete.is_terminal());
        check!(eq; complete.rosters_remaining, 0);
        check!(eq; observed_names(&reopened)?, accepted_names(&corrected, &team)?);
        let complete_digest = replay::digest(&reopened)?;
        check!(collect(&reopened, &fetcher, &team).await?.is_terminal());
        check!(eq; replay::digest(&reopened)?, complete_digest);
    }
    Ok(())
}

#[tokio::test]
async fn conflicting_requested_jurisdiction_never_admits_athletes_or_completion() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let mut team = team()?;
    team.city_state = "Washington, DC".to_string();
    let document = synthetic_owned_roster_document(&team, WI_ROSTER_FIXTURE);
    seed_cache(
        &store.http_cache_dir(),
        &format!("{}/roster", team.url),
        &document,
    )?;
    let fetcher = fetcher(&store)?;
    configure_inventory(&store, &fetcher, &team).await?;
    let first = collect(&store, &fetcher, &team).await?;
    check!(!first.is_terminal());
    check!(eq; first.rosters_remaining, 1);
    check!(eq; observed_names(&store)?, Vec::<String>::new());
    let before = replay::digest(&store)?;
    drop(store);
    let reopened = Store::open(root.path())?;
    let resumed = collect(&reopened, &fetcher, &team).await?;
    check!(!resumed.is_terminal());
    check!(eq; resumed.errors, first.errors);
    check!(eq; resumed.rosters_remaining, 1);
    check!(eq; replay::digest(&reopened)?, before);
    Ok(())
}
