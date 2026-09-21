//! `start` workflow: prepare a workbook run against the control service and submit it to
//! the run coordinator, either as a plain run or as an automatic run-and-export.

use super::args::Start;
use super::output::{destination, emit};
use super::transport;
use anyhow::{bail, Context, Result};
use athletic_rust_pipeline::{
    domain::identity::WorkbookDigest,
    runtime::{
        control::{PipelineControlIngressClient, PrepareRequest, RunAndExportRequest},
        identity::fingerprint,
        import::ImportRequest,
        rankings::{RankingsScope, SeasonKind},
        run::RunCoordinatorIngressClient,
        run_protocol::{preparation_key, RunRequest, Selection},
    },
};
use restate_sdk::prelude::*;

pub(super) async fn start(args: Start) -> Result<()> {
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
