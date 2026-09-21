use crate::model::{normalize_name, CanonicalCoach, SchoolId};
use crate::net::FetchOptions;
use crate::sources::{AdapterContext, AdapterReport};
use crate::store::Table;
use anyhow::{bail, Context, Result};
use serde_json::json;
use std::collections::HashSet;

use super::map::{
    ad_coaches, ad_role, coach_entities, provider_key, school_domains, school_entities,
};
use super::parse::{
    listing_page_url, parse_next_listing_page, parse_school_detail, parse_school_list,
    school_page_url,
};
use super::teams::{parse_coach_records, parse_team_nodes, select_team_nodes, TeamCoaches};
use super::{Options, COACH_API_PREFIX, MAX_LISTING_PAGES, SOURCE_ID, TEAMS_VIEW_URL};

/// Fetch one school's team nodes and their coach records.
///
/// Returns the entities plus notes for surfaces that were unavailable; the caller has already written the
/// AD rows, so a failure here never discards work.
async fn collect_team_coaches(
    ctx: &AdapterContext<'_>,
    fetch_options: &FetchOptions,
    school_key: &str,
    school_id: &SchoolId,
    domains: &[String],
    observed_on: &str,
) -> (Vec<CanonicalCoach>, Vec<String>) {
    let teams_url = format!(
        "{TEAMS_VIEW_URL}?views-argument%5B%5D={school_key}&fields%5Bnode--participant%5D=title,path,drupal_internal__nid"
    );
    let outcome = match ctx.fetcher.get(&teams_url, fetch_options).await {
        Ok(outcome) => outcome,
        Err(error) => {
            return (
                Vec::new(),
                vec![format!("team list {teams_url}: {error:#}")],
            )
        }
    };
    let selected = select_team_nodes(&parse_team_nodes(&outcome.text()));
    let mut teams: Vec<TeamCoaches> = Vec::with_capacity(selected.len());
    let mut notes: Vec<String> = Vec::new();
    for node in selected {
        let api_url = format!("{COACH_API_PREFIX}{}", node.nid);
        match ctx.fetcher.get(&api_url, fetch_options).await {
            Ok(outcome) => teams.push(TeamCoaches {
                node,
                api_url,
                records: parse_coach_records(&outcome.text()),
            }),
            Err(error) => notes.push(format!("coach list {api_url}: {error:#}")),
        }
    }
    (
        coach_entities(&teams, school_id, domains, observed_on),
        notes,
    )
}

/// Fetch options for this run: the run-level refresh flag or the adapter's own.
fn fetch_options(ctx: &AdapterContext<'_>, options: &Options) -> FetchOptions {
    FetchOptions {
        refresh: options.refresh || ctx.refresh,
        allow_not_found: false,
        headers: Vec::new(),
    }
}

