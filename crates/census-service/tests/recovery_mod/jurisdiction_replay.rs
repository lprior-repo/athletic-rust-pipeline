use super::material::{store_seeded_with_wisconsin, wi_options};
use super::{
    census, count_of, fetcher_for, journal_keys, milesplit, note, open_store, table_counts,
    wi_rosters_phase, wi_teams_phase, BTreeMap, BTreeSet, CanonicalAthlete, GradYear, Table,
    UsJurisdiction, WI_ROSTER_FIXTURE,
};

#[test]
fn jurisdiction_walk_resumes_from_the_journaled_index_and_the_unclaimed_rosters(
) -> super::TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
    const SCENARIO: &str = "jurisdiction-walk";
    let dir = tempfile::tempdir()?;
    let site = milesplit::Site::for_jurisdiction(UsJurisdiction::Wisconsin);

    let control_root = dir.path().join("control");
    let control = {
        let store = store_seeded_with_wisconsin(&control_root, &site)?;
        let fetcher = fetcher_for(&store)?;
        let teams = census::collect_state_teams(&fetcher, &store, UsJurisdiction::Wisconsin, false)
            .await
            ?;
        let index_stats = fetcher.stats().await;
        check!(eq; index_stats.requests, 0,
        "the team index must come from the seeded cache");
        check!(index_stats.cache_hits >= 1);
        let progress = census::collect_state_rosters(
            &fetcher,
            &store,
            &teams,
            &wi_options(None),
            UsJurisdiction::Wisconsin,
        )
        .await
        ?;
        let athletes = store
            .scan::<CanonicalAthlete>(Table::Athletes)
            ?;
        let parsed = milesplit::parse_roster(WI_ROSTER_FIXTURE, teams[0].clone())?;
        let per_roster = parsed
            .roster()
            .ok_or("roster fixture carries no readable athletes")?
            .athletes
            .iter()
            .filter(|athlete| athlete.grad_year == GradYear::CO2027)
            .count();
        check!(per_roster > 0, "the roster fixture carries Co2027 athletes");
        let walk = Walk {
            teams,
            progress,
            athletes,
            index_journal: journal_keys(&store, &wi_teams_phase())?,
            roster_journal: journal_keys(&store, &wi_rosters_phase())?,
            counts: table_counts(&store)?,
            stats: fetcher.stats().await,
            per_roster,
        };
        note(
            SCENARIO,
            format!(
                "control: teams={} rosters_committed={} co2027={} athletes={} requests={} cache_hits={} \
                 tables={:?}",
                walk.teams.len(),
                walk.progress.rosters_committed,
                walk.progress.class_of_2027,
                walk.athletes.len(),
                walk.stats.requests,
                walk.stats.cache_hits,
                walk.counts
            ),
        );
        check!(eq; walk.index_journal,
        BTreeSet::from(["WI".to_string()]),
        "the team index is journaled once per state");
        check!(eq; walk.progress.rosters_committed,
        walk.teams.len(),
        "a clean pass walks every roster the index lists");
        check!(eq; walk.progress.rosters_skipped, 0);
        check!(eq;
            walk.progress.rosters_committed
                + walk.progress.rosters_skipped
                + walk.progress.rosters_remaining,
            walk.progress.rosters_total,
            "the three roster buckets reconstruct the indexed teams");
        check!(walk.progress.errors.is_empty(),
        "the seeded cache leaves no roster unfetched: {:?}",
        walk.progress.errors);
        check!(eq; walk.stats.requests, 0, "no walk step needs the network");
        check!(eq; walk.roster_journal.len(),
        walk.teams.len(),
        "one journal entry per finished roster");
        check!(eq; walk.progress.class_of_2027,
        per_roster * walk.teams.len(),
        "every walked roster reports its own Co2027 count");
        walk
    };

    let restart_root = dir.path().join("restart");
    let stopped_after = {
        let store = store_seeded_with_wisconsin(&restart_root, &site)?;
        let fetcher = fetcher_for(&store)?;
        let teams = census::collect_state_teams(&fetcher, &store, UsJurisdiction::Wisconsin, false)
            .await
            ?;
        let first = census::collect_state_rosters(
            &fetcher,
            &store,
            &teams,
            &wi_options(Some(1)),
            UsJurisdiction::Wisconsin,
        )
        .await
        ?;
        let journal = journal_keys(&store, &wi_rosters_phase())?;
        note(
            SCENARIO,
            format!(
                "first pass (limit 1): rosters_committed={} skipped={} journal={journal:?}",
                first.rosters_committed, first.rosters_skipped
            ),
        );
        check!(eq; first.rosters_committed, 1);
        check!(eq; first.rosters_skipped, 0,
        "held counts rosters retained without clean admission, and this clean pass admits every \
         walked row: the limit defers the rest to remaining instead");
        check!(eq; journal.len(), 1);
        journal
    };

    let store = open_store(&restart_root)?;
    let fetcher = fetcher_for(&store)?;
    let teams = census::collect_state_teams(&fetcher, &store, UsJurisdiction::Wisconsin, false)
        .await
        ?;
    let index_stats = fetcher.stats().await;
    note(
        SCENARIO,
        format!(
            "replay: teams={} requests={} cache_hits={}",
            teams.len(),
            index_stats.requests,
            index_stats.cache_hits
        ),
    );
    check!(eq; teams, control.teams,
    "the second stage re-reads the journaled index rather than rebuilding it");
    check!(eq; index_stats.requests, 0,
    "the index replay is answered from the cache");

    let resumed = census::collect_state_rosters(
        &fetcher,
        &store,
        &teams,
        &wi_options(None),
        UsJurisdiction::Wisconsin,
    )
    .await
    ?;
    let athletes = store
        .scan::<CanonicalAthlete>(Table::Athletes)
        ?;
    let counts = table_counts(&store)?;
    let stats = fetcher.stats().await;
    note(
        SCENARIO,
        format!(
            "restart: rosters_committed={} skipped={} co2027={} athletes={} requests={} cache_hits={} \
             tables={counts:?}",
            resumed.rosters_committed,
            resumed.rosters_skipped,
            resumed.class_of_2027,
            athletes.len(),
            stats.requests,
            stats.cache_hits
        ),
    );
    check!(eq; resumed.rosters_committed,
    teams.len(),
    "the restart reports the state's cumulative commit count, not this pass's");
    check!(eq; resumed.rosters_skipped, 0,
    "a fully admitted state holds no roster back");
    check!(eq;
        resumed.rosters_committed + resumed.rosters_skipped + resumed.rosters_remaining,
        resumed.rosters_total,
        "the restarted buckets match the indexed teams");
    check!(eq; stats.requests, 0,
    "every roster the first pass journaled is skipped unread");
    check!(stopped_after.is_subset(&journal_keys(&store, &wi_rosters_phase())?),
    "the restarted store keeps every roster the limited first pass journaled");
    check!(resumed.errors.is_empty(),
    "the resumed pass leaves no roster unfetched: {:?}",
    resumed.errors);
    check!(eq; journal_keys(&store, &wi_rosters_phase())?,
    control.roster_journal,
    "the restarted store completes exactly the roster set the control pass covered");
    check!(eq; journal_keys(&store, &wi_teams_phase())?,
    control.index_journal,
    "the team-index journal is untouched by the roster stage");
    check!(eq; resumed.class_of_2027,
    control.per_roster * teams.len(),
    "the resumed pass reports the cohort the journal holds for the whole state");
    check!(eq; athletes, control.athletes,
    "the restarted store merges to exactly the control's athletes");
    check!(eq; counts, control.counts,
    "observation counters match the control: no roster was written twice");
    check!(eq; count_of(&counts, Table::Athletes),
    count_of(&control.counts, Table::Athletes),
    "athlete observations match the control one for one");
    Ok(())
        })
}

struct Walk {
    teams: Vec<milesplit::TeamRef>,
    progress: census::StateProgress,
    athletes: Vec<CanonicalAthlete>,
    index_journal: BTreeSet<String>,
    roster_journal: BTreeSet<String>,
    counts: BTreeMap<String, u64>,
    stats: census_crawl::net::FetchStats,
    per_roster: usize,
}
