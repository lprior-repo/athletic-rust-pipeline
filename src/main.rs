#![forbid(unsafe_code)]

mod cli;

use anyhow::{Context, Result};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    let filter = match std::env::var("RUST_LOG") {
        Ok(value) => EnvFilter::try_new(value).context("invalid RUST_LOG filter")?,
        Err(std::env::VarError::NotPresent) => EnvFilter::new("info"),
        Err(error @ std::env::VarError::NotUnicode(_)) => {
            return Err(error).context("RUST_LOG is not Unicode")
        }
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init()
        .map_err(|error| anyhow::anyhow!("initializing tracing: {error}"))?;
    cli::run().await
}
