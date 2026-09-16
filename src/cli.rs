mod args;
mod transport;

use anyhow::{bail, Context, Result};
use args::{Cli, Command, Start};
use athletic_rust_pipeline::{
    domain::identity::{EvidenceDigest, WorkbookDigest},
    runtime::{
        control::{PipelineControlIngressClient, PrepareRequest},
        export::EXPORT_HEADERS,
        export_worker::{ExportRequest, ExportWorkerIngressClient},
        identity::fingerprint,
        import::ImportRequest,
        run::RunCoordinatorIngressClient,
        run_protocol::Selection,
        worker,
    },
};
use clap::Parser;
use restate_sdk::prelude::*;
use std::io::Write;

pub async fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Worker { config, bind } => worker::serve(&config, bind).await,
        Command::Deploy { admin, endpoint } => emit(&transport::deploy(&admin, &endpoint).await?),
        Command::Start(args) => start(args).await,
        Command::Status { ingress, run } => {
            let client =
                RunCoordinatorIngressClient::from_client(transport::client(&ingress)?, "global");
            let response = client
                .status(Json(EvidenceDigest::parse(&run)?))
                .call()
                .await?
                .into_body()?;
            emit(&response.0)
        }
        Command::Export {
            ingress,
            run,
            output,
        } => {
            let request = ExportRequest {
                run: EvidenceDigest::parse(&run)?,
                destination: destination(output)?,
            };
            let key = request.key()?;
            let request_key = fingerprint(&("native-export-v1", &request))?;
            let client = ExportWorkerIngressClient::from_client(transport::client(&ingress)?, &key);
            let published = client
                .publish(Json(request))
                .idempotency_key(request_key.as_str())
                .call()
                .await?
                .into_body()?
                .0;
            emit(&published)
        }
        Command::Verify {
            input,
            output,
            sha256,
        } => {
            let digest = WorkbookDigest::parse(&sha256)?;
            let report = tokio::task::spawn_blocking(move || {
                let headers = EXPORT_HEADERS
                    .iter()
                    .map(|header| (*header).to_owned())
                    .collect::<Vec<_>>();
                athletic_rust_pipeline::workbook_verify::verify_fields(
                    &input, &output, &digest, &headers,
                )
            })
            .await
            .context("joining independent field verification")??;
            emit(
                &serde_json::json!({"scope": "source_fields_and_hash_only", "verification": report}),
            )
        }
    }
}

async fn start(args: Start) -> Result<()> {
    let selection = match (args.per_sheet, args.all) {
        (Some(rows), false) => Selection::PerSheet { rows },
        (None, true) => Selection::All,
        (Some(_), true) | (None, false) => bail!("choose exactly one of --per-sheet or --all"),
    };
    let source = ImportRequest {
        original: args
            .input
            .canonicalize()
            .context("canonicalizing source workbook")?,
        workbook: WorkbookDigest::parse(&args.sha256)?,
    };
    let request = PrepareRequest {
        source,
        selection,
        concurrency: args.concurrency,
        snapshot_label: args.snapshot,
        execution: args.execution,
    };
    let preparation_key = fingerprint(&("native-prepare-v1", &request))?;
    let client = transport::client(&args.ingress)?;
    let control = PipelineControlIngressClient::from_client(client.clone());
    let prepared = control
        .prepare(Json(request))
        .idempotency_key(preparation_key.as_str())
        .call()
        .await?
        .into_body()?
        .0;
    let key = prepared.key()?;
    let coordinator = RunCoordinatorIngressClient::from_client(client, "global");
    let sent = coordinator
        .run(Json(prepared))
        .idempotency_key(&key)
        .send()
        .await?;
    emit(
        &serde_json::json!({"run": key, "invocation": sent.invocation_handle().invocation_id(),
        "state": "submitted", "note": "Submission is not completion; inspect status and verified export."}),
    )
}

fn destination(path: std::path::PathBuf) -> Result<std::path::PathBuf> {
    let name = path
        .file_name()
        .context("export destination must name a file")?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    Ok(parent
        .canonicalize()
        .context("canonicalizing export parent")?
        .join(name))
}

fn emit(value: &impl serde::Serialize) -> Result<()> {
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer_pretty(&mut stdout, value)?;
    stdout.write_all(b"\n")?;
    Ok(())
}
