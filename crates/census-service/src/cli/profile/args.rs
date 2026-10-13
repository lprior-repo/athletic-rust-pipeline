use census_domain::model::{ProfileId, ProfileSelectionV1};
use clap::{Args, Subcommand};
use serde::Serialize;
use std::path::PathBuf;

use super::error::{ProfileCliError, ProfileCliResult};
use super::input;

#[derive(Debug, Args)]
#[command(
    about = "Register or inspect immutable configuration, or prepare a checked selection document; explicit --store and a stopped store owner required. Does not activate a runtime profile"
)]
pub(in crate::cli) struct ProfileArgs {
    #[command(subcommand)]
    pub(super) command: ProfileCommand,
}

#[derive(Debug, Subcommand)]
pub(super) enum ProfileCommand {
    #[command(about = "Register one checked query-profile JSON payload as immutable metadata")]
    Register(RegisterArgs),
    #[command(about = "Show an exact retained profile; no source or policy resolution")]
    Show(ShowArgs),
    #[command(
        about = "Prepare or recheck an installation-bound selection document. No active selection is persisted; runtime activation remains unavailable"
    )]
    Select(SelectArgs),
}

#[derive(Debug, Args)]
pub(super) struct RegisterArgs {
    #[arg(
        long,
        value_name = "JSON",
        help = "Version-1 query-profile payload, at most 65,536 encoded bytes"
    )]
    pub(super) file: PathBuf,
}

#[derive(Debug, Args)]
pub(super) struct ShowArgs {
    #[arg(long, value_name = "PROFILE_ID", value_parser = ProfileId::parse)]
    pub(super) id: ProfileId,
}

#[derive(Debug, Args)]
pub(super) struct SelectArgs {
    #[arg(
        long,
        value_name = "PROFILE_ID",
        value_parser = ProfileId::parse,
        required_unless_present = "selection",
        conflicts_with = "selection",
        help = "Retained profile ID to bind to the existing store identity"
    )]
    pub(super) id: Option<ProfileId>,
    #[arg(
        long,
        value_name = "JSON",
        required_unless_present = "id",
        conflicts_with = "id",
        help = "Existing ProfileSelectionV1 document to recheck; foreign ownership refuses"
    )]
    pub(super) selection: Option<PathBuf>,
}

pub(super) enum SelectionRequest {
    LocalId(ProfileId),
    Document(ProfileSelectionV1),
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Operation {
    Register,
    Show,
    Select,
}

impl ProfileArgs {
    pub(super) fn operation(&self) -> Operation {
        match self.command {
            ProfileCommand::Register(_) => Operation::Register,
            ProfileCommand::Show(_) => Operation::Show,
            ProfileCommand::Select(_) => Operation::Select,
        }
    }
}

impl SelectArgs {
    pub(super) fn prepare(&self) -> ProfileCliResult<SelectionRequest> {
        match (&self.id, &self.selection) {
            (Some(id), None) => Ok(SelectionRequest::LocalId(id.clone())),
            (None, Some(path)) => input::read_selection(path).map(SelectionRequest::Document),
            _ => Err(ProfileCliError::SelectionArguments),
        }
    }
}
