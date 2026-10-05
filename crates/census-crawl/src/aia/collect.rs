use super::map::{school_entities, SchoolExtract};
use super::parse::{parse_school_profile, parse_search_json, ParsedRow};
use super::{Options, ASSOCIATION, HOST};
use crate::net::FetchOptions;
use crate::recording::RowBatch;
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::SourceNamespace;
use census_domain::UsJurisdiction;
use census_store::Table;

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new("aia", "schools");
    let before = ctx.fetcher.stats().await;

    if !options.states.is_empty() && !options.states.contains(&UsJurisdiction::Arizona) {
        let after = ctx.fetcher.stats().await;
        report.requests = after.requests.saturating_sub(before.requests);
        report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
        let codes: Vec<&str> = options.states.iter().map(|state| state.code()).collect();
        report.note(format!(
            "states {codes:?} do not include AZ; this adapter covers Arizona only"
        ));
        return Ok(report);
    }

    let observed_on = if options.observed_on.trim().is_empty() {
        ctx.observed_on.clone()
    } else {
        options.observed_on.clone()
    };

    let schools = if options.school_names.is_empty() {
        resolve_via_city_searches(ctx, options).await?
    } else {
        resolve_named_schools(ctx, options).await?
    };

    let mut tally = Tally::default();
    for school in &schools {
        match emit_school(ctx, school, &observed_on, &mut tally).await {
            Ok(()) => {}
            Err(error) => {
                report.errors = report.errors.saturating_add(1);
                report.note(format!("school {}: {error}", school.name));
            }
        }
    }

    let after = ctx.fetcher.stats().await;
    report.requests = after.requests.saturating_sub(before.requests);
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    report.rows = tally.processed;
    report.note(format!(
        "processed {} schools ({} coach_rows); the search API returns at most 10 results per query, so this collection is a sample, not the full member list",
        tally.processed, tally.coach_rows
    ));

    Ok(report)
}

#[derive(Default)]
struct Tally {
    processed: u64,
    coach_rows: usize,
}

fn search_url(query: &str) -> CrawlResult<String> {
    let mut url = url::Url::parse(&format!("{HOST}/schools/search.json")).map_err(|error| {
        CrawlError::Schema {
            url: format!("{HOST}/schools/search.json"),
            detail: error.to_string(),
        }
    })?;
    url.query_pairs_mut().append_pair("q", query);
    Ok(url.into())
}

async fn fetch_body(ctx: &AdapterContext<'_>, url: &str, refresh: bool) -> CrawlResult<String> {
    let options = FetchOptions {
        refresh: ctx.refresh || refresh,
        allow_not_found: false,
        headers: Vec::new(),
    };
    let outcome = ctx.fetcher.get(url, &options).await?;
    Ok(String::from_utf8_lossy(&outcome.body).into_owned())
}

async fn search_schools(
    ctx: &AdapterContext<'_>,
    options: &Options,
    query: &str,
) -> CrawlResult<Vec<ParsedRow>> {
    let url = search_url(query)?;
    let body = fetch_body(ctx, &url, options.refresh).await?;
    parse_search_json(&body).map_err(|error| CrawlError::Schema {
        url,
        detail: error.to_string(),
    })
}

async fn resolve_named_schools(
    ctx: &AdapterContext<'_>,
    options: &Options,
) -> CrawlResult<Vec<ParsedRow>> {
    let limit = options.limit.map_or(usize::MAX, |value| value);
    let mut found: Vec<ParsedRow> = Vec::new();
    for name in &options.school_names {
        let rows = search_schools(ctx, options, name).await?;
        if let Some(row) = rows
            .into_iter()
            .find(|row| row.name.to_lowercase().contains(&name.to_lowercase()))
        {
            found.push(row);
        }
        if found.len() >= limit {
            found.truncate(limit);
            break;
        }
    }
    Ok(found)
}

async fn resolve_via_city_searches(
    ctx: &AdapterContext<'_>,
    options: &Options,
) -> CrawlResult<Vec<ParsedRow>> {
    let cities = [
        "phoenix",
        "tucson",
        "mesa",
        "chandler",
        "scottsdale",
        "glendale",
        "gilbert",
        "tempe",
        "peoria",
        "surprise",
        "yuma",
        "flagstaff",
        "avondale",
        "casa grande",
        "maricopa",
        "prescott",
    ];
    let limit = options.limit.map_or(usize::MAX, |value| value);
    let mut found: Vec<ParsedRow> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for city in cities {
        let rows = search_schools(ctx, options, city).await?;
        for row in rows {
            if seen.insert(row.school_id.clone()) {
                found.push(row);
            }
        }
        if found.len() >= limit {
            found.truncate(limit);
            break;
        }
    }
    Ok(found)
}

async fn emit_school(
    ctx: &AdapterContext<'_>,
    school: &ParsedRow,
    observed_on: &str,
    tally: &mut Tally,
) -> CrawlResult<()> {
    let url = format!("{HOST}/schools/{}", school.school_id);
    let body = fetch_body(ctx, &url, false).await?;
    let profile = parse_school_profile(&body).map_err(|error| CrawlError::Schema {
        url: url.clone(),
        detail: error.to_string(),
    })?;

    let extract = school_entities(
        &school.name,
        &school.school_id,
        school.city.as_deref(),
        &profile,
        observed_on,
    );
    let namespace = SourceNamespace::AssociationSchool {
        association: ASSOCIATION.to_string(),
    };

    let mut batch = ctx.write_batch();
    batch.append_many(Table::Schools, std::slice::from_ref(&extract.school))?;
    batch.append_many(
        Table::SourceObservations,
        ctx.school_observation(&namespace, &extract.school)
            .as_slice(),
    )?;
    for coach in &extract.coaches {
        batch.append_many(Table::Coaches, std::slice::from_ref(coach))?;
    }

    let school_key = format!("AZ:{}", extract.school.id);
    journal_school(&mut batch, &school_key, &extract)?;
    batch.commit()?;

    tally.processed = tally.processed.saturating_add(1);
    tally.coach_rows = tally.coach_rows.saturating_add(extract.coaches.len());
    Ok(())
}

fn journal_school(
    batch: &mut RowBatch<'_>,
    school_key: &str,
    extract: &SchoolExtract,
) -> CrawlResult<()> {
    batch.journal_done(
        "aia_schools",
        school_key,
        &serde_json::json!({
            "name": extract.school.name,
            "coaches": extract.coaches.len(),
        }),
    )?;
    for coach in &extract.coaches {
        batch.journal_done(
            "aia_coaches",
            school_key,
            &serde_json::json!({
                "coach_name": coach.name,
                "sport": format!("{:?}", coach.sport),
                "gender": format!("{:?}", coach.gender),
            }),
        )?;
    }
    Ok(())
}
