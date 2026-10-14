use anyhow::{Context, Result};
use census_domain::model::CaptureQualification;
use census_store::Store;
use clap::{Args, Subcommand};
use std::io::Write;
use std::path::Path;

use super::Cli;

#[derive(Debug, Args)]
pub(super) struct CaptureProtocolArgs {
    #[command(subcommand)]
    command: CaptureProtocolCommand,
}

#[derive(Debug, Subcommand)]
enum CaptureProtocolCommand {
    #[command(about = "Create lifecycle-4 capture qualification in a new or empty root")]
    Init,
    #[command(about = "Create fresh protocol-v2 capture qualification in a new or empty root")]
    InitV2,
    #[command(about = "Inspect an existing capture qualification without execution")]
    Show,
}

pub(super) fn run(cli: &Cli, args: &CaptureProtocolArgs) -> Result<()> {
    let stdout = std::io::stdout();
    run_to(cli.store.as_deref(), args, &mut stdout.lock())
}

fn run_to<W: Write>(root: Option<&Path>, args: &CaptureProtocolArgs, output: &mut W) -> Result<()> {
    let root = root.context("capture-protocol requires an explicit --store root")?;
    let qualification = match args.command {
        CaptureProtocolCommand::Init => {
            CaptureQualification::V1(Store::create_capture_qualification_v1(root)?)
        }
        CaptureProtocolCommand::InitV2 => {
            CaptureQualification::V2(Store::create_capture_qualification_v2(root)?)
        }
        CaptureProtocolCommand::Show => {
            Store::capture_qualification_control_existing(root)?.show()?
        }
    };
    serde_json::to_writer_pretty(&mut *output, &qualification)?;
    writeln!(output)?;
    Ok(())
}

#[cfg(test)]
mod tests;
