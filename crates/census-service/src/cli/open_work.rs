use anyhow::Result;
use clap::Args;
use restate_sdk::prelude::Json;

use census_service::restate_services::{
    CensusIngressClient, JurisdictionOpen, OpenWorkReply, OpenWorkRequest,
};

use super::Cli;
use census_service::ingress;

mod archive_growth;
use archive_growth::{compare_archive_activity, print_archive_growth, print_sample_json};

#[derive(Debug, Args)]
#[command(about = "`census-service open-work`")]
pub(super) struct OpenWorkArgs {
    #[arg(
        help = "Ingress origin of the local Restate server. The local census deployment when omitted"
    )]
    #[arg(long, value_name = "ORIGIN")]
    ingress: Option<String>,
    #[arg(help = "Season start year: 2026 is the 2026-27 school year")]
    #[arg(long, default_value_t = 2026)]
    season: i16,
    #[arg(help = "Run revision: the one the run was submitted under, not a new one")]
    #[arg(long, default_value_t = 1)]
    revision: u32,
    #[arg(
        help = "Ingest object key to read, e.g. `milesplit_wi`. Repeatable, because an object key is the caller's to choose and the service cannot enumerate them: naming none leaves the count unmeasured rather than reporting it as zero"
    )]
    #[arg(long = "source-object", value_name = "KEY")]
    source_objects: Vec<String>,
    #[arg(help = "Print the reply as JSON instead of a table")]
    #[arg(long)]
    json: bool,
    #[arg(
        long,
        value_name = "SECS",
        value_parser = clap::value_parser!(u64).range(1..=60),
        help = "Measure net archive artifact growth with two serving reads 1..60 seconds apart"
    )]
    capture_sample_seconds: Option<u64>,
}

#[tracing::instrument(skip_all, fields(command = "open-work"))]
pub(super) async fn run_open_work(cli: &Cli, args: &OpenWorkArgs) -> Result<()> {
    let origin = cli.service_origin("open-work", args.ingress.as_deref())?;
    let census = CensusIngressClient::from_client(ingress::client(origin)?);
    let request = OpenWorkRequest {
        season: args.season,
        revision: args.revision,
        source_objects: args.source_objects.clone(),
    };
    let Some(seconds) = args.capture_sample_seconds else {
        let reply = fetch(&census, request).await?;
        if args.json {
            println!("{}", serde_json::to_string_pretty(&reply)?);
            return Ok(());
        }
        return print_open_work(&reply);
    };
    let first = fetch(&census, request.clone()).await?;
    tokio::time::sleep(std::time::Duration::from_secs(seconds)).await;
    let second = fetch(&census, request).await?;
    let growth = compare_archive_activity(
        &first.archive_capture_activity,
        &second.archive_capture_activity,
    );
    if args.json {
        return print_sample_json(&second, &growth);
    }
    print_open_work(&second)?;
    print_archive_growth(&growth)
}

async fn fetch<Executor: restate_sdk::ingress::RequestExecutor>(
    census: &CensusIngressClient<Executor>,
    request: OpenWorkRequest,
) -> Result<OpenWorkReply> {
    Ok(census
        .open_work(Json(request))
        .call()
        .await
        .map_err(ingress::error)?
        .into_body()
        .map_err(ingress::error)?
        .0)
}

fn print_open_work(reply: &OpenWorkReply) -> Result<()> {
    println!("season {} revision {}", reply.season, reply.revision);
    println!(
        "jurisdiction sweeps owed: {} of {}",
        count(reply.jurisdiction_sweeps),
        reply.jurisdictions.len()
    );
    println!("source objects owed: {}", count(reply.source_objects));
    print_archive_activity(&reply.archive_capture_activity)?;
    for row in &reply.jurisdictions {
        if row.unreadable {
            println!(
                "  {} {}: unreadable, counted as owing",
                row.jurisdiction.code(),
                row.identity
            );
            continue;
        }
        print_diagnostics(&mut std::io::stdout().lock(), row)?;
        let owing = row.stages.owing();
        if owing.is_empty() {
            continue;
        }
        println!(
            "  {} {} owes: {}",
            row.jurisdiction.code(),
            row.identity,
            owing.join(", ")
        );
    }
    for row in &reply.endpoints {
        let unreadable = if row.unreadable { " (unreadable)" } else { "" };
        println!(
            "  {} observations {} windows {} disposition {:?}{unreadable}",
            row.endpoint, row.observations, row.windows, row.disposition
        );
        if let Some(outcome) = &row.teams_source {
            println!("    teams source: {}", serde_json::to_string(outcome)?);
        }
    }
    Ok(())
}

fn print_archive_activity(
    activity: &census_service::restate_services::ArchiveCaptureActivity,
) -> Result<()> {
    use census_service::restate_services::ArchiveCaptureActivity;
    use std::io::Write;
    let mut output = std::io::stdout().lock();
    match activity {
        ArchiveCaptureActivity::Measured(sample) => {
            writeln!(
                output,
                "archive artifacts: {} documents, {} body addresses, {} logical body bytes; sample {} to {}",
                sample.capture_documents,
                sample.unique_body_files,
                sample.unique_body_payload_bytes,
                sample.started_at,
                sample.finished_at,
            )?;
            writeln!(
                output,
                "archive scope: store {}; directory {:?}:{:?}; metric {}; not run-attributed",
                sample.epoch.store_identity,
                sample.epoch.archive_dev,
                sample.epoch.archive_ino,
                sample.epoch.metric_version,
            )?;
        }
        ArchiveCaptureActivity::Unmeasured(sample) => {
            writeln!(
                output,
                "archive artifacts: unmeasured ({:?})",
                sample.reason
            )?;
        }
    }
    Ok(())
}

fn count(value: Option<u64>) -> String {
    value.map_or_else(|| "unmeasured".to_string(), |count| count.to_string())
}

fn print_diagnostics(writer: &mut impl std::io::Write, row: &JurisdictionOpen) -> Result<()> {
    if row.unreadable {
        return Ok(());
    }
    let code = row.jurisdiction.code();
    match &row.teams_stage {
        Some(stage) => writeln!(
            writer,
            "  {code} teams stage: {}",
            serde_json::to_string(stage)?
        )?,
        None => writeln!(writer, "  {code} teams stage: unmeasured")?,
    }
    match &row.source_refusals {
        Some(refusals) => {
            writeln!(writer, "  {code} source refusals: {}", refusals.len())?;
            refusals.iter().try_for_each(|refusal| -> Result<()> {
                writeln!(writer, "    {}", serde_json::to_string(refusal)?)?;
                Ok(())
            })?;
        }
        None => writeln!(writer, "  {code} source refusals: unmeasured")?,
    }
    Ok(())
}

#[cfg(test)]
#[path = "open_work/tests.rs"]
mod tests;
