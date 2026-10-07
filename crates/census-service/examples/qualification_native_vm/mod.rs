use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod artifacts;
mod bootstrap;
mod cancellation;
mod captures;
mod drain;
mod guest;
mod host;
mod native;
mod oracle;
mod process;
mod qmp;
mod transport;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Mode,
}

#[derive(Subcommand)]
enum Mode {
    Host(Host),
    GuestInit,
    GuestSupervise,
    GuestAction {
        #[arg(long)]
        action: String,
    },
}

#[derive(clap::Args)]
struct Host {
    #[arg(long)]
    root: PathBuf,
    #[arg(long)]
    tools: PathBuf,
    #[arg(long)]
    base_image: PathBuf,
    #[arg(long)]
    census_serve: PathBuf,
    #[arg(long)]
    restate: PathBuf,
    #[arg(long)]
    captures: PathBuf,
}

const GUEST: &str = "/srv/qualification";
const ENDPOINT: &str = "vm_fixture_replay_2026_r1";
const WORKFLOW: &str = "vm_fixture_replay_2026_r1_sweep";
const CLOCK_WORKFLOW: &str = "vm_fixture_replay_2026_r1_clock";

pub fn run() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    match Cli::parse().command {
        Mode::Host(config) => host::run(config),
        Mode::GuestInit => guest::initialize(),
        Mode::GuestSupervise => guest::supervise(),
        Mode::GuestAction { action } => {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .context("creating guest action runtime")?;
            let seconds = match action.as_str() {
                "jurisdiction-reboot-finish" => 3_400,
                _ => 150,
            };
            runtime.block_on(async {
                tokio::time::timeout(
                    std::time::Duration::from_secs(seconds),
                    native::action(&action),
                )
                .await
                .with_context(|| {
                    format!("guest action exceeded {seconds}-second bounded qualification deadline")
                })?
            })
        }
    }
}
