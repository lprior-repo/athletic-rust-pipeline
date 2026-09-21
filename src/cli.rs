mod args;
mod flow_control;
mod transport;

use anyhow::{bail, Context, Result};
use args::{Cli, Command, Start};
use athletic_rust_pipeline::{
    domain::identity::{EvidenceDigest, WorkbookDigest},
    runtime::{
        browser_session::{BrowserSessionIngressClient, ReadinessRequest, BROWSER_SESSION_KEY},
        control::{
            PipelineControlIngressClient, PrepareRequest, RankingControlsInput, RunAndExportRequest,
        },
        export::EXPORT_HEADERS,
        export_worker::{ExportRequest, ExportWorkerIngressClient, PublishedExport},
        identity::fingerprint,
        import::ImportRequest,
        rankings::{RankingsScope, SeasonKind},
        run::RunCoordinatorIngressClient,
        run_protocol::{preparation_key, RunRequest, Selection},
        worker,
    },
    store::ArtifactStore,
};
use clap::Parser;
use futures::{StreamExt, TryStreamExt};
use restate_sdk::ingress::{InvocationHandle, Output};
use restate_sdk::prelude::*;
use std::{fs, io::Write, path::Path, time::Duration};

pub async fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Worker { config, bind } => worker::serve(&config, bind).await,
        Command::Deploy { admin, endpoint } => emit(&transport::deploy(&admin, &endpoint).await?),
        Command::Start(args) => start(args).await,
        Command::Status { ingress, run } => run_status(&ingress, &run).await,
        Command::RankingsStatus { ingress, run } => rankings_status(&ingress, &run).await,
        Command::RankingsPause { ingress, run } => rankings_pause(&ingress, &run).await,
        Command::RankingsResume { ingress, run } => rankings_resume(&ingress, &run).await,
        Command::BrowserStart { ingress } => browser_start(&ingress).await,
        Command::BrowserStatus { ingress } => browser_status(&ingress).await,
        Command::Export {
            ingress,
            run,
            output,
        } => export_run(&ingress, &run, output).await,
        Command::Verify {
            input,
            output,
            sha256,
            store,
        } => verify_retained_bundle(input, output, &sha256, store).await,
    }
}

async fn run_status(ingress: &str, run: &str) -> Result<()> {
    let client = RunCoordinatorIngressClient::from_client(transport::client(ingress)?, "global");
    let response = client
        .status(Json(EvidenceDigest::parse(run)?))
        .call()
        .await
        .map_err(transport::ingress_error)?
        .into_body()
        .map_err(transport::ingress_error)?;
    emit(&response.0)
}

fn rankings_input(run: &str) -> Result<RankingControlsInput> {
    Ok(RankingControlsInput {
        run: EvidenceDigest::parse(run)?,
    })
}

async fn rankings_status(ingress: &str, run: &str) -> Result<()> {
    let input = rankings_input(run)?;
    let control = PipelineControlIngressClient::from_client(transport::client(ingress)?);
    let state = control
        .rankings_progress(Json(input))
        .call()
        .await
        .map_err(transport::ingress_error)?
        .into_body()
        .map_err(transport::ingress_error)?;
    emit(&state.0)
}

async fn rankings_pause(ingress: &str, run: &str) -> Result<()> {
    let input = rankings_input(run)?;
    let control = PipelineControlIngressClient::from_client(transport::client(ingress)?);
    control
        .rankings_pause(Json(input))
        .call()
        .await
        .map_err(transport::ingress_error)?
        .into_body()
        .map_err(transport::ingress_error)?;
    emit(&serde_json::json!({"status": "paused"}))
}

async fn rankings_resume(ingress: &str, run: &str) -> Result<()> {
    let input = rankings_input(run)?;
    let control = PipelineControlIngressClient::from_client(transport::client(ingress)?);
    control
        .rankings_resume(Json(input))
        .call()
        .await
        .map_err(transport::ingress_error)?
        .into_body()
        .map_err(transport::ingress_error)?;
    emit(&serde_json::json!({"status": "resumed"}))
}

async fn browser_start(ingress: &str) -> Result<()> {
    let client =
        BrowserSessionIngressClient::from_client(transport::client(ingress)?, BROWSER_SESSION_KEY);
    let submitted = client
        .await_ready(Json(ReadinessRequest { operator: true }))
        .send()
        .await
        .map_err(transport::ingress_error)?;
    emit(&serde_json::json!({"invocation_id": submitted.invocation_handle().invocation_id()}))
}

async fn browser_status(ingress: &str) -> Result<()> {
    let client =
        BrowserSessionIngressClient::from_client(transport::client(ingress)?, BROWSER_SESSION_KEY);
    let response = client
        .status()
        .call()
        .await
        .map_err(transport::ingress_error)?
        .into_body()
        .map_err(transport::ingress_error)?;
    emit(&response.0)
}

