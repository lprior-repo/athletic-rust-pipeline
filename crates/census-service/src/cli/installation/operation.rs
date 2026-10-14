use census_domain::model::InstallationConfigV1;
use census_service::installation;
use census_store::{InstallationEnrollmentOutcome, InstallationState, Store};
use std::io::Write;
use std::path::Path;

use super::args::{InstallationArgs, InstallationCommand, Operation};
use super::error::{InstallationCliError, InstallationCliResult};
use super::output::{self, Status};
use crate::cli::profile::input::{self, InputKind};

pub(super) fn execute<W: Write>(
    root: Option<&Path>,
    args: &InstallationArgs,
    writer: &mut W,
) -> InstallationCliResult<()> {
    match &args.command {
        InstallationCommand::Defaults => output::reply(
            writer,
            Operation::Defaults,
            Status::Defaults,
            installation::defaults()?,
        ),
        InstallationCommand::Show => {
            let root = root.ok_or(InstallationCliError::ExplicitStoreRequired)?;
            let control = Store::installation_control_existing(root)?;
            let state = control.show()?;
            let status = match &state {
                InstallationState::NotEnrolled => Status::NotEnrolled,
                InstallationState::Enrolled { .. } => Status::Shown,
            };
            output::reply(
                writer,
                Operation::Show,
                status,
                output::State { state: &state },
            )
        }
        InstallationCommand::Enroll(args) => {
            let root = root.ok_or(InstallationCliError::ExplicitStoreRequired)?;
            let config: InstallationConfigV1 =
                input::parse_versioned(&input::read_file(&args.file)?, InputKind::Installation)?;
            installation::check_configuration(&config)?;
            let mut control = Store::installation_control_existing(root)?;
            let outcome = installation::enroll(&mut control, &config)?;
            let (status, enrollment) = match &outcome {
                InstallationEnrollmentOutcome::Enrolled(enrollment) => {
                    (Status::Enrolled, enrollment)
                }
                InstallationEnrollmentOutcome::Repeated(enrollment) => {
                    (Status::Repeated, enrollment)
                }
            };
            output::reply(
                writer,
                Operation::Enroll,
                status,
                output::Enrollment { enrollment },
            )
        }
    }
}
