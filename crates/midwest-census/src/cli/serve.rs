//! The `serve` subcommand: print the command that runs the `midwest-serve` Restate
//! endpoint against this store.

use anyhow::Result;
use midwest_census::bootstrap::ServeOptions;

use super::Cli;

/// `midwest-serve` owns the Restate endpoint; this reports how to start it against this store.
///
/// The flags and their values come from the service's own defaults, so the printed command cannot
/// drift from what `midwest-serve` parses.
pub(super) fn run_serve(cli: &Cli) -> Result<()> {
    let defaults = ServeOptions::default();
    let flags = format!(
        "--listen {} --data-dir {} --max-concurrent {} --drain-timeout {}",
        defaults.listen,
        cli.store.display(),
        defaults.max_concurrent,
        defaults.drain_timeout.as_secs()
    );
    println!("midwest-serve {flags}");
    println!("run it with: cargo run --release -p midwest-census --bin midwest-serve -- {flags}");
    Ok(())
}