async fn export_run(ingress: &str, run: &str, output: std::path::PathBuf) -> Result<()> {
    let request = ExportRequest {
        run: EvidenceDigest::parse(run)?,
        destination: destination(output)?,
    };
    let key = request.key()?;
    let request_key = fingerprint(&("native-export-v4", &request))?;
    let client = ExportWorkerIngressClient::from_client(transport::client(ingress)?, &key);
    let submitted = client
        .publish(Json(request))
        .idempotency_key(request_key.as_str())
        .send()
        .await
        .map_err(transport::ingress_error)?;
    emit(&await_export(submitted.invocation_handle()).await?)
}

async fn verify_retained_bundle(
    input: std::path::PathBuf,
    output: std::path::PathBuf,
    sha256: &str,
    store: std::path::PathBuf,
) -> Result<()> {
    let digest = WorkbookDigest::parse(sha256)?;
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

async fn await_export(
    invocation: InvocationHandle<reqwest::Client, Json<PublishedExport>>,
) -> Result<PublishedExport> {
    let handle = &invocation;
    let observations = futures::stream::iter(0..86_400)
        .then(move |_| async move {
            let output = match handle
                .output()
                .await
                .map_err(transport::ingress_error)?
                .into_body()
            {
                Output::Ready(Json(published)) => Some(published),
                Output::NotReady => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    None
                }
            };
            Ok::<_, anyhow::Error>(output)
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
    if args.max_pages_per_event < 1 || args.max_pages_per_event > 10_000 {
        bail!("max_pages_per-event must be 1..=10000");
    }
    let source = ImportRequest {
        original: args
            .input
            .canonicalize()
            .context("canonicalizing source workbook")?,
        workbook: WorkbookDigest::parse(&args.sha256)?,
    };
    let rankings_scope = rankings_scope_for(&args)?;
    let request = PrepareRequest {
        source,
        selection,
        concurrency: args.concurrency,
        snapshot_label: args.snapshot,
        execution: args.execution,
        rankings_scope,
    };
    let preparation_key = preparation_key(&request)?;
    let client = transport::client(&args.ingress)?;
    let control = PipelineControlIngressClient::from_client(client.clone());
    let prepared = control
        .prepare(Json(request))
        .idempotency_key(preparation_key.as_str())
        .call()
        .await
        .map_err(transport::ingress_error)?
        .into_body()
        .map_err(transport::ingress_error)?
        .0;
    let key = prepared.key()?;
    if let Some(output) = args.output {
        return submit_with_export(&control, prepared, output, key).await;
    }
    let coordinator = RunCoordinatorIngressClient::from_client(client, "global");
    let sent = coordinator
        .run(Json(prepared))
        .idempotency_key(&key)
        .send()
        .await
        .map_err(transport::ingress_error)?;
    emit(
        &serde_json::json!({"run": key, "invocation": sent.invocation_handle().invocation_id(),
        "state": "submitted", "note": "Submission is not completion; inspect status and verified export."}),
    )
}

fn rankings_scope_for(args: &Start) -> Result<Option<RankingsScope>> {
    if args.rankings {
        let season_kind = match args.rankings_season.as_str() {
            "indoor" => SeasonKind::Indoor,
            "outdoor" => SeasonKind::Outdoor,
            other => bail!("unsupported rankings season: {other}"),
        };
        let scope = RankingsScope::for_division(
            season_kind,
            &args.rankings_gender,
            args.max_pages_per_event,
        )
        .context("building rankings scope")?;
        Ok(Some(scope))
    } else {
        Ok(None)
    }
}

async fn submit_with_export(
    control: &PipelineControlIngressClient<reqwest::Client>,
    prepared: RunRequest,
    output: std::path::PathBuf,
    key: String,
) -> Result<()> {
    let automated = RunAndExportRequest {
        request: prepared,
        destination: destination(output)?,
    };
    let automation_key = fingerprint(&("run-and-export-v1", &automated))?;
    let submitted = control
        .run_and_export(Json(automated))
        .idempotency_key(automation_key.as_str())
        .send()
        .await
        .map_err(transport::ingress_error)?;
    emit(&serde_json::json!({
        "run": key, "invocation": submitted.invocation_handle().invocation_id(),
        "state": "submitted", "automatic_export": true,
        "note": "Restate publishes the verified export after run completion; submission is not completion."
    }))
}

fn destination(path: std::path::PathBuf) -> Result<std::path::PathBuf> {
    let name = path
        .file_name()
        .context("export destination must name a file")?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map_or_else(|| std::path::Path::new("."), |parent| parent);
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
