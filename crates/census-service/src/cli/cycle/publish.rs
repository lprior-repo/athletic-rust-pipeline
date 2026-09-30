use anyhow::{Context, Result};
use census_report::export::ExportDataset;
use census_report::report::{self, Census, Derivation};
use census_report::{bests, workbook};
use census_service::restate_services::{BestsReply, WorkbookReply, WorkbookRequest};
use census_store::Store;

use super::RunArgs;
use crate::cli::cohort_label;
use crate::cli::live;

pub(super) fn publish_scope_with(
    dataset: &ExportDataset,
    store: &Store,
    scope: report::Scope,
) -> Result<Census> {
    let derivation = Derivation::of(dataset, scope, None);
    let census = report::build_census(&derivation, &store.out_dir());
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
    Ok(census)
}

pub(super) fn publish_bests_and_workbook_with(
    dataset: &ExportDataset,
    store: &Store,
    args: &RunArgs,
    scope: report::Scope,
    grad_year: i16,
    censuses: &workbook::Censuses,
) -> Result<()> {
    let bests = bests::Options {
        scope,
        grad_year: Some(grad_year),
        limit: args.limit,
    };
    let rows = bests::build_from_dataset(dataset, &bests);
    let cohort = cohort_label(Some(grad_year));
    let (jsonl, csv) =
        bests::write(&store.out_dir(), &rows, &cohort).context("writing the best marks")?;
    println!(
        "bests\tcohort={cohort} rows={} scope={}\t{}",
        rows.len(),
        scope.as_str(),
        jsonl.display()
    );
    println!("\t{}", csv.display());

    let workbook = workbook::Options {
        grad_year: Some(grad_year),
        out: args.out.clone(),
        limit: args.limit,
        scope,
        school_year: None,
    };
    let path = workbook::build_from(dataset, store, &workbook, censuses)
        .context("building the census workbook")?;
    println!("workbook\t{}", path.display());
    Ok(())
}

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
        scope.as_str()
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
