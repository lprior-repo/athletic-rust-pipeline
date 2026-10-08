pub mod map;
pub mod parse;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use crate::net::FetchOptions;
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::UsJurisdiction;
use census_store::Table;

pub use map::{school_entities, ProfileFacts, SchoolExtract};
pub use parse::{parse_staff_directory, StaffDirectory, StaffRow};

pub const HOST: &str = "https://gomats.org";
pub const DIRECTORY_URL: &str = "https://gomats.org/staff-directory";
pub const SOURCE_ID: &str = "sidearm_staff";
pub const STATE: UsJurisdiction = UsJurisdiction::California;

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
    if !options.states.is_empty() && !options.states.contains(&STATE) {
        report.note("this adapter covers only the verified gomats.org school in CA");
        return Ok(report);
    }
    if options.limit == Some(0) {
        return Ok(report);
    }
    let before = ctx.fetcher.stats().await;
    let observed_on = if options.observed_on.trim().is_empty() {
        ctx.observed_on.as_str()
    } else {
        options.observed_on.as_str()
    };
    refresh_school(ctx, options, observed_on, &mut report).await?;
    let after = ctx.fetcher.stats().await;
    report.requests = after
        .physical_requests()
        .saturating_sub(before.physical_requests());
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    Ok(report)
}

async fn refresh_school(
    ctx: &AdapterContext<'_>,
    options: &Options,
    observed_on: &str,
    report: &mut AdapterReport,
) -> CrawlResult<()> {
    let fetch_options = FetchOptions {
        refresh: ctx.refresh || options.refresh,
        allow_not_found: false,
        headers: Vec::new(),
    };
    let outcome = match ctx.fetcher.get(DIRECTORY_URL, &fetch_options).await {
        Ok(outcome) => outcome,
        Err(error) => {
            report.errors = report.errors.saturating_add(1);
            report.note(format!("failed to fetch {DIRECTORY_URL}: {error}"));
            return Ok(());
        }
    };
    let html = String::from_utf8_lossy(&outcome.body);
    let directory = match parse_staff_directory(&html) {
        Ok(directory) => directory,
        Err(error) => {
            report.errors = report.errors.saturating_add(1);
            report.note(format!("failed to parse {DIRECTORY_URL}: {error}"));
            return Ok(());
        }
    };
    if !options.school_names.is_empty()
        && !options
            .school_names
            .iter()
            .any(|name| directory.name.contains(name))
    {
        return Ok(());
    }
    let extract = school_entities(
        &ProfileFacts {
            state: STATE,
            host: HOST,
            url: DIRECTORY_URL,
            observed_on,
        },
        &directory,
    );
    emit_school(ctx, &extract, observed_on)?;
    report.rows = 1;
    report.with_email = extract
        .coaches
        .iter()
        .filter(|coach| coach.has_published_email())
        .fold(0u64, |count, _| count.saturating_add(1));
    report.note(format!(
        "processed {} ({} coach_rows, {} with email); verified host only: gomats.org",
        extract.school.name,
        extract.coaches.len(),
        report.with_email,
    ));
    Ok(())
}

fn emit_school(
    ctx: &AdapterContext<'_>,
    extract: &SchoolExtract,
    observed_on: &str,
) -> CrawlResult<()> {
    let mut batch = ctx.write_batch();
    batch.append_many(Table::Schools, std::slice::from_ref(&extract.school))?;
    batch.append_many(
        Table::SourceObservations,
        crate::school_observations_of(
            &map::namespace(extract.state),
            std::slice::from_ref(&extract.school),
            observed_on,
        )
        .as_slice(),
    )?;
    batch.append_many(Table::Coaches, &extract.coaches)?;
    let school_key = format!("{}:{}", extract.state.code(), extract.school_id);
    batch.journal_done(
        "sidearm_staff_schools",
        &school_key,
        &serde_json::json!({ "name": extract.school.name, "coaches": extract.coaches.len() }),
    )?;
    extract.coaches.iter().try_for_each(|coach| {
        batch.journal_done(
            "sidearm_staff_coaches",
            &school_key,
            &serde_json::json!({
                "coach_name": coach.name,
                "sport": format!("{:?}", coach.sport),
                "role": format!("{:?}", coach.role),
            }),
        )
    })?;
    batch.commit()?;
    Ok(())
}
