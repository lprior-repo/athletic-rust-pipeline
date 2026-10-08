use super::{
    parse_directory_links, parse_school_details, school_entities, ProfileFacts, SchoolExtract,
    Section, HOST, SECTIONS, SOURCE_ID,
};
use crate::net::{FetchOptions, FetchStats};
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::UsJurisdiction;
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
    let mut report = AdapterReport::new(SOURCE_ID, "schools");
    let before = ctx.fetcher.stats().await;

    let sections = selected_sections(options);
    if sections.is_empty() {
        apply_stats(&mut report, &before, &ctx.fetcher.stats().await);
        let codes: Vec<&str> = options.states.iter().map(|state| state.code()).collect();
        report.note(format!(
            "states {codes:?} do not include CA, FL or NJ; this adapter covers California sections 1-9 and 13, Florida section 10 and New Jersey section 12"
        ));
        return Ok(report);
    }

    let observed_on = if options.observed_on.trim().is_empty() {
        ctx.observed_on.clone()
    } else {
        options.observed_on.clone()
    };

    let refresh = ctx.refresh || options.refresh;
    let mut tally = Tally::default();
    let mut remaining = options.limit.map_or(usize::MAX, |value| value);

    for section in &sections {
        if remaining == 0 {
            break;
        }
        let links =
            fetch_section_links(ctx, section, options, refresh, remaining, &mut report).await?;
        run_section_details(
            ctx,
            section,
            &links,
            refresh,
            &observed_on,
            &mut tally,
            &mut report,
        )
        .await?;
        remaining = remaining.saturating_sub(links.len());
    }

    apply_stats(&mut report, &before, &ctx.fetcher.stats().await);
    report.rows = tally.processed;
    report.with_email = tally.with_email;
    report.note(format!(
        "processed {} schools ({} coach_rows, {} with email, {} errors)",
        tally.processed, tally.coach_rows, tally.with_email, report.errors
    ));

    Ok(report)
}

fn apply_stats(report: &mut AdapterReport, before: &FetchStats, after: &FetchStats) {
    report.requests = after
        .physical_requests()
        .saturating_sub(before.physical_requests());
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
}

fn selected_sections(options: &Options) -> Vec<Section> {
    SECTIONS
        .iter()
        .copied()
        .filter(|section| options.states.is_empty() || options.states.contains(&section.state))
        .collect()
}

async fn fetch_section_links(
    ctx: &AdapterContext<'_>,
    section: &Section,
    options: &Options,
    refresh: bool,
    remaining: usize,
    report: &mut AdapterReport,
) -> CrawlResult<Vec<super::parse::SchoolLink>> {
    let directory_url = format!("{HOST}/widget/school/directory?section={}", section.number);
    let directory_opts = FetchOptions {
        refresh,
        allow_not_found: false,
        headers: Vec::new(),
    };

    let outcome = match ctx.fetcher.get(&directory_url, &directory_opts).await {
        Ok(outcome) => outcome,
        Err(err) => {
            report.errors = report.errors.saturating_add(1);
            report.note(format!(
                "failed to fetch directory section {}: {err}",
                section.number
            ));
            return Ok(Vec::new());
        }
    };

    let html = String::from_utf8_lossy(&outcome.body);
    let links: Vec<super::parse::SchoolLink> = parse_directory_links(&html)
        .into_iter()
        .filter(|link| {
            options.school_names.is_empty()
                || options
                    .school_names
                    .iter()
                    .any(|name| link.name.contains(name))
        })
        .take(remaining)
        .collect();

    report.note(format!(
        "section {} ({}) parsed {} schools",
        section.number,
        section.state.code(),
        links.len()
    ));

    Ok(links)
}

async fn run_section_details(
    ctx: &AdapterContext<'_>,
    section: &Section,
    links: &[super::parse::SchoolLink],
    refresh: bool,
    observed_on: &str,
    tally: &mut Tally,
    report: &mut AdapterReport,
) -> CrawlResult<()> {
    let directory_url = format!("{HOST}/widget/school/directory?section={}", section.number);
    let detail_opts = FetchOptions {
        refresh,
        allow_not_found: false,
        headers: vec![
            ("x-requested-with".to_string(), "XMLHttpRequest".to_string()),
            ("referer".to_string(), directory_url),
        ],
    };

    for link in links {
        if let Some(note) =
            refresh_school(ctx, section, link, &detail_opts, observed_on, tally).await?
        {
            report.errors = report.errors.saturating_add(1);
            report.note(note);
        }
    }

    Ok(())
}

async fn refresh_school(
    ctx: &AdapterContext<'_>,
    section: &Section,
    link: &super::parse::SchoolLink,
    fetch_opts: &FetchOptions,
    observed_on: &str,
    tally: &mut Tally,
) -> CrawlResult<Option<String>> {
    let details_url = format!("{HOST}/widget/get-school-details/{}/details", link.id);

    let outcome = match ctx.fetcher.get(&details_url, fetch_opts).await {
        Ok(outcome) => outcome,
        Err(err) => {
            return Ok(Some(format!(
                "failed to fetch details for {}: {err}",
                link.name
            )));
        }
    };

    let body = String::from_utf8_lossy(&outcome.body);
    let Some(details) = parse_school_details(&body) else {
        return Ok(Some(format!(
            "unparsable details for {} ({details_url})",
            link.name
        )));
    };

    let name = if details.profile.name.trim().is_empty() {
        link.name.clone()
    } else {
        details.profile.name.clone()
    };

    let extract = school_entities(
        &ProfileFacts {
            state: section.state,
            association: section.association,
            name: &name,
            city: &details.profile.city,
            address: &details.profile.address,
            zip: &details.profile.zip,
            league: &details.profile.league,
            phone: &details.profile.phone,
            url: &details_url,
            observed_on,
        },
        &details.coaches,
        &details.faculties,
    );

    emit_school(ctx, &extract, tally)?;
    Ok(None)
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
                association: extract.association.clone(),
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

    let school_key = format!("{}:{}", extract.state.code(), extract.school.id);
    batch.journal_done(
        "home_campus_schools",
        &school_key,
        &serde_json::json!({
            "name": extract.school.name,
            "coaches": extract.coaches.len(),
        }),
    )?;
    for coach in &extract.coaches {
        batch.journal_done(
            "home_campus_coaches",
            &school_key,
            &serde_json::json!({
                "coach_name": coach.name,
                "sport": format!("{:?}", coach.sport),
                "role": format!("{:?}", coach.role),
            }),
        )?;
    }

    batch.commit()?;
    tally.processed = tally.processed.saturating_add(1);

    Ok(())
}
