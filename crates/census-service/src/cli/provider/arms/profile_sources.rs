use anyhow::{Context, Result};
use census_crawl::{self as providers, AdapterContext, AdapterReport, CollectionDisposition};
use census_domain::model::GradYear;
use census_domain::UsJurisdiction;
use futures::stream::{self, StreamExt};

use super::super::ProviderArgs;
use census_service::census::profiles::{self, Cursor, Selection};

pub(crate) async fn milesplit_profiles_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
) -> Result<AdapterReport> {
    let states = crate::cli::resolve_states(args.all_states, &args.states)?;
    let cohort = grad_year(args.grad_year)?;
    let lanes = context.fetcher.family_parallelism().max(1);
    let mut per_state: Vec<(UsJurisdiction, Result<AdapterReport>)> =
        stream::iter(states.iter().copied())
            .map(|state| async move {
                let outcome = collect_state_profiles(context, args, state, cohort).await;
                (state, outcome)
            })
            .buffer_unordered(lanes)
            .collect()
            .await;
    per_state.sort_by(|left, right| left.0.code().cmp(right.0.code()));
    let mut merged = AdapterReport::new("milesplit_profiles", "profile PR rows");
    let mut complete = true;
    for (state, outcome) in per_state {
        match outcome {
            Ok(report) => {
                complete = complete && report.disposition.is_complete();
                merged.rows = merged.rows.saturating_add(report.rows);
                merged.requests = merged.requests.saturating_add(report.requests);
                merged.from_cache = merged.from_cache.saturating_add(report.from_cache);
                merged.errors = merged.errors.saturating_add(report.errors);
                for note in report.notes {
                    merged.note(format!("state {}: {note}", state.code()));
                }
            }
            Err(error) => {
                complete = false;
                merged.errors = merged.errors.saturating_add(1);
                merged.note(format!("state {} failed: {error:#}", state.code()));
            }
        }
    }
    merged.disposition = if complete {
        CollectionDisposition::Complete
    } else {
        CollectionDisposition::Partial
    };
    merged.note(format!("states={}", states.len()));
    Ok(merged)
}

fn grad_year(raw: Option<i16>) -> Result<GradYear> {
    match raw {
        Some(year) => {
            GradYear::new(year).with_context(|| format!("--grad-year {year} is not a year"))
        }
        None => Ok(GradYear::CO2027),
    }
}

#[tracing::instrument(skip(context, args), fields(state = state.code()))]
async fn collect_state_profiles(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    state: UsJurisdiction,
    cohort: GradYear,
) -> Result<AdapterReport> {
    let (targets, skipped) = enumerate_targets(context, args, state, cohort)?;
    let mut report = providers::milesplit::collect_profile_prs(
        context,
        &providers::milesplit::ProfilePrOptions { targets },
    )
    .await
    .with_context(|| format!("collecting {} profile PRs", state.code()))?;
    if skipped > 0 {
        report.note(format!("unresolvable_profile_url={skipped}"));
    }
    Ok(report)
}

fn enumerate_targets(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    state: UsJurisdiction,
    cohort: GradYear,
) -> Result<(Vec<providers::milesplit::ProfilePrTarget>, usize)> {
    let mut targets = Vec::new();
    let mut skipped = 0_usize;
    let mut cursor: Option<Cursor> = None;
    loop {
        if args.limit.is_some_and(|limit| targets.len() >= limit) {
            break;
        }
        match profiles::select(context.store, state, cohort, cursor.as_ref())
            .with_context(|| format!("selecting {} profile targets", state.code()))?
        {
            Selection::Profile(found) => {
                cursor = Some(found.next.clone());
                match profiles::profile_url(context.store, &found.packet, &found.athlete_id) {
                    Ok(url) => targets.push(providers::milesplit::ProfilePrTarget {
                        athlete_id: found.athlete_id,
                        profile_url: url,
                        site: state,
                    }),
                    Err(_) => skipped = skipped.saturating_add(1),
                }
            }
            Selection::Advance { next, .. } => cursor = Some(next),
            Selection::End => break,
        }
    }
    Ok((targets, skipped))
}
