use super::*;

#[tracing::instrument(skip_all, fields(state = jurisdiction.code()))]
pub async fn collect_state_rosters(
    fetcher: &Fetcher,
    store: &Store,
    teams: &[TeamRef],
    options: &CollectOptions,
    jurisdiction: UsJurisdiction,
) -> CrawlResult<StateProgress> {
    collect_rosters(
        fetcher,
        store,
        teams,
        options,
        jurisdiction,
        units::RosterSource::Historical,
    )
    .await
}

pub async fn collect_owned_state_rosters(
    fetcher: &Fetcher,
    store: &Store,
    teams: &[TeamRef],
    options: &CollectOptions,
    jurisdiction: UsJurisdiction,
    source: &OwnedRosterSource<'_>,
) -> CrawlResult<StateProgress> {
    collect_rosters(
        fetcher,
        store,
        teams,
        options,
        jurisdiction,
        units::RosterSource::Owned(source),
    )
    .await
}

async fn collect_rosters(
    fetcher: &Fetcher,
    store: &Store,
    teams: &[TeamRef],
    options: &CollectOptions,
    jurisdiction: UsJurisdiction,
    source: units::RosterSource<'_>,
) -> CrawlResult<StateProgress> {
    team_bound(teams.len())?;
    let pending = source.pending(store, teams, options, jurisdiction)?;
    let shared = shared(pending.len())?;
    let run = RosterRun {
        site: Site::for_jurisdiction(jurisdiction),
        observed_on: &options.observed_on,
        school_year: options.school_year,
        refresh: options.refresh,
        revision: options.revision,
    };
    stream::iter(&pending)
        .take(options.limit_per_state.map_or(pending.len(), |limit| limit))
        .for_each_concurrent(options.concurrency.clamp(1, 64), |team| {
            roster_unit(fetcher, store, &shared, team, &run, source)
        })
        .await;
    let phase = rosters_phase(jurisdiction, options.school_year, options.revision);
    let summary = roster::summarize(store, &phase, teams, jurisdiction, options.school_year)?;
    let progress = progress_of(
        jurisdiction,
        summary.total,
        summary,
        &mut *shared.lock().await,
    )?;
    index::guard_progress(store, jurisdiction, teams, progress)
}
