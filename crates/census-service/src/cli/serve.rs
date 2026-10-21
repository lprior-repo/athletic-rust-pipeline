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
    println!("build it with: env -u CI tools/moon-local run pipeline:build-portable");
    println!("target/moon-portable/x86_64-unknown-linux-gnu/release/census-serve {flags}");
    Ok(())
}
