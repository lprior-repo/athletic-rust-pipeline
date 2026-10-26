use super::{
    browser_session, capture_protocol, census_doc, cycle, dispatch, gather, init_tracing,
    installation, local_adjudication, merge_coaches, national, nces_address_accept, open_work,
    profile, publish, retained_capture, retained_events, retained_marks, review,
    school_address_join, school_address_preflight, school_sites, seal, serve, store, verify,
    verify_coaches, Cli, Command,
};
use anyhow::Result;
use census_store::Store;
use clap::Parser;

pub(crate) fn run() -> Result<()> {
    init_tracing();
    let cli = Cli::parse();
    if let Command::CompareReference(args) = &cli.command {
        super::compare_reference::run(args);
    }
    if let Command::CaptureProtocol(args) = &cli.command {
        return capture_protocol::run(&cli, args);
    }
    if let Command::Installation(args) = &cli.command {
        return installation::run(&cli, args);
    }
    if let Command::Profile(args) = &cli.command {
        return profile::run(&cli, args);
    }
    if let Command::VerifyRetainedCapture(args) = &cli.command {
        return retained_capture::run(&cli, args);
    }
    if let Command::LocalIdentityAdjudication(args) = &cli.command {
        return local_adjudication::run(&cli, args);
    }
    if let Command::SelectedPublication(args) = &cli.command {
        return super::selected_publication::run(&cli, args);
    }
    if let Command::NcesAddressAccept(args) = &cli.command {
        return nces_address_accept::run(&cli, args);
    }
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(run_async(cli))
}

fn open_offline_store(root: &std::path::Path) -> Result<Store> {
    match Store::open(root) {
        Ok(store) => Ok(store),
        Err(census_store::StoreError::PublicIdentity(
            census_store::PublicIdentityError::OptInRequired,
        )) => {
            let (store, _writer) = Store::open_public_identity_qualification_v3(root)?.into_parts();
            Ok(store)
        }
        Err(error) => Err(error.into()),
    }
}

async fn run_async(cli: Cli) -> Result<()> {
    if let Command::Review(args) = &cli.command {
        args.validate_configuration()?;
    }
    if let Command::Provider(args) = &cli.command {
        args.validate_offline_store(cli.store.as_deref())?;
    }
    match &cli.command {
        Command::SelectedPublication(_)
        | Command::CompareReference(_)
        | Command::CaptureProtocol(_)
        | Command::Installation(_)
        | Command::Profile(_)
        | Command::LocalIdentityAdjudication(_)
        | Command::VerifyRetainedCapture(_)
        | Command::NcesAddressAccept(_) => {
            anyhow::bail!("offline diagnostic commands are routed before the runtime starts")
        }
        Command::Review(args) => {
            let prepared = args.prepare()?;
            let store = open_offline_store(&cli.store_root())?;
            review::run_review(&store, &prepared).await
        }
        Command::National(args) => national::run_national(&cli, args).await,
        Command::Jurisdiction(args) => national::run_jurisdiction(&cli, args).await,
        Command::NationalReport(args) => national::run_national_report(&cli, args).await,
        Command::SchoolAddressPreflight(args) => school_address_preflight::run(args),
        Command::OpenWork(args) => open_work::run_open_work(&cli, args).await,
        Command::BrowserSession(args) => browser_session::run_browser_session(&cli, args).await,
        Command::Seal(args) => seal::run_seal(&cli, args).await,
        Command::Teams(args) => gather::run_teams(&cli, args).await,
        Command::Meets(args) => gather::run_meets(&cli, args).await,
        Command::Collect(args) => gather::run_collect(&cli, args).await,
        Command::Report(args) => publish::report::run_census_report(&cli, args).await,
        Command::Bests(args) => publish::bests::run_bests(&cli, args).await,
        Command::Workbook(args) => publish::workbook::run_workbook(&cli, args).await,
        Command::Run(args) => cycle::run_cycle(&cli, args).await,
        Command::MergeCoaches(args) => merge_coaches::run_merge_coaches(args),
        Command::VerifyCoaches(args) => verify_coaches::run_verify_coaches(args).await,
        Command::CensusDoc(args) => census_doc::run_census_doc(args),
        Command::StoreRestore(args) => store::run_restore(args),
        Command::SchoolAddress(args) => census_service::school_address::run(args),
        Command::StoreBackup(args) => store::run_backup(&cli.store_root(), args),
        Command::StoreIntegrity => store::run_integrity(&cli.store_root()),
        Command::StoreMigrate => store::run_migrate(&cli.store_root()),
        Command::Verify(args) => verify::run_verify(&cli.store_root(), args),
        Command::Serve(args) => serve::run_serve(&cli, args).await,
        Command::RepairRetainedMarks(args) => retained_marks::run_repair_retained_marks(&cli, args),
        Command::RepairRetainedEvents(args) => {
            retained_events::run_repair_retained_events(&cli, args)
        }
        Command::SchoolAddressJoin(args) => school_address_join::run(&cli, args),
        Command::SchoolSites(args) => school_sites::run(&cli, args).await,
        Command::Index(_) | Command::FjallStats => {
            let store = open_offline_store(&cli.store_root())?;
            dispatch::dispatch(&cli, &store).await
        }
        Command::Provider(_) => {
            let store = open_offline_store(&cli.store_root())?;
            dispatch::dispatch(&cli, &store).await
        }
        _ => {
            let store = Store::open(cli.store_root())?;
            dispatch::dispatch(&cli, &store).await
        }
    }
}
