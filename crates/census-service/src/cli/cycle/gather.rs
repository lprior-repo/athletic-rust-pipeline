use anyhow::{Context, Result};
use census_domain::model::{
    serialized_digest, AcquisitionDigest, CaptureCacheIntent, CensusRun, GradYear, HistoryWindow,
    ResultsCaptureScopeV1, RunManifest, SchoolYear, SelectedHistoryV1, SourceAdmissionV1,
};
use census_domain::UsJurisdiction;
use census_store::Store;

use super::RunArgs;
use crate::cli::{build_fetcher, Cli};

pub(super) async fn gather_athleticnet(
    cli: &Cli,
    store: &Store,
    args: &RunArgs,
    observed_on: String,
) -> Result<()> {
    let fetcher = build_fetcher(cli, store)?;
    let season =
        SchoolYear::new(2026).ok_or_else(|| anyhow::anyhow!("2026 is not a valid school year"))?;
    let (_durable_root, _durable_store, durable) = durable_capture_context()?;
    let context = census_crawl::AdapterContext {
        capture: census_crawl::CaptureContext::DurableV1(&durable),
        fetcher: &fetcher,
        store,
        refresh: args.refresh,
        school_year: season,
        observed_on: observed_on.clone(),
        performance_as_of: match args.as_of {
            Some(date) => date,
            None => chrono::Utc::now().date_naive(),
        },
        recording: None,
    };
    let report = census_crawl::athleticnet::collect(
        &context,
        &census_crawl::athleticnet::Options {
            input: args.input.clone(),
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            states: args.states.clone(),
            meets: args.meets.clone(),
            event_metadata: args.event_metadata,
            meet_limit: args.meet_limit,
        },
    )
    .await
    .with_context(|| {
        format!(
            "gathering athletic.net rows: input={:?} meets={:?}",
            args.input, args.meets
        )
    })?;
    println!(
        "gather\tathleticnet\trows={} requests={} errors={}",
        report.rows, report.requests, report.errors
    );
    for note in &report.notes {
        println!("\t{note}");
    }
    if report.errors > 0 {
        anyhow::bail!(
            "Athletic.net gathering remains incomplete: {} acquisition or admission errors",
            report.errors
        );
    }
    Ok(())
}

fn durable_capture_context() -> Result<(
    tempfile::TempDir,
    Store,
    census_crawl::ResultsCaptureContext,
)> {
    let root = tempfile::tempdir().context("opening the durable capture scratch dir")?;
    Store::create_capture_qualification(root.path().join("store"))
        .context("creating the durable capture store")?;
    let (store, writer) = Store::open_current_qualification_v3(root.path().join("store"))
        .context("opening the durable capture store")?
        .into_parts();
    store
        .bind_run(&RunManifest {
            store_identity: writer.store_identity().as_str().to_owned(),
            run: CensusRun::new(
                SchoolYear::new(2026).ok_or_else(|| anyhow::anyhow!("2026 school year"))?,
                1,
            )
            .ok_or_else(|| anyhow::anyhow!("census run"))?,
            cohort: GradYear::CO2027,
            jurisdictions: vec![UsJurisdiction::Michigan],
        })
        .context("binding the durable capture run")?;
    let manifest = store
        .run_manifest()
        .context("reading the durable capture manifest")?
        .ok_or_else(|| anyhow::anyhow!("durable capture run manifest"))?;
    let scope =
        ResultsCaptureScopeV1::new(
            "jurisdiction:MI:2026-27:1",
            UsJurisdiction::Michigan,
            manifest.run,
            AcquisitionDigest::parse(&serialized_digest(&manifest).map_err(|error| {
                anyhow::anyhow!("digesting the durable capture manifest: {error}")
            })?)
            .map_err(|error| anyhow::anyhow!("parsing the durable capture digest: {error}"))?,
            SelectedHistoryV1::new(
                HistoryWindow::cohort("2026-10-14")
                    .map_err(|error| anyhow::anyhow!("cohort history window: {error}"))?,
                2026,
            )
            .map_err(|error| anyhow::anyhow!("durable capture history: {error}"))?,
            SourceAdmissionV1::milesplit_results(&"a".repeat(64))
                .map_err(|error| anyhow::anyhow!("durable capture admission: {error}"))?,
            CaptureCacheIntent::ReuseEligible,
        )
        .map_err(|error| anyhow::anyhow!("durable capture scope: {error}"))?;
    let owner = writer
        .results_owner(&store, scope)
        .map_err(|error| anyhow::anyhow!("durable capture owner: {error}"))?;
    let grant = writer
        .continuation_for_owned_meet(&store, &owner, UsJurisdiction::Michigan, "725218")
        .map_err(|error| anyhow::anyhow!("durable capture grant: {error}"))?;
    let capture =
        census_crawl::ResultsCaptureContext::new(owner, grant, UsJurisdiction::Michigan, "725218")
            .map_err(|error| anyhow::anyhow!("durable capture context: {error}"))?;
    Ok((root, store, capture))
}
