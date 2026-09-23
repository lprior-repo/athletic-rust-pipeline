//! The `serve` subcommand: print the command that runs the `census-serve` Restate
//! endpoint against this store.

use anyhow::Result;
use census_service::bootstrap::ServeOptions;

use super::Cli;

/// `census-serve` owns the Restate endpoint; this reports how to start it against this store.
///
/// The flags and their values come from the service's own defaults, so the printed command cannot
/// drift from what `census-serve` parses.
pub(super) fn run_serve(cli: &Cli) -> Result<()> {
    let defaults = ServeOptions::default();
    let flags = format!(
        "--listen {} --data-dir {} --max-concurrent {} --drain-timeout {}",
        defaults.listen,
        cli.store_root().display(),
        defaults.max_concurrent,
        defaults.drain_timeout.as_secs()
    );
    println!("census-serve {flags}");
    println!("run it with: cargo run --release -p census-service --bin census-serve -- {flags}");
    Ok(())
}
