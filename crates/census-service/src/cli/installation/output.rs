use census_domain::model::InstallationEnrollmentV1;
use census_store::InstallationState;
use serde::Serialize;
use std::io::Write;

use super::args::Operation;
use super::error::{InstallationCliError, InstallationCliResult};

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Status {
    Enrolled,
    Repeated,
    Shown,
    NotEnrolled,
    Defaults,
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
pub(super) struct Enrollment<'a> {
    pub(super) enrollment: &'a InstallationEnrollmentV1,
}

#[derive(Serialize)]
pub(super) struct State<'a> {
    pub(super) state: &'a InstallationState,
}

#[derive(Serialize)]
struct Failure<'a> {
    code: &'static str,
    message: &'a str,
}

pub(super) fn failure<W: Write>(
    writer: &mut W,
    operation: Operation,
    error: &InstallationCliError,
) -> InstallationCliResult<()> {
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
) -> InstallationCliResult<()> {
    serde_json::to_writer(
        &mut *writer,
        &Reply {
            version: 1,
            operation,
            status,
            body,
        },
    )
    .map_err(InstallationCliError::OutputJson)?;
    writer
        .write_all(b"\n")
        .map_err(InstallationCliError::OutputIo)?;
    writer.flush().map_err(InstallationCliError::OutputIo)
}
