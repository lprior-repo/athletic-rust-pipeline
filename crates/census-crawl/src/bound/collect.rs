use super::SOURCE_ID;
use crate::net::FetchOptions;
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{Gender, Sport};
use census_domain::UsJurisdiction;
use census_store::Table;

pub(super) mod emit;
pub(super) mod index;

use emit::{emit_school, process_school, SchoolJob};

const BOUND_UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36 census-service/0.1";

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    pub states: Vec<UsJurisdiction>,
    pub school_names: Vec<String>,
}

struct Bound {
    state: UsJurisdiction,
    association: &'static str,
    state_code: &'static str,
}

const BOUND: [Bound; 2] = [
    Bound {
        state: UsJurisdiction::Iowa,
        association: "ighsau",
        state_code: "ia",
    },
    Bound {
        state: UsJurisdiction::SouthDakota,
        association: "sdhsaa",
        state_code: "sd",
    },
];

fn sport_entries(association: &str) -> Vec<(&str, Sport, Gender)> {
    let mut entries = Vec::new();
    if association == "ighsau" {
        entries.push(("boystrack", Sport::OutdoorTrack, Gender::Boys));
        entries.push(("girlstrack", Sport::OutdoorTrack, Gender::Girls));
        entries.push(("boyscrosscountry", Sport::CrossCountry, Gender::Boys));
        entries.push(("girlscrosscountry", Sport::CrossCountry, Gender::Girls));
    } else {
        entries.push(("boystrackfield", Sport::OutdoorTrack, Gender::Boys));
        entries.push(("girlstrackfield", Sport::OutdoorTrack, Gender::Girls));
        entries.push(("boyscrosscountry", Sport::CrossCountry, Gender::Boys));
        entries.push(("girlscrosscountry", Sport::CrossCountry, Gender::Girls));
    }
    entries
}

fn bound_fetch_options(ctx: &AdapterContext<'_>) -> FetchOptions {
    FetchOptions {
        refresh: ctx.refresh,
        allow_not_found: false,
        headers: vec![("user-agent".to_string(), BOUND_UA.to_string())],
    }
}

fn bound_fetch_options_permissive(ctx: &AdapterContext<'_>) -> FetchOptions {
    FetchOptions {
        refresh: ctx.refresh,
        allow_not_found: true,
        headers: vec![("user-agent".to_string(), BOUND_UA.to_string())],
    }
}

#[derive(Default)]
struct Totals {
    schools: usize,
    matched: usize,
    ambiguous: usize,
    unmatched: usize,
}

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new(SOURCE_ID, "schools");
    let before = ctx.fetcher.stats().await;

    let observed_on = if options.observed_on.is_empty() {
        ctx.observed_on.clone()
    } else {
        options.observed_on.clone()
    };

    let mut totals = Totals::default();
    for source in &BOUND {
        if !options.states.is_empty() && !options.states.contains(&source.state) {
            continue;
        }
        walk_state(ctx, source, options, &observed_on, &mut totals, &mut report).await?;
    }

    let after = ctx.fetcher.stats().await;
    report.requests = after.requests.saturating_sub(before.requests);
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);

    Ok(report)
}

async fn walk_state(
    ctx: &AdapterContext<'_>,
    source: &Bound,
    options: &Options,
    observed_on: &str,
    totals: &mut Totals,
    report: &mut AdapterReport,
) -> CrawlResult<()> {
    let index = index::fetch_school_index(ctx, source.state_code).await?;
    let index_count = index.len();

    let schools = load_schools(ctx, source.state, options)?;
    totals.schools = totals.schools.saturating_add(schools.len());

    let slugified = index::resolve_slugs(schools, &index);
    totals.matched = totals.matched.saturating_add(slugified.matched.len());
    totals.ambiguous = totals.ambiguous.saturating_add(slugified.ambiguous.len());
    totals.unmatched = totals.unmatched.saturating_add(slugified.unmatched.len());

    if !slugified.ambiguous.is_empty() {
        report.note(format!(
            "ambiguous Bound matches for {}: {}",
            source.state.code(),
            slugified.ambiguous.join(", ")
        ));
    }
    if !slugified.unmatched.is_empty() {
        report.note(format!(
            "no Bound match for {}: {}",
            source.state.code(),
            slugified.unmatched.join(", ")
        ));
    }

    let entries = sport_entries(source.association);
    let matched = match options.limit {
        Some(limit) => slugified.matched.into_iter().take(limit).collect(),
        None => slugified.matched,
    };

    let (school_tally, coach_tally) =
        sweep(ctx, source, &matched, &entries, observed_on, report).await?;

    report.rows = report
        .rows
        .saturating_add(u64::try_from(school_tally).map_or(u64::MAX, |value| value));

    report.note(format!(
        "{}: index {index_count} schools, matched {} of {} (ambiguous {}, unmatched {}); processed {school_tally} ({coach_tally} coaches)",
        source.state.code(),
        totals.matched,
        totals.schools,
        totals.ambiguous,
        totals.unmatched
    ));

    Ok(())
}

async fn sweep(
    ctx: &AdapterContext<'_>,
    source: &Bound,
    matched: &[(String, String)],
    entries: &[(&str, Sport, Gender)],
    observed_on: &str,
    report: &mut AdapterReport,
) -> CrawlResult<(usize, usize)> {
    let mut school_tally = 0usize;
    let mut coach_tally = 0usize;

    for (school_name, slug) in matched {
        let job = SchoolJob {
            name: school_name.as_str(),
            slug: slug.as_str(),
            state_code: source.state_code,
            state: source.state,
        };
        let extract = process_school(ctx, &job, source.association, entries, observed_on).await?;

        let Some(extract) = extract else {
            report.note(format!(
                "no Bound staff page for {} {school_name}",
                source.state.code()
            ));
            continue;
        };

        school_tally = school_tally.saturating_add(1);
        emit_school(ctx, &extract, &mut coach_tally)?;
    }

    Ok((school_tally, coach_tally))
}

pub(super) fn load_schools(
    ctx: &AdapterContext<'_>,
    state: UsJurisdiction,
    options: &Options,
) -> CrawlResult<Vec<String>> {
    if !options.school_names.is_empty() {
        return Ok(options.school_names.clone());
    }

    let mut schools = Vec::new();
    let scan = ctx
        .store
        .scan::<census_domain::model::CanonicalSchool>(Table::Schools)
        .map_err(CrawlError::Store)?;

    for school in scan {
        if school.state == Some(state) {
            schools.push(school.name);
        }
    }

    Ok(schools)
}