/// `u64` view of a `usize` count: lossless on every supported target, saturating otherwise.
fn count(value: usize) -> u64 {
    // `clippy::manual_unwrap_or` (a `-D warnings` error) requires this over a `match`, and the
    // fallback is unreachable on every supported target.
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// Collect this provider's schools and coach/AD contacts into the canonical store.
///
/// Per school: one school page (facts + AD rows), one team-node list and up to
/// `MAX_TEAMS_PER_SCHOOL` coach requests. Progress is journalled per school, so a re-run resumes
/// without re-fetching finished schools.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> Result<AdapterReport> {
    let mut report = AdapterReport::new(SOURCE_ID, "schools");
    let stats_before = ctx.fetcher.stats().await;
    if !options.states.is_empty()
        && !options
            .states
            .iter()
            .any(|state| state.trim().eq_ignore_ascii_case("MN"))
    {
        report.note("MSHSL covers Minnesota only; requested states do not include MN, so nothing was fetched");
        return Ok(report);
    }
    let wanted: HashSet<String> = options
        .school_names
        .iter()
        .map(|name| normalize_name(name))
        .filter(|name| !name.is_empty())
        .collect();
    let done = ctx.store.journal_keys("mshsl_schools")?;
    let fetch = fetch_options(ctx, options);
    let mut processed = 0usize;
    let mut skipped = 0usize;
    let mut unparsed = 0usize;
    let mut ad_rows = 0usize;
    let mut coach_rows = 0usize;
    let mut with_email = 0u64;
    let mut office_roles = 0usize;
    let mut page = 0usize;
    'pages: while page < MAX_LISTING_PAGES {
        let url = listing_page_url(page);
        let outcome = ctx
            .fetcher
            .get(&url, &fetch)
            .await
            .with_context(|| format!("fetching MSHSL school listing page {page}"))?;
        let html = outcome.text();
        let rows = parse_school_list(&html);
        if rows.is_empty() {
            if page == 0 {
                bail!("MSHSL school listing {url} contained no school rows (markup change or empty page)");
            }
            report.note(format!(
                "listing page {url} contained no school rows; pagination stopped"
            ));
            break;
        }
        for row in rows {
            if options.limit.is_some_and(|limit| processed >= limit) {
                break 'pages;
            }
            if !wanted.is_empty() && !wanted.contains(&normalize_name(&row.name)) {
                continue;
            }
            let key = format!("MN:{}", row.slug);
            if done.contains(&key) {
                skipped = skipped.saturating_add(1);
                continue;
            }
            let page_url = school_page_url(&row.slug);
            let detail = match ctx.fetcher.get(&page_url, &fetch).await {
                Ok(outcome) => parse_school_detail(&outcome.text()),
                Err(error) => {
                    report.note(format!("school page {page_url}: {error:#}"));
                    continue;
                }
            };
            let Some((school, school_id)) =
                school_entities(&row, &detail, &page_url, &options.observed_on)
            else {
                unparsed = unparsed.saturating_add(1);
                report.note(format!(
                    "school page {page_url}: no school name in listing row or page"
                ));
                continue;
            };
            let school_key = provider_key(&row, &detail);
            let ads = ad_coaches(
                &detail,
                &school_id,
                &school_key,
                &page_url,
                &options.observed_on,
            );
            let domains = school_domains(&detail);
            office_roles = office_roles.saturating_add(
                detail
                    .admin
                    .iter()
                    .filter(|entry| ad_role(&entry.role).is_none())
                    .count(),
            );
            ctx.store.append(Table::Schools, &school)?;
            ctx.store.append_many(Table::Coaches, &ads)?;
            let (sport_coaches, notes) = match detail.school_id.as_deref() {
                Some(id) => {
                    collect_team_coaches(
                        ctx,
                        &fetch,
                        id,
                        &school_id,
                        &domains,
                        &options.observed_on,
                    )
                    .await
                }
                None => (
                    Vec::new(),
                    vec![format!(
                        "school page {page_url}: no /group/<id>/ link, sport coaches skipped"
                    )],
                ),
            };
            ctx.store.append_many(Table::Coaches, &sport_coaches)?;
            for note in notes {
                report.note(format!("{}: {note}", school.name));
            }
            let school_with_email = ads
                .iter()
                .chain(sport_coaches.iter())
                .filter(|coach| coach.professional_email.is_some())
                .count();
            ad_rows = ad_rows.saturating_add(ads.len());
            coach_rows = coach_rows.saturating_add(sport_coaches.len());
            with_email = with_email.saturating_add(count(school_with_email));
            ctx.store.journal_done(
                "mshsl_schools",
                &key,
                &json!({
                    "school": school.name,
                    "school_id": school_key,
                    "page_url": page_url,
                    "city": row.city,
                    "ad_rows": ads.len(),
                    "sport_coach_rows": sport_coaches.len(),
                    "with_email": school_with_email,
                    "observed_on": options.observed_on,
                }),
            )?;
            ctx.store.journal_done(
                "mshsl_coaches",
                &key,
                &json!({
                    "school_id": school_key,
                    "ad_rows": ads.len(),
                    "sport_coach_rows": sport_coaches.len(),
                    "with_email": school_with_email,
                    "observed_on": options.observed_on,
                }),
            )?;
            processed = processed.saturating_add(1);
        }
        match parse_next_listing_page(&html, page) {
            Some(next) => page = next,
            None => break,
        }
    }
    let stats_after = ctx.fetcher.stats().await;
    report.rows = count(processed);
    report.requests = stats_after.requests.saturating_sub(stats_before.requests);
    report.from_cache = stats_after
        .cache_hits
        .saturating_sub(stats_before.cache_hits);
    report.errors = stats_after
        .errors
        .saturating_sub(stats_before.errors)
        .saturating_add(count(unparsed));
    report.with_email = with_email;
    report.note(format!(
        "{processed} school(s) processed ({skipped} already journalled): {ad_rows} athletic-director row(s), {coach_rows} sport-coach row(s), {with_email} with a professional email"
    ));
    report.note(format!(
        "{office_roles} Administration-block entry/entries were office roles (principal, superintendent, AD administrative assistant, trainer, advisors, Title IX, sports representatives) and were not emitted"
    ));
    report.note(
        "AD contacts come from the school page Administration block (Cloudflare-obfuscated addresses decoded locally); sport coaches come from /api/coaches/<team nid> reached through /jsonapi/views/teams/list_school, filtered to MSHSL coach levels and school-domain addresses - personal-domain addresses and every phone column are never parsed",
    );
    Ok(report)
}
