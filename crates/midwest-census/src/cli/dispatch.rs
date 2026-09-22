//! Dispatch: route a parsed [`Command`] to the module that owns its logic.

use anyhow::Result;
use midwest_census::store::Store;

use super::census_doc;
use super::cycle;
use super::export_data;
use super::gather;
use super::merge_coaches;
use super::national;
use super::provider;
use super::publish;
use super::qa_reports;
use super::review;
use super::school_names;
use super::seal;
use super::serve;
use super::store;
use super::verify;
use super::Cli;
use super::{resolve_states, Command};

pub(super) async fn dispatch(cli: &Cli, store: &Store) -> Result<()> {
    match &cli.command {
        Command::Sites => gather::run_sites()?,
        Command::Fetch { url, refresh } => gather::run_fetch(cli, store, url, *refresh).await?,
        Command::Teams {
            states,
            all_states,
            refresh,
        } => {
            let states = resolve_states(*all_states, states)?;
            gather::run_teams(cli, store, &states, *refresh).await?
        }
        Command::Meets {
            states,
            all_states,
            year,
            refresh,
        } => {
            let states = resolve_states(*all_states, states)?;
            gather::run_meets(cli, store, &states, *year, *refresh).await?
        }
        Command::Collect(args) => gather::run_collect(cli, store, args).await?,
        Command::ImportCoaches { csv, observed_on } => {
            gather::run_import_coaches(store, csv, observed_on)?
        }
        Command::Provider(args) => provider::run_provider(cli, store, args).await?,
        Command::Consolidate => publish::run_consolidate(store)?,
        Command::Review(args) => review::run_review(store, args).await?,
        Command::Index => publish::run_index(store)?,
        Command::Report { print, core } => publish::run_report(store, *print, *core)?,
        Command::Bests(args) => publish::run_bests(store, args)?,
        Command::Workbook(args) => publish::run_workbook(store, args)?,
        Command::Seal(args) => seal::run_seal(store, args)?,
        Command::Verify(args) => verify::run_verify(store, args)?,
        Command::Run(args) => cycle::run_cycle(cli, store, args).await?,
        Command::FjallStats => store::print_store_stats(store)?,
        Command::ImportLegacy => store::run_legacy_import(store)?,
        Command::StoreBackup(args) => store::run_backup(store, args)?,
        Command::StoreRestore(args) => store::run_restore(args)?,
        Command::StoreIntegrity => store::run_integrity(store)?,
        Command::Serve => serve::run_serve(cli)?,
        Command::National(args) => national::run_national(args).await?,
        Command::Jurisdiction(args) => national::run_jurisdiction(args).await?,
        Command::NationalReport(args) => national::run_national_report(args).await?,
        Command::ExportData(args) => export_data::run_export_data(args)?,
        Command::QaReports(args) => qa_reports::run_qa_reports(args)?,
        Command::MergeCoaches(args) => merge_coaches::run_merge_coaches(args)?,
        Command::SchoolNames(args) => school_names::run_school_names(store, args)?,
        Command::CensusDoc(args) => census_doc::run_census_doc(args)?,
        Command::VerifyCoaches(_) => {
            anyhow::bail!("VerifyCoaches is handled in run() before dispatch")
        }
    }
    Ok(())
}
