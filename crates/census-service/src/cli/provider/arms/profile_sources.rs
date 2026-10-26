use std::collections::{BTreeMap, HashMap};

use anyhow::{Context, Result};
use census_crawl::{self as providers, AdapterContext, AdapterReport, CollectionDisposition};
use census_domain::model::{CanonicalAthlete, CanonicalSchool, GradYear};
use census_domain::UsJurisdiction;
use census_store::{StoreSnapshot, Table};
use futures::stream::{self, StreamExt};

use super::super::ProviderArgs;

pub(crate) async fn milesplit_profiles_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
) -> Result<AdapterReport> {
    let states = crate::cli::resolve_states(args.all_states, &args.states)?;
    let cohort = grad_year(args.grad_year)?;
    let snapshot = context.store.snapshot();
    let grouped = group_targets(&snapshot, &states, cohort, args.limit)
        .with_context(|| "enumerating profile targets".to_string())?;
    let lanes = context.fetcher.family_parallelism().max(1);
    let grouped = &grouped;
    let mut per_state: Vec<(UsJurisdiction, Result<AdapterReport>)> =
        stream::iter(states.iter().copied())
            .map(|state| async move {
                let targets = grouped.get(&state).cloned().unwrap_or_default();
                let outcome = collect_state_profiles(context, state, targets).await;
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

#[tracing::instrument(skip(context, targets), fields(state = state.code()))]
async fn collect_state_profiles(
    context: &AdapterContext<'_>,
    state: UsJurisdiction,
    targets: Vec<providers::milesplit::ProfilePrTarget>,
) -> Result<AdapterReport> {
    providers::milesplit::collect_profile_prs(
        context,
        &providers::milesplit::ProfilePrOptions { targets },
    )
    .await
    .with_context(|| format!("collecting {} profile PRs", state.code()))
}

fn group_targets(
    snapshot: &StoreSnapshot<'_>,
    states: &[UsJurisdiction],
    cohort: GradYear,
    limit: Option<usize>,
) -> Result<BTreeMap<UsJurisdiction, Vec<providers::milesplit::ProfilePrTarget>>> {
    let wanted: HashMap<&str, UsJurisdiction> =
        states.iter().map(|state| (state.code(), *state)).collect();
    let schools: HashMap<String, UsJurisdiction> = snapshot
        .scan::<CanonicalSchool>(Table::Schools)?
        .into_iter()
        .filter_map(|school| {
            school
                .state
                .map(|state| (school.id.as_str().to_owned(), state))
        })
        .collect();
    let mut grouped: BTreeMap<UsJurisdiction, Vec<providers::milesplit::ProfilePrTarget>> =
        BTreeMap::new();
    let mut skipped = 0_usize;
    for athlete in snapshot.athletes()? {
        if athlete.grad_year != cohort {
            continue;
        }
        let Some((state, url)) = placement(&athlete, &schools) else {
            skipped = skipped.saturating_add(1);
            continue;
        };
        if !wanted.contains_key(state.code()) {
            continue;
        }
        let targets = grouped.entry(state).or_default();
        if limit.is_some_and(|cap| targets.len() >= cap) {
            continue;
        }
        targets.push(providers::milesplit::ProfilePrTarget {
            athlete_id: athlete.source.as_ref().map_or_else(
                || athlete.id.as_str().to_owned(),
                |source| source.id.clone(),
            ),
            profile_url: url,
            site: state,
        });
    }
    if skipped > 0 {
        tracing::warn!("profile targets without a resolvable state or URL: {skipped}");
    }
    Ok(grouped)
}

fn placement(
    athlete: &CanonicalAthlete,
    schools: &HashMap<String, UsJurisdiction>,
) -> Option<(UsJurisdiction, String)> {
    let mut fallback: Option<String> = None;
    for url in &athlete.public_profile_urls {
        let Some(host) = host_of(url) else {
            continue;
        };
        if !host.ends_with("milesplit.com") {
            continue;
        }
        let code = host.split('.').next().unwrap_or(host);
        if code.eq_ignore_ascii_case("www") {
            if fallback.is_none() {
                fallback = Some(url.clone());
            }
            continue;
        }
        if let Some(state) = UsJurisdiction::from_code(code) {
            return Some((state, url.clone()));
        }
    }
    let url = fallback.or_else(|| athlete.public_profile_urls.first().cloned())?;
    let state = schools.get(athlete.school.as_str()).copied()?;
    Some((state, url))
}

fn host_of(url: &str) -> Option<&str> {
    url.split("://").nth(1)?.split('/').next()
}
