use clap::{Args, Subcommand};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Args)]
#[command(
    about = "Immutable stopped-store enrollment and inspection; reusable execution remains unsupported"
)]
pub(in crate::cli) struct InstallationArgs {
    #[command(subcommand)]
    pub(super) command: InstallationCommand,
}

#[derive(Debug, Subcommand)]
pub(super) enum InstallationCommand {
    #[command(
        about = "Enroll an existing logically empty stopped store using exact checked configuration"
    )]
    Enroll(EnrollArgs),
    #[command(about = "Show verified enrollment without activation")]
    Show,
    #[command(
        about = "Inspect compiled enrollment-only defaults and source catalog; does not open a store"
    )]
    Defaults,
}

#[derive(Debug, Args)]
pub(super) struct EnrollArgs {
    #[arg(
        long,
        value_name = "JSON",
        help = "Version-1 configuration, at most 65,536 encoded bytes"
    )]
    pub(super) file: PathBuf,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Operation {
    Enroll,
    Show,
    Defaults,
}

impl InstallationArgs {
    pub(super) fn operation(&self) -> Operation {
        match self.command {
            InstallationCommand::Enroll(_) => Operation::Enroll,
            InstallationCommand::Show => Operation::Show,
            InstallationCommand::Defaults => Operation::Defaults,
        }
    }
}
