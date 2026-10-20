use anyhow::{Context, Result};
use census_store::Store;
use clap::{Args, Subcommand};
use std::io::Write;
use std::path::Path;

use super::Cli;

mod roster_enrollment;
mod roster_intake;

#[derive(Debug, Args)]
pub(super) struct CaptureProtocolArgs {
    #[command(subcommand)]
    command: CaptureProtocolCommand,
}

#[derive(Debug, Subcommand)]
enum CaptureProtocolCommand {
    #[command(
        about = "Create current graph-V3 and capture-V2 qualification atomically in a new or empty root"
    )]
    Init,
    #[command(about = "Inspect an existing capture qualification without execution")]
    Show,
    #[command(
        about = "Explicitly enable graph admission and versioned input tracking on a stopped graph-V3 store"
    )]
    EnableAdmission,
    #[command(
        about = "Preview exact roster-capability approval bytes; supply their SHA256 to apply, without accepting membership"
    )]
    RosterEnrollment(roster_enrollment::RosterEnrollmentArgs),
    #[command(
        about = "Retain original roster rows as unreviewed facts without accepting identity or affiliation"
    )]
    RosterIntake(roster_intake::RosterIntakeArgs),
}

pub(super) fn run(cli: &Cli, args: &CaptureProtocolArgs) -> Result<()> {
    let stdout = std::io::stdout();
    run_to(cli.store.as_deref(), args, &mut stdout.lock())
}

fn run_to<W: Write>(root: Option<&Path>, args: &CaptureProtocolArgs, output: &mut W) -> Result<()> {
    let root = root.context("capture-protocol requires an explicit --store root")?;
    match &args.command {
        CaptureProtocolCommand::Init => serde_json::to_writer_pretty(
            &mut *output,
            &Store::create_current_qualification_v3(root)?,
        )?,
        CaptureProtocolCommand::Show => serde_json::to_writer_pretty(
            &mut *output,
            &Store::capture_qualification_control_existing(root)?.show()?,
        )?,
        CaptureProtocolCommand::EnableAdmission => {
            let (store, writer) = Store::open_public_identity_qualification_v3(root)?.into_parts();
            writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
            serde_json::to_writer_pretty(&mut *output, writer.store_identity())?;
        }
        CaptureProtocolCommand::RosterEnrollment(args) => {
            return roster_enrollment::run_to(root, args, output);
        }
        CaptureProtocolCommand::RosterIntake(args) => {
            return roster_intake::run_to(root, args, output);
        }
    };
    writeln!(output)?;
    Ok(())
}

#[cfg(test)]
mod tests;
