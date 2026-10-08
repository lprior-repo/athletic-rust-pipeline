use census_crawl::milesplit::{self, boundary, Roster, RosterOutcome, TeamRef};
use census_crawl::net::{FetchOptions, Fetcher};
use census_crawl::{CrawlError, CrawlResult};
use census_store::{Application, Store};
use futures::{stream, TryStreamExt};

use super::{rosters_phase, units::RosterRun};

mod commit;
mod counts;
mod effects;
pub(crate) mod journal;
mod metadata;
pub(super) mod refusal;
mod summary;
#[cfg(test)]
mod tests;

use commit::Commit;
use counts::Counts;
use journal::Records;
use metadata::Window;
pub(super) use refusal::retain_error;
pub(super) use summary::{remaining, summarize, Summary};

#[tracing::instrument(skip_all, fields(team = team.id))]
pub(super) async fn fetch_and_store(
    fetcher: &Fetcher,
    store: &Store,
    team: &TeamRef,
    run: &RosterRun<'_>,
) -> CrawlResult<Application> {
    match milesplit::fetch_roster(
        fetcher,
        team,
        &FetchOptions {
            refresh: run.refresh,
            allow_not_found: true,
            headers: Vec::new(),
        },
        boundary_hook(run),
    )
    .await
    {
        Ok(read) => apply(store, team, run, &read).await,
        Err(error) => Err(retain_error(store, team, run, error)),
    }
}

#[tracing::instrument(skip_all, fields(team = team.id))]
async fn apply(
    store: &Store,
    team: &TeamRef,
    run: &RosterRun<'_>,
    read: &RosterOutcome,
) -> CrawlResult<Application> {
    let commit = Commit::new(store, team, run, read)?;
    let outcome = match read.verdict.roster() {
        Some(roster) => apply_roster(&commit, roster, run).await,
        None => commit.window(None, &Counts::default(), 0, true).await,
    };
    outcome.map_err(|error| refusal::retain_capture_error(store, team, run, read, error))
}

async fn apply_roster(
    commit: &Commit<'_>,
    roster: &Roster,
    run: &RosterRun<'_>,
) -> CrawlResult<Application> {
    let counts = Counts::new()?;
    stream::try_unfold((0, counts), |(offset, mut counts)| async move {
        let Some(window) = Window::next(roster, offset, run.observed_on)? else {
            return Ok(None);
        };
        let records = Records::of(&window, &run.site, run.school_year)?;
        counts.include(window.athletes(), records.teams())?;
        let next =
            offset
                .checked_add(window.athletes().len())
                .ok_or_else(|| CrawlError::Arithmetic {
                    detail: "roster window cursor overflowed".to_string(),
                })?;
        let application = commit
            .window(
                Some(&records),
                &counts,
                offset,
                next == roster.athletes.len(),
            )
            .await?;
        Ok::<_, CrawlError>(Some((application, (next, counts))))
    })
    .try_fold(None, |_, application| async move {
        Ok::<_, CrawlError>(Some(application))
    })
    .await?
    .ok_or_else(|| CrawlError::Invariant {
        detail: "complete roster supplied no accepted rows".to_string(),
    })
}

fn preserve_counts<S, C, R>(journal: &mut journal::Journal<S, C, R>, prior: &journal::Journal) {
    journal.athletes = journal.athletes.max(prior.athletes);
    journal.teams = journal.teams.max(prior.teams);
    journal.co2027 = journal.co2027.max(prior.co2027);
    journal.co2027_boys = journal.co2027_boys.max(prior.co2027_boys);
    journal.co2027_girls = journal.co2027_girls.max(prior.co2027_girls);
}

async fn reached(run: &RosterRun<'_>, point: boundary::Point) -> CrawlResult<()> {
    match boundary_hook(run) {
        Some(hook) => hook.reached(point).await,
        None => Ok(()),
    }
}

pub(super) fn prior(
    store: &Store,
    team: &TeamRef,
    run: &RosterRun<'_>,
) -> CrawlResult<Option<journal::Journal>> {
    let phase = rosters_phase(run.site.jurisdiction(), run.school_year, run.revision);
    let key = format!("{}:{}", run.site.jurisdiction().code(), team.id);
    store
        .journal_payload(&phase, &key)?
        .map(|value| {
            let journal: journal::Journal =
                serde_json::from_value(value).map_err(|source| CrawlError::Decode {
                    url: format!("journal:{phase}/{key}"),
                    source,
                })?;
            if !journal.matches_scope(&team.id, run.school_year) {
                return Err(CrawlError::Invariant {
                    detail: format!("foreign roster receipt at {phase}/{key}"),
                });
            }
            Ok(journal)
        })
        .transpose()
}

#[cfg(feature = "native-fault-injection")]
static BOUNDARY_HOOK: std::sync::OnceLock<&'static dyn boundary::Hook> = std::sync::OnceLock::new();

#[cfg(feature = "native-fault-injection")]
pub fn install_boundary_hook(hook: &'static dyn boundary::Hook) {
    if BOUNDARY_HOOK.set(hook).is_err() {
        tracing::warn!("roster boundary hook was already installed");
    }
}

fn boundary_hook(_run: &RosterRun<'_>) -> Option<&'static dyn boundary::Hook> {
    #[cfg(feature = "native-fault-injection")]
    {
        BOUNDARY_HOOK.get().copied()
    }
    #[cfg(not(feature = "native-fault-injection"))]
    {
        None
    }
}
