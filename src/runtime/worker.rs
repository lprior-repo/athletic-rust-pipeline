use super::{
    browser_session::BrowserSession, control::PipelineControl, export_worker::ExportWorker,
    import_worker::WorkbookImport, profile_worker::ProfileWorker, query_worker::QueryWorker,
    rankings_collection::RankingsCollectionState, review_case::ReviewCase, reviewer::LocalReviewer,
    row_worker::RowWorker, run::RunCoordinator, source::SourceGateway, source_cache::SourceCache,
    Runtime,
};
use anyhow::{bail, Context, Result};
use restate_sdk::prelude::{Endpoint, HttpServer};
use std::{net::SocketAddr, path::Path};
use tokio::signal::unix::{signal, Signal, SignalKind};

#[tracing::instrument(skip_all, fields(bind = %bind))]
pub async fn serve(config: &Path, bind: SocketAddr) -> Result<()> {
    if !bind.ip().is_loopback() {
        bail!("worker must bind a loopback address");
    }
    let terminate = signal(SignalKind::terminate()).context("installing SIGTERM handler")?;
    let interrupt = signal(SignalKind::interrupt()).context("installing SIGINT handler")?;
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .context("binding native worker")?;
    let runtime = Runtime::open(config)?;
    let endpoint = Endpoint::builder()
        .bind(PipelineControl {
            runtime: runtime.clone(),
        })
        .bind(BrowserSession {
            runtime: runtime.clone(),
        })
        .bind(WorkbookImport {
            runtime: runtime.clone(),
        })
        .bind(RunCoordinator {
            runtime: runtime.clone(),
        })
        .bind(ExportWorker {
            runtime: runtime.clone(),
        })
        .bind(RowWorker {
            runtime: runtime.clone(),
        })
        .bind(QueryWorker {
            runtime: runtime.clone(),
        })
        .bind(ProfileWorker {
            runtime: runtime.clone(),
        })
        .bind(ReviewCase {
            runtime: runtime.clone(),
        })
        .bind(LocalReviewer {
            runtime: runtime.clone(),
        })
        .bind(SourceGateway {
            runtime: runtime.clone(),
        })
        .bind(SourceCache)
        .bind(RankingsCollectionState {
            runtime: runtime.clone(),
        })
        .build();
    tracing::info!(address = %listener.local_addr()?, "Native athlete worker ready");
    HttpServer::new(endpoint)
        .serve_with_cancel(listener, shutdown(terminate, interrupt))
        .await;
    // The drain logs its own certificate; there is nothing a failure here could still act on.
    runtime.drain().await;
    Ok(())
}

async fn shutdown(mut terminate: Signal, mut interrupt: Signal) {
    let (name, received) = tokio::select! {
        value = terminate.recv() => ("SIGTERM", value),
        value = interrupt.recv() => ("SIGINT", value),
    };
    match received {
        Some(()) => tracing::info!(signal = name, "Draining native worker"),
        None => tracing::error!(signal = name, "Signal stream closed; stopping worker"),
    }
}
