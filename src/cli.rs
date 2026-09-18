mod args;
mod flow_control;
mod transport;

use anyhow::{bail, Context, Result};
use args::{Cli, Command, Start};
use athletic_rust_pipeline::{
    domain::identity::{EvidenceDigest, WorkbookDigest},
    runtime::{
        control::{PipelineControlIngressClient, PrepareRequest},
        export::EXPORT_HEADERS,
        export_worker::{ExportRequest, ExportWorkerIngressClient, PublishedExport},
        identity::fingerprint,
        import::ImportRequest,
        run::RunCoordinatorIngressClient,
        run_protocol::{preparation_key, Selection},
        worker,
    },
    store::ArtifactStore,
};
use clap::Parser;
use futures::{StreamExt, TryStreamExt};
use restate_sdk::ingress::{ClientError, InvocationHandle, Output};
use restate_sdk::prelude::*;
use std::{fs, io::Write, path::Path, time::Duration};

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
            let request_key = fingerprint(&("native-export-v4", &request))?;
            let client = ExportWorkerIngressClient::from_client(transport::client(&ingress)?, &key);
            let submitted = client
                .publish(Json(request))
                .idempotency_key(request_key.as_str())
                .send()
                .await?;
            emit(&await_export(submitted.invocation_handle()).await?)
        }
        Command::Verify {
            input,
            output,
            sha256,
            store,
        } => {
            let digest = WorkbookDigest::parse(&sha256)?;
            let report = tokio::task::spawn_blocking(move || {
                let artifact_store = existing_stopped_store(&store)?;
                let headers = EXPORT_HEADERS
                    .iter()
                    .map(|header| (*header).to_owned())
                    .collect::<Vec<_>>();
                athletic_rust_pipeline::bundle_verify::verify_bundle(
                    &input,
                    &output,
                    &digest,
                    &headers,
                    &artifact_store,
                )
            })
            .await
            .context("joining independent bundle verification")??;
            emit(
                &serde_json::json!({"scope": "source_preservation_and_retained_result_evidence_consistency", "verification": report}),
            )
        }
    }
}

async fn await_export(
    invocation: InvocationHandle<reqwest::Client, Json<PublishedExport>>,
) -> Result<PublishedExport> {
    let handle = &invocation;
    let observations = futures::stream::iter(0..86_400)
        .then(move |_| async move {
            let output = match handle.output().await?.into_body() {
                Output::Ready(Json(published)) => Some(published),
                Output::NotReady => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    None
                }
            };
            Ok::<_, ClientError>(output)
        })
        .try_filter_map(|output| futures::future::ready(Ok(output)));
    futures::pin_mut!(observations);
    let published = tokio::time::timeout(Duration::from_secs(86_400), observations.try_next())
        .await
        .with_context(|| {
            format!(
                "export observation timed out; invocation {} was not cancelled; rerun the same export command to reattach",
                invocation.invocation_id()
            )
        })?
        .with_context(|| format!("reading export invocation {}", invocation.invocation_id()))?;
    published.with_context(|| {
        format!(
            "export observation limit reached; invocation {} was not cancelled; rerun the same export command to reattach",
            invocation.invocation_id()
        )
    })
}

fn existing_stopped_store(path: &Path) -> Result<ArtifactStore> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("opening existing artifact store {}", path.display()))?;
    if !metadata.file_type().is_dir() {
        bail!("artifact store path is not a directory");
    }
    // Pinned Fjall 3 chooses recovery versus creation from this marker.
    let marker = fs::symlink_metadata(path.join("version"))
        .context("existing Fjall version marker is required; refusing to create a store")?;
    if !marker.file_type().is_file() {
        bail!("artifact store version marker is not a regular file");
    }
    ArtifactStore::open(path)
        .context("opening stopped artifact store (an active worker store is refused)")
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
    let preparation_key = preparation_key(&request)?;
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
