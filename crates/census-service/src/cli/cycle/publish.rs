//! One cycle stage per transport: the publishing half of `run`, against the store this process
//! opened and against the running service.
//!
//! These live apart from `run_cycle` because they are the part that speaks to the two transports;
//! the run itself (gather, consolidate, index) stays in the parent module.

use anyhow::{Context, Result};
use census_store::Store;
use census_report::report;
use census_service::restate_services::{BestsReply, WorkbookReply, WorkbookRequest};
use census_report::{bests, workbook};

use super::RunArgs;
use crate::cli::cohort_label;
use crate::cli::live;

/// Build one scope's census, write its JSON and CSV, and print the stage's lines.
pub(super) fn publish_scope(store: &Store, scope: report::Scope) -> Result<()> {
    let census = report::build_census(store, scope).context("building the census")?;
    let (json_path, csv_path) = report::write_census(store, &census, scope)?;
    println!(
        "report\t{}\tscope={} schools={} athletes={} profile_url={} multisource={}",
        json_path.display(),
        census.scope,
        census.totals.schools,
        census.totals.athletes,
        census.totals.class_of_2027_with_profile_url,
        census.totals.class_of_2027_multisource
    );
    println!("\t{}", csv_path.display());
    Ok(())
}

/// Reduce the best marks, write them with their workbook, and print the stage's lines.
pub(super) fn publish_bests_and_workbook(
    store: &Store,
    args: &RunArgs,
    scope: report::Scope,
    grad_year: i16,
) -> Result<()> {
    let bests = bests::Options {
        scope,
        grad_year: Some(grad_year),
        limit: args.limit,
    };
    let rows = bests::build(store, &bests).context("reducing the best marks")?;
    let cohort = cohort_label(Some(grad_year));
    let (jsonl, csv) = bests::write(store, &rows, &cohort).context("writing the best marks")?;
    println!(
        "bests\tcohort={cohort} rows={} scope={}\t{}",
        rows.len(),
        if args.all_sources { "all" } else { "core" },
        jsonl.display()
    );
    println!("\t{}", csv.display());

    let workbook = workbook::Options {
        grad_year: Some(grad_year),
        out: args.out.clone(),
        limit: args.limit,
        scope,
    };
    let path = workbook::build(store, &workbook).context("building the census workbook")?;
    println!("workbook\t{}", path.display());
    Ok(())
}

/// Build one scope's census in the running service and print the stage's lines.
pub(super) async fn publish_scope_live(origin: &str, scope: report::Scope) -> Result<()> {
    let summary = live::report(Some(origin), scope).await?;
    println!(
        "report\t{}\tscope={} schools={} athletes={} profile_url={} multisource={}",
        summary.json_path,
        summary.scope,
        summary.total("schools")?,
        summary.total("athletes")?,
        summary.total("class_of_2027_with_profile_url")?,
        summary.total("class_of_2027_multisource")?
    );
    println!("\t{}", summary.csv_path);
    Ok(())
}

/// Reduce the best marks, write them with their workbook, and print the stage's lines.
pub(super) async fn publish_bests_and_workbook_live(
    origin: &str,
    args: &RunArgs,
    scope: report::Scope,
    grad_year: i16,
) -> Result<()> {
    let BestsReply {
        cohort,
        rows,
        jsonl,
        csv,
    } = live::bests(Some(origin), scope, Some(grad_year), args.limit).await?;
    println!(
        "bests\tcohort={cohort} rows={rows} scope={}\t{jsonl}",
        if args.all_sources { "all" } else { "core" }
    );
    println!("\t{csv}");

    let workbook = WorkbookRequest {
        grad_year: Some(grad_year),
        out: args
            .out
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        limit: args.limit,
        scope: Some(scope.as_str().to_string()),
    };
    let WorkbookReply { path, .. } = live::workbook(Some(origin), workbook).await?;
    println!("workbook\t{path}");
    Ok(())
}
