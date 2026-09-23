//! Dispatch: route a parsed [`Command`] to the module that owns its logic.
//!
//! Everything here is an offline tool: the caller has already opened the store for it, and the pipeline
//! commands — the ones that submit through the ingress, and the ones that write their own files without
//! a store — were handled before this point. Adding a command to [`super::run`] instead of here is the
//! way to keep it out of a store open it does not want.

use anyhow::Result;
use midwest_census::store::Store;

use super::export_data;
use super::gather;
use super::provider;
use super::publish;
use super::qa_reports;
use super::review;
use super::school_names;
use super::seal;
use super::source;
use super::store;
use super::verify;
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
        Command::Review(args) => review::run_review(store, args).await?,
        Command::Index => publish::run_index(store)?,
        Command::Seal(args) => seal::run_seal(store, args)?,
        Command::Verify(args) => verify::run_verify(store, args)?,
        Command::FjallStats => store::print_store_stats(store)?,
        Command::ImportLegacy => store::run_legacy_import(store)?,
        Command::StoreBackup(args) => store::run_backup(store, args)?,
        Command::StoreRestore(args) => store::run_restore(args)?,
        Command::StoreIntegrity => store::run_integrity(store)?,
        Command::ExportData(args) => export_data::run_export_data(args)?,
        Command::QaReports(args) => qa_reports::run_qa_reports(args)?,
        Command::SchoolNames(args) => school_names::run_school_names(store, args)?,
        // The pipeline commands are matched in `super::run` before the store is opened: they either
        // submit through the ingress or need no store at all.
        other => anyhow::bail!("{other:?} is routed before dispatch and never opens the store"),
    }
    Ok(())
}
