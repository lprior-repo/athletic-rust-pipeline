use anyhow::Result;
use census_store::Store;

use super::export_data;
use super::gather;
use super::provider;
use super::publish;
use super::qa_reports;
use super::school_names;
use super::source;
use super::store;
use super::Cli;
use super::Command;

pub(super) async fn dispatch(cli: &Cli, store: &Store) -> Result<()> {
    match &cli.command {
        Command::Sites => source::run_sites()?,
        Command::Fetch { url, refresh } => source::run_fetch(cli, store, url, *refresh).await?,
        Command::ImportCoaches { csv, observed_on } => {
            gather::run_import_coaches(store, csv, observed_on)?
        }
        Command::Provider(args) => provider::run_provider(cli, store, args).await?,
        Command::Consolidate => publish::run_consolidate(store)?,
        Command::Index => publish::run_index(store)?,
        Command::FjallStats => store::print_store_stats(store)?,
        Command::StoreRestore(args) => store::run_restore(args)?,
        Command::StoreIntegrity => store::run_integrity(store)?,
        Command::ExportData(args) => export_data::run_export_data(store, args)?,
        Command::QaReports(args) => qa_reports::run_qa_reports(args)?,
        Command::SchoolNames(args) => school_names::run_school_names(store, args)?,
        other => anyhow::bail!("{other:?} is routed before dispatch and never opens the store"),
    }
    Ok(())
}
