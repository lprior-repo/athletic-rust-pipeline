use anyhow::{Context, Result};
use census_report::workbook;
use census_service::restate_services::{ExportGeneration, WorkbookReply, WorkbookRequest};
use census_store::Store;

use super::RunArgs;
use crate::cli::{contact_school_year, live, school_year, scope_of};

pub(super) fn publication_options(args: &RunArgs) -> Result<workbook::Options> {
    Ok(workbook::Options {
        grad_year: Some(school_year(args.grad_year)?),
        out: args.out.clone(),
        limit: args.limit,
        scope: scope_of(args.core),
        jurisdictions: Vec::new(),
        school_year: contact_school_year(args.school_year)?,
    })
}

pub(super) fn publish_workbook_offline(store: &Store, options: &workbook::Options) -> Result<()> {
    let path = workbook::build(store, options).context("publishing the managed workbook bundle")?;
    println!("workbook\t{}", path.display());
    Ok(())
}

pub(super) async fn publish_workbook_live(
    origin: &str,
    options: &workbook::Options,
    generation: &ExportGeneration,
) -> Result<()> {
    let request = WorkbookRequest {
        grad_year: options.grad_year,
        out: options
            .out
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        limit: options.limit,
        scope: Some(options.scope.as_str().to_string()),
        school_year: Some(options.school_year.get()),
        jurisdictions: options.jurisdictions.clone(),
    };
    let WorkbookReply { path, .. } = live::workbook(Some(origin), request, generation).await?;
    println!("workbook\t{path}");
    Ok(())
}
