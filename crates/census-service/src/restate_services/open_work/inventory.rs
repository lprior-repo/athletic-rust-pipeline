use super::{complete, count, history, obligation, push, ReadJurisdiction};
use crate::restate_services::wire::{JurisdictionState, SourceObjectOpen, TeamsSourceOutcome};
use crate::restate_services::{jurisdiction::TeamsSourceClient, teams_arms};
use census_crawl::registry;
use census_crawl::CollectionDisposition as Disposition;
use futures::{stream, StreamExt, TryStreamExt};
use restate_sdk::prelude::*;

pub(super) fn validate(state: &JurisdictionState) -> Result<(), HandlerError> {
    let limit = registry::descriptors().count();
    let bounded = state.history.meets.len() <= 8
        && state.history.results.len() <= 8
        && state
            .history
            .meets
            .values()
            .all(|outcome| outcome.sources.len() <= limit)
        && state
            .history
            .results
            .values()
            .all(|outcome| outcome.per_source.len() <= limit)
        && state.plan.as_ref().is_none_or(|plan| {
            plan.sweepable.len() <= limit
                && plan.refused.len() <= limit
                && plan
                    .sweepable
                    .iter()
                    .all(|slug| slug.len() <= super::MAX_ENDPOINTS)
                && plan
                    .refused
                    .iter()
                    .all(|row| row.slug.len() <= super::MAX_ENDPOINTS)
        });
    if bounded {
        Ok(())
    } else {
        Err(super::capacity())
    }
}

pub(super) fn plan_covers(
    jurisdiction: census_domain::UsJurisdiction,
    state: &JurisdictionState,
) -> bool {
    let Some(plan) = &state.plan else {
        return false;
    };
    let required = census_crawl::applicability::applicable_sources(jurisdiction);
    !plan.fingerprint.is_empty()
        && plan.sweepable.len().checked_add(plan.refused.len()) == Some(required.len())
        && required.iter().all(|source| {
            plan.sweepable
                .iter()
                .filter(|slug| slug.as_str() == source.slug)
                .count()
                .checked_add(
                    plan.refused
                        .iter()
                        .filter(|row| row.slug == source.slug)
                        .count(),
                )
                == Some(1)
        })
}

fn append_frontiers(
    entry: &ReadJurisdiction,
    rows: &mut Vec<SourceObjectOpen>,
) -> Result<(), HandlerError> {
    let Some(window) = entry.state.history_window else {
        return push(
            rows,
            obligation(
                format!("{}/history/unmeasured", entry.row.identity),
                Disposition::Unknown,
            ),
        );
    };
    window.years().try_for_each(|year| {
        [history::Kind::Meets, history::Kind::Results]
            .into_iter()
            .try_for_each(|kind| {
                push(
                    rows,
                    obligation(
                        format!(
                            "{}/history/{year}/{}/frontier",
                            entry.row.identity,
                            kind.name()
                        ),
                        complete(history::stage_complete(&entry.state, year, kind)),
                    ),
                )
            })
    })
}

pub(super) async fn append(
    ctx: &Context<'_>,
    jurisdictions: &[ReadJurisdiction],
    rows: &mut Vec<SourceObjectOpen>,
) -> Result<(), HandlerError> {
    stream::iter(jurisdictions)
        .map(Ok::<_, HandlerError>)
        .try_fold(rows, |rows, entry| async move {
            append_entry(ctx, entry, rows).await?;
            Ok(rows)
        })
        .await
        .map(|_| ())
}

