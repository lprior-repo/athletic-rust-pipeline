use anyhow::{Context, Result};
use census_store::Store;
use clap::{Args, Subcommand};
use std::io::Write;
use std::path::Path;

use super::Cli;

mod participation_intake;
mod result_intake;
mod roster_enrollment;
mod roster_intake;
mod roster_inventory;
mod roster_replay;
mod roster_review;

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
    #[command(about = "List bounded unreviewed retained roster packets and matching program IDs")]
    RosterInventory,
    #[command(
        about = "Preview source-backed roster approval; supply its SHA256 to accept the exact retained representation"
    )]
    RosterReview(roster_review::RosterReviewArgs),
    #[command(
        about = "Retain an original result document with exact checked source-row ownership"
    )]
    ResultIntake(result_intake::ResultIntakeArgs),
    #[command(
        about = "Retain an independently checked participation packet without accepting identity or eligibility"
    )]
    ParticipationIntake(participation_intake::ParticipationIntakeArgs),
    #[command(
        about = "Read the original reviewed roster receipt without re-inspection or new acceptance"
    )]
    RosterReplay(roster_replay::RosterReplayArgs),
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
        CaptureProtocolCommand::RosterInventory => {
            return roster_inventory::run_to(root, output);
        }
        CaptureProtocolCommand::RosterReview(args) => {
            return roster_review::run_to(root, args, output);
        }
        CaptureProtocolCommand::ResultIntake(args) => {
            return result_intake::run_to(root, args, output);
        }
        CaptureProtocolCommand::ParticipationIntake(args) => {
            return participation_intake::run_to(root, args, output);
        }
        CaptureProtocolCommand::RosterReplay(args) => {
            return roster_replay::run_to(root, args, output);
        }
    };
    writeln!(output)?;
    Ok(())
}

#[cfg(test)]
mod tests;
