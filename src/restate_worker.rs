use crate::{config::Config, restate_types};
use anyhow::{Context, Result};
use restate_sdk::prelude::*;
use std::{net::SocketAddr, path::Path, sync::Arc};

#[path = "restate_row_analysis.rs"]
mod analysis;
#[path = "restate_gateway_model.rs"]
mod model;
#[path = "restate_row.rs"]
mod row;
#[path = "restate_gateway_source.rs"]
mod source;
#[path = "restate_step.rs"]
mod step;

pub(super) struct Runtime {
    config: Config,
    digest: String,
}

impl Runtime {
    fn validate(&self, digest: &str) -> Result<(), TerminalError> {
        if digest != self.digest {
            return Err(TerminalError::new(
                "configuration digest differs from immutable worker deployment",
            ));
        }
        Ok(())
    }
}

pub async fn serve(config_path: &Path, bind: SocketAddr) -> Result<()> {
    if !bind.ip().is_loopback() {
        anyhow::bail!("Restate worker must bind to loopback; configure authenticated ingress before exposing personal data");
    }
    let path = config_path.to_owned();
    let runtime = tokio::task::spawn_blocking(move || {
        let digest = restate_types::config_digest(&path)?;
        let config = Config::load(&path)?;
        if restate_types::config_digest(&path)? != digest {
            anyhow::bail!("configuration changed while loading worker");
        }
        Ok::<_, anyhow::Error>(Arc::new(Runtime { config, digest }))
    })
    .await
    .context("joining worker configuration load")??;
    let filter = match tracing_subscriber::EnvFilter::try_from_default_env() {
        Ok(filter) => filter,
        Err(_) => tracing_subscriber::EnvFilter::new("info"),
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init()
        .map_err(|error| anyhow::anyhow!("initializing worker tracing: {error}"))?;
    let endpoint = Endpoint::builder()
        .bind(row::AthleteRow {
            runtime: runtime.clone(),
        })
        .bind(source::AthleticSource {
            runtime: runtime.clone(),
        })
        .bind(model::AthleticModels { runtime })
        .build();
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .context("binding Restate worker")?;
    let mut termination =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let shutdown = async move {
        tokio::select! {
            result = tokio::signal::ctrl_c() => {
                if let Err(error) = result { tracing::error!(%error, "interrupt handler failed"); }
            }
            _ = termination.recv() => {}
        }
        tracing::info!("stopping worker intake; SDK drains connections for up to ten seconds; Restate retains unfinished invocations");
    };
    tracing::info!(%bind, "Restate worker ready");
    HttpServer::new(endpoint)
        .serve_with_cancel(listener, shutdown)
        .await;
    Ok(())
}
