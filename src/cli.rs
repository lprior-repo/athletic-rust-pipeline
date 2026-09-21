//! Operator CLI for the native Restate pipeline.
//!
//! `args` owns argument parsing, `transport` the ingress clients, and `flow_control` the
//! Restate source-scope admission rules. Each command family owns one submodule: `status`
//! (read-only observation), `start` (workbook run submission), `export` (verified export
//! publication), and `verify` (retained bundle verification). `output` holds the shared
//! stdout and destination helpers.

mod args;
mod export;
mod flow_control;
mod output;
mod start;
mod status;
mod transport;
mod verify;

use anyhow::Result;
use args::{Cli, Command};
use athletic_rust_pipeline::runtime::worker;
use clap::Parser;
use output::emit;

pub async fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Worker { config, bind } => worker::serve(&config, bind).await,
        Command::Deploy { admin, endpoint } => emit(&transport::deploy(&admin, &endpoint).await?),
        Command::Start(args) => start::start(args).await,
        Command::Status { ingress, run } => status::run_status(&ingress, &run).await,
        Command::RankingsStatus { ingress, run } => status::rankings_status(&ingress, &run).await,
        Command::RankingsPause { ingress, run } => status::rankings_pause(&ingress, &run).await,
        Command::RankingsResume { ingress, run } => status::rankings_resume(&ingress, &run).await,
        Command::BrowserStart { ingress } => status::browser_start(&ingress).await,
        Command::BrowserStatus { ingress } => status::browser_status(&ingress).await,
        Command::Export {
            ingress,
            run,
            output,
        } => export::export_run(&ingress, &run, output).await,
        Command::Verify {
            input,
            output,
            sha256,
            store,
        } => verify::verify_retained_bundle(input, output, &sha256, store).await,
    }
}
