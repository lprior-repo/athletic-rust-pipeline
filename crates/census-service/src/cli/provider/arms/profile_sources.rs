use std::collections::{BTreeMap, HashMap};

use anyhow::{Context, Result};
use census_crawl::{self as providers, AdapterContext, AdapterReport};
use census_domain::model::{CanonicalAthlete, CanonicalSchool, GradYear};
use census_domain::UsJurisdiction;
use census_store::{StoreSnapshot, Table};

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
    let rosters: Vec<(UsJurisdiction, &Vec<providers::milesplit::ProfilePrTarget>)> = states
        .iter()
        .filter_map(|state| {
            let targets = grouped.get(state)?;
            (!targets.is_empty()).then_some((*state, targets))
        })
        .collect();
    let depth = rosters
        .iter()
        .map(|(_, targets)| targets.len())
        .max()
        .unwrap_or(0);
    let mut pool: Vec<providers::milesplit::ProfilePrTarget> = Vec::new();
    for index in 0..depth {
        for (_, targets) in &rosters {
            if let Some(target) = targets.get(index) {
                pool.push(target.clone());
            }
        }
    }
    let report = collect_mixed_profiles(context, pool).await?;
    let mut merged = AdapterReport::new("milesplit_profiles", "profile PR rows");
    merged.rows = report.rows;
    merged.requests = report.requests;
    merged.from_cache = report.from_cache;
    merged.errors = report.errors;
    merged.disposition = report.disposition;
    merged.note(format!("states={}", states.len()));
    for note in report.notes {
        merged.note(note);
    }
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

async fn collect_mixed_profiles(
    context: &AdapterContext<'_>,
    targets: Vec<providers::milesplit::ProfilePrTarget>,
) -> Result<AdapterReport> {
    providers::milesplit::collect_profile_prs(
        context,
        &providers::milesplit::ProfilePrOptions { targets },
    )
    .await
    .with_context(|| "collecting pooled profile PRs".to_string())
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
