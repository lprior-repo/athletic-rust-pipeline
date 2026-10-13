use census_domain::model::{ProfileId, ProfileSelectionV1, QueryProfileV1};
use serde::Serialize;
use std::io::Write;

use super::args::Operation;
use super::error::{ProfileCliError, ProfileCliResult};

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Status {
    Registered,
    Repeated,
    Shown,
    SelectionPrepared,
    Failed,
}

#[derive(Serialize)]
struct Reply<T: Serialize> {
    version: u32,
    operation: Operation,
    status: Status,
    #[serde(flatten)]
    body: T,
}

#[derive(Serialize)]
pub(super) struct ProfileRef<'a> {
    pub(super) profile_id: &'a ProfileId,
}

#[derive(Serialize)]
pub(super) struct Shown<'a> {
    pub(super) profile_id: &'a ProfileId,
    pub(super) payload: &'a QueryProfileV1,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum RuntimeActivation {
    Unavailable,
}

#[derive(Serialize)]
pub(super) struct SelectionPrepared<'a> {
    selection: &'a ProfileSelectionV1,
    runtime_activation: RuntimeActivation,
    reason: &'static str,
}

impl<'a> SelectionPrepared<'a> {
    pub(super) fn new(selection: &'a ProfileSelectionV1) -> Self {
        Self {
            selection,
            runtime_activation: RuntimeActivation::Unavailable,
            reason: "selection document only; active profile is unchanged and runtime source, region, completion, and policy contract resolution is not performed",
        }
    }
}

#[derive(Serialize)]
struct Failure<'a> {
    code: &'static str,
    message: &'a str,
}

pub(super) fn failure<W: Write>(
    writer: &mut W,
    operation: Operation,
    error: &ProfileCliError,
) -> ProfileCliResult<()> {
    let message = error.to_string();
    reply(
        writer,
        operation,
        Status::Failed,
        Failure {
            code: error.code(),
            message: &message,
        },
    )
}

pub(super) fn reply<W: Write, T: Serialize>(
    writer: &mut W,
    operation: Operation,
    status: Status,
    body: T,
) -> ProfileCliResult<()> {
    serde_json::to_writer(
        &mut *writer,
        &Reply {
            version: 1,
            operation,
            status,
            body,
        },
    )
    .map_err(ProfileCliError::OutputJson)?;
    writer.write_all(b"\n").map_err(ProfileCliError::OutputIo)?;
    writer.flush().map_err(ProfileCliError::OutputIo)
}