async fn append_entry(
    ctx: &Context<'_>,
    entry: &ReadJurisdiction,
    rows: &mut Vec<SourceObjectOpen>,
) -> Result<(), HandlerError> {
    push(
        rows,
        obligation(
            format!("{}/rosters", entry.row.identity),
            entry.row.stages.rosters,
        ),
    )?;
    append_frontiers(entry, rows)?;
    if !plan_covers(entry.row.jurisdiction, &entry.state) {
        push(
            rows,
            obligation(
                format!("{}/source-plan/incomplete-inventory", entry.row.identity),
                Disposition::Unknown,
            ),
        )?;
    }
    let Some(plan) = &entry.state.plan else {
        return push(
            rows,
            obligation(
                format!("{}/source-plan/unmeasured", entry.row.identity),
                Disposition::Unknown,
            ),
        );
    };
    plan.refused.iter().try_for_each(|refusal| {
        push(
            rows,
            obligation(
                format!(
                    "{}/refused/{}/{:?}",
                    entry.row.identity, refusal.slug, refusal.kind
                ),
                Disposition::Blocked,
            ),
        )
    })?;
    stream::iter(&plan.sweepable)
        .map(Ok::<_, HandlerError>)
        .try_fold(rows, |rows, slug| async move {
            append_source(ctx, entry, slug, rows).await?;
            Ok(rows)
        })
        .await
        .map(|_| ())
}

async fn append_source(
    ctx: &Context<'_>,
    entry: &ReadJurisdiction,
    slug: &str,
    rows: &mut Vec<SourceObjectOpen>,
) -> Result<(), HandlerError> {
    let teams = teams_arms::arm_for(slug).is_some();
    if teams {
        push(rows, team_object(ctx, &entry.row.identity, slug).await?)?;
    }
    if !teams && !history::Kind::Meets.requires(slug) && !history::Kind::Results.requires(slug) {
        push(
            rows,
            obligation(
                format!("{}/source/{slug}/unmeasured", entry.row.identity),
                Disposition::Unknown,
            ),
        )?;
    }
    append_history(entry, slug, rows)
}

fn append_history(
    entry: &ReadJurisdiction,
    slug: &str,
    rows: &mut Vec<SourceObjectOpen>,
) -> Result<(), HandlerError> {
    let Some(window) = entry.state.history_window else {
        return push(
            rows,
            obligation(
                format!("{}/history/{slug}/unmeasured", entry.row.identity),
                Disposition::Unknown,
            ),
        );
    };
    [history::Kind::Meets, history::Kind::Results]
        .into_iter()
        .filter(|kind| kind.requires(slug))
        .try_for_each(|kind| {
            window.years().try_for_each(|year| {
                let (disposition, observations) = history::source(&entry.state, year, slug, kind)?;
                let mut row = obligation(
                    format!(
                        "{}/history/{year}/{}/{slug}",
                        entry.row.identity,
                        kind.name()
                    ),
                    disposition,
                );
                row.observations = observations;
                row.windows = u64::from(disposition.is_complete());
                push(rows, row)
            })
        })
}

#[tracing::instrument(skip_all)]
async fn team_object(
    ctx: &Context<'_>,
    identity: &str,
    slug: &str,
) -> Result<SourceObjectOpen, HandlerError> {
    let key = format!("{identity}/teams/{slug}");
    let mut row = obligation(key.clone(), Disposition::Unknown);
    match ctx
        .object_client::<TeamsSourceClient>(&key)
        .state()
        .call()
        .await
    {
        Ok(Json(Some(TeamsSourceOutcome::Completed { outcome, .. }))) => {
            row.disposition = if outcome.errors.is_empty() && outcome.unfinished.is_empty() {
                outcome.disposition
            } else {
                Disposition::Partial
            };
            row.observations = count(outcome.records)?;
            row.windows = u64::from(row.disposition.is_complete());
        }
        Ok(Json(Some(TeamsSourceOutcome::Terminal { .. }))) => {
            row.disposition = Disposition::Failed
        }
        Ok(Json(Some(TeamsSourceOutcome::Exhausted { .. }))) => {
            row.disposition = Disposition::Exhausted
        }
        Ok(Json(Some(TeamsSourceOutcome::Interrupted { .. }))) => {
            row.disposition = Disposition::Partial
        }
        Ok(Json(None)) => {}
        Err(error) => {
            tracing::warn!(%error, key, "unreadable teams source obligation");
            row.unreadable = true;
        }
    }
    Ok(row)
}
