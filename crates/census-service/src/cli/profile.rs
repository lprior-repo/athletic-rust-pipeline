use super::Cli;
use anyhow::Result;
use std::io::Write;
use std::path::Path;

mod args;
pub(in crate::cli) mod error;
pub(in crate::cli) mod input;
mod operation;
mod output;

pub(super) use args::ProfileArgs;
use error::ProfileCliResult;

pub(super) fn run(cli: &Cli, args: &ProfileArgs) -> Result<()> {
    let stdout = std::io::stdout();
    let mut writer = stdout.lock();
    run_to(cli.store.as_deref(), args, &mut writer).map_err(Into::into)
}

fn run_to<W: Write>(
    root: Option<&Path>,
    args: &ProfileArgs,
    writer: &mut W,
) -> ProfileCliResult<()> {
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
