//! `export` workflow: submit a verified export to the export worker and observe its
//! completion through the Restate ingress.

use super::output::{destination, emit};
use super::transport;
use anyhow::{Context, Result};
use athletic_rust_pipeline::{
    domain::identity::EvidenceDigest,
    runtime::{
        export_worker::{ExportRequest, ExportWorkerIngressClient, PublishedExport},
        identity::fingerprint,
    },
};
use futures::{StreamExt, TryStreamExt};
use restate_sdk::ingress::{InvocationHandle, Output};
use restate_sdk::prelude::*;
use std::time::Duration;

#[tracing::instrument(skip_all, fields(command = "export"))]
pub(super) async fn export_run(ingress: &str, run: &str, output: std::path::PathBuf) -> Result<()> {
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
