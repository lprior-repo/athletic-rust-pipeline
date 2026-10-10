use anyhow::Result;
use census_service::bootstrap::ServeOptions;

use super::Cli;

pub(super) fn run_serve(cli: &Cli) -> Result<()> {
    let defaults = ServeOptions::default();
    let flags = format!(
        "--listen {} --data-dir {} --max-concurrent {} --drain-timeout {} --memory-budget-gib {}",
        defaults.listen,
        cli.store_root().display(),
        defaults.max_concurrent,
        defaults.drain_timeout.as_secs(),
        defaults.memory_budget_bytes / (1024 * 1024 * 1024)
    );
    println!("census-serve {flags}");
    println!("run it with: cargo run --release -p census-service --bin census-serve -- {flags}");
    Ok(())
}
