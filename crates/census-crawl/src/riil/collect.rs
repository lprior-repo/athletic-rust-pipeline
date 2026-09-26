//! The adapter body: one GET against the RIIL directory, then map and journal.
//!
//! Network and store access live here and nowhere else in this module.

use super::map::school_entities;
use super::pages::parse_directory;
use super::{Options, ASSOCIATION};
use crate::net::FetchOptions;
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::SourceNamespace;
use census_store::Table;

/// Collect RIIL schools and coaches.
///
/// Single request: one GET to the directory page at `HOST/Directory.aspx`.
/// The page returns all schools in one HTML document, each as a `<details>` block.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new("riil", "schools");
    let observed_on = if options.observed_on.trim().is_empty() {
        ctx.observed_on.clone()
    } else {
        options.observed_on.clone()
    };
    let before = ctx.fetcher.stats().await;

    let url = format!("{}/Directory.aspx", super::HOST);
    let html = match ctx
        .fetcher
        .get(
            &url,
            &FetchOptions {
                refresh: options.refresh,
                ..Default::default()
            },
        )
        .await
    {
        Ok(outcome) if outcome.status == 200 => outcome.text(),
        Ok(outcome) if outcome.status == 404 => {
            report.note("directory page returned 404".to_string());
            return Ok(report);
        }
        Ok(outcome) => {
            report.note(format!("directory page returned HTTP {}", outcome.status));
            return Ok(report);
        }
        Err(e) => {
            report.note(format!("directory page: {}", e));
            return Ok(report);
        }
    };

    let tables = parse_directory(&html);
    let mut tally = Tally::default();

    for table in &tables {
        process_school(ctx, table, &observed_on, &mut report, &mut tally)?;
    }

    let after = ctx.fetcher.stats().await;
    report.requests = after.requests.saturating_sub(before.requests);
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);

    report.note(format!(
        "parsed {} schools with {} coach rows across {} XC/TF rows",
        tally.schools, tally.coach_rows, tally.xc_tf_rows
    ));

    Ok(report)
}

/// Counters for one `collect` run.
#[derive(Default)]
struct Tally {
    schools: usize,
    coach_rows: usize,
    xc_tf_rows: usize,
}

/// Process one school: map to canonical entities, append and journal.
fn process_school(
    ctx: &AdapterContext<'_>,
    table: &super::map::SchoolTable,
    observed_on: &str,
    report: &mut AdapterReport,
    tally: &mut Tally,
) -> CrawlResult<()> {
    let extract = school_entities(table, observed_on);

    let mut batch = ctx.store.write_batch();
    batch.append_many(Table::Schools, std::slice::from_ref(&extract.school))?;
    ctx.observe_school(
        &SourceNamespace::association_school(ASSOCIATION),
        &extract.school,
    )?;
    report.rows = report.rows.saturating_add(1);

    let mut xc_tf_count = 0u64;
    for coach in &extract.coaches {
        tally.coach_rows = tally.coach_rows.saturating_add(1);
        if coach.sport.is_some() {
            tally.xc_tf_rows = tally.xc_tf_rows.saturating_add(1);
            xc_tf_count = xc_tf_count.saturating_add(1);
        }
        batch.append_many(Table::Coaches, std::slice::from_ref(coach))?;
    }

    let school_key = format!("RI:{}", extract.school.id);
    batch.journal_done(
        "riil_schools",
        &school_key,
        &serde_json::json!({
            "name": extract.school.name,
            "coaches": extract.coaches.len(),
            "xc_tf_coaches": xc_tf_count,
        }),
    )?;

    for coach in &extract.coaches {
        batch.journal_done(
            "riil_coaches",
            &school_key,
            &serde_json::json!({
                "coach_name": coach.name,
                "sport": format!("{:?}", coach.sport),
                "gender": format!("{:?}", coach.gender),
                "role": format!("{:?}", coach.role),
            }),
        )?;
    }

    batch.commit()?;
    tally.schools = tally.schools.saturating_add(1);
    Ok(())
}
