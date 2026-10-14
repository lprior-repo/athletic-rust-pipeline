use anyhow::Result;
use std::io::Write;
use std::path::Path;

use super::Cli;

mod args;
mod error;
mod operation;
mod output;

pub(super) use args::InstallationArgs;
use error::InstallationCliResult;

pub(super) fn run(cli: &Cli, args: &InstallationArgs) -> Result<()> {
    let stdout = std::io::stdout();
    let mut writer = stdout.lock();
    run_to(cli.store.as_deref(), args, &mut writer).map_err(Into::into)
}

fn run_to<W: Write>(
    root: Option<&Path>,
    args: &InstallationArgs,
    writer: &mut W,
) -> InstallationCliResult<()> {
    match operation::execute(root, args, writer) {
        Ok(()) => Ok(()),
        Err(error) if error.output_failed() => Err(error),
        Err(error) => {
            output::failure(writer, args.operation(), &error)?;
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests;
