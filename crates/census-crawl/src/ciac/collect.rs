//! The adapter body: fetch the CIAC directory page, parse and map.
//!
//! Network and store access live here and nowhere else in this module.

use super::map::SchoolExtract;
use super::search::resolve_schools;
use super::{Options, ASSOCIATION};
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::SourceNamespace;
use census_domain::UsJurisdiction;
use census_store::Table;

/// Collect CIAC schools and coaches.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new("ciac", "schools");

    let before = ctx.fetcher.stats().await;

    if !options.states.is_empty() && !options.states.contains(&UsJurisdiction::Connecticut) {
        let after = ctx.fetcher.stats().await;
        report.requests = after.requests.saturating_sub(before.requests);
        report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
        let codes: Vec<&str> = options.states.iter().map(|state| state.code()).collect();
        report.note(format!(
            "states {codes:?} do not include CT; this adapter covers Connecticut only"
        ));
        return Ok(report);
    }

    let to_process = resolve_schools(ctx, options, &mut report).await?;
    let mut tally = Tally::default();

    for extract in &to_process {
        emit_school(ctx, extract, &mut tally)?;
    }

    let after = ctx.fetcher.stats().await;
    report.requests = after.requests.saturating_sub(before.requests);
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);

    report.rows = tally.processed;
    report.note(format!(
        "processed {} schools ({} coach_rows)",
        tally.processed, tally.coach_rows
    ));

    Ok(report)
}

/// Counters for one `collect` run.
#[derive(Default)]
struct Tally {
    processed: u64,
    coach_rows: usize,
}

/// Append and journal one school's rows, tallying its coach count.
///
/// The journal key is the same `CT:<id>` the school's rows carry, so a re-run reads the completed
/// schools from the log instead of trusting the page's own ordering.
fn emit_school(
    ctx: &AdapterContext<'_>,
    extract: &SchoolExtract,
    tally: &mut Tally,
) -> CrawlResult<()> {
    let mut batch = ctx.write_batch();
    batch.append_many(Table::Schools, std::slice::from_ref(&extract.school))?;
    ctx.observe_school(
        &SourceNamespace::AssociationSchool {
            association: ASSOCIATION.to_string(),
        },
        &extract.school,
    )?;

    for coach in &extract.coaches {
        tally.coach_rows = tally.coach_rows.saturating_add(1);
        batch.append_many(Table::Coaches, std::slice::from_ref(coach))?;
    }

    let school_key = format!("CT:{}", extract.school.id);
    batch.journal_done(
        "ciac_schools",
        &school_key,
        &serde_json::json!({
            "name": extract.school.name,
            "coaches": extract.coaches.len(),
        }),
    )?;
    for coach in &extract.coaches {
        batch.journal_done(
            "ciac_coaches",
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
