pub mod map;
pub mod parse;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use census_domain::UsJurisdiction;

pub use map::school_entities;
pub use map::ProfileFacts;
pub use map::SchoolExtract;
pub use parse::parse_coaches_from_profile;
pub use parse::parse_directory_links;
pub use parse::parse_school_profile;

pub const HOST: &str = "https://uhsaa.org";
pub const SOURCE_ID: &str = "uhsaa";
pub const ASSOCIATION: &str = "uhsaa";
pub const STATE: UsJurisdiction = UsJurisdiction::Utah;

use crate::net::FetchOptions;
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_store::Table;

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    pub states: Vec<UsJurisdiction>,
    pub school_names: Vec<String>,
}

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new("uhsaa", "schools");
    let before = ctx.fetcher.stats().await;

    if !options.states.is_empty() && !options.states.contains(&UsJurisdiction::Utah) {
        let after = ctx.fetcher.stats().await;
        report.requests = after.requests.saturating_sub(before.requests);
        report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
        let codes: Vec<&str> = options.states.iter().map(|state| state.code()).collect();
        report.note(format!(
            "states {codes:?} do not include UT; this adapter covers Utah only"
        ));
        return Ok(report);
    }

    let url = format!("{HOST}/school-directory-new/");
    let fetch_opts = FetchOptions {
        refresh: ctx.refresh || options.refresh,
        allow_not_found: false,
        headers: Vec::new(),
    };

    let outcome = ctx.fetcher.get(&url, &fetch_opts).await?;
    let html = String::from_utf8_lossy(&outcome.body);

    let links = parse_directory_links(&html);
    let link_count = links.len();

    let observed_on = if options.observed_on.trim().is_empty() {
        ctx.observed_on.clone()
    } else {
        options.observed_on.clone()
    };

    let to_process = select_links(links, options);
    report.note(format!(
        "parsed {link_count} school links from directory (processing {})",
        to_process.len()
    ));

    let mut tally = Tally::default();

    for link in &to_process {
        if let Some(note) = refresh_school(ctx, link, &fetch_opts, &observed_on, &mut tally).await?
        {
            report.errors = report.errors.saturating_add(1);
            report.note(note);
        }
    }

    let after = ctx.fetcher.stats().await;
    report.requests = after.requests.saturating_sub(before.requests);
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);

    report.rows = tally.processed;
    report.with_email = tally.with_email;
    report.note(format!(
        "processed {} schools ({} coach_rows, {} with email)",
        tally.processed, tally.coach_rows, tally.with_email
    ));

    Ok(report)
}

fn select_links(links: Vec<parse::SchoolLink>, options: &Options) -> Vec<parse::SchoolLink> {
    let limit = options.limit.map_or(usize::MAX, |value| value);
    links
        .into_iter()
        .filter(|link| {
            options.school_names.is_empty()
                || options
                    .school_names
                    .iter()
                    .any(|name| link.name.contains(name))
        })
        .take(limit)
        .collect()
}

async fn refresh_school(
    ctx: &AdapterContext<'_>,
    link: &parse::SchoolLink,
    fetch_opts: &FetchOptions,
    observed_on: &str,
    tally: &mut Tally,
) -> CrawlResult<Option<String>> {
    let profile_url = profile_url(link);

    let profile_outcome = match ctx.fetcher.get(&profile_url, fetch_opts).await {
        Ok(outcome) => outcome,
        Err(err) => {
            return Ok(Some(format!(
                "failed to fetch profile for {}: {err}",
                link.name
            )));
        }
    };

    let html = String::from_utf8_lossy(&profile_outcome.body);
    let mut profile = parse_school_profile(&html);

    if profile.name.is_empty() {
        profile.name = link.name.clone();
    }

    let extract = school_entities(
        &ProfileFacts {
            name: &profile.name,
            address: &profile.address,
            district: &profile.district,
            classification: &profile.classification,
            region: &profile.region,
            url: &profile_url,
            observed_on,
        },
        &profile.coaches,
    );

    emit_school(ctx, &extract, tally)?;
    Ok(None)
}

fn profile_url(link: &parse::SchoolLink) -> String {
    if link.url.starts_with("http") {
        link.url.clone()
    } else {
        let relative = match link.url.strip_prefix("../") {
            Some(stripped) => stripped,
            None => link.url.as_str(),
        };
        format!("{HOST}/{relative}")
    }
}

#[derive(Default)]
struct Tally {
    processed: u64,
    coach_rows: usize,
    with_email: u64,
}

fn emit_school(
    ctx: &AdapterContext<'_>,
    extract: &SchoolExtract,
    tally: &mut Tally,
) -> CrawlResult<()> {
    let mut batch = ctx.write_batch();
    batch.append_many(Table::Schools, std::slice::from_ref(&extract.school))?;
    batch.append_many(
        Table::SourceObservations,
        ctx.school_observation(
            &census_domain::model::SourceNamespace::AssociationSchool {
                association: ASSOCIATION.to_string(),
            },
            &extract.school,
        )
        .as_slice(),
    )?;

    for coach in &extract.coaches {
        tally.coach_rows = tally.coach_rows.saturating_add(1);
        if coach.has_published_email() {
            tally.with_email = tally.with_email.saturating_add(1);
        }
        batch.append_many(Table::Coaches, std::slice::from_ref(coach))?;
    }

    let school_key = format!("UT:{}", extract.school.id);
    batch.journal_done(
        "uhsaa_schools",
        &school_key,
        &serde_json::json!({
            "name": extract.school.name,
            "coaches": extract.coaches.len(),
        }),
    )?;
    for coach in &extract.coaches {
        batch.journal_done(
            "uhsaa_coaches",
            &school_key,
            &serde_json::json!({
                "coach_name": coach.name,
                "sport": format!("{:?}", coach.sport),
                "gender": format!("{:?}", coach.gender),
            }),
        )?;
    }

    batch.commit()?;
    tally.processed = tally.processed.saturating_add(1);

    Ok(())
}
