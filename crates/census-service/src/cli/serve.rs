use std::path::{Path, PathBuf};

use anyhow::Result;
use census_service::bootstrap::{serve_command, ServeOptions};
use clap::Args;

use super::Cli;

#[derive(Args, Debug)]
pub(super) struct ServeArgs {
    #[arg(long, value_name = "ADDR")]
    listen: Option<String>,
    #[arg(long, value_name = "DIR", conflicts_with = "store")]
    data_dir: Option<PathBuf>,
    #[arg(long, value_name = "PROTOCOL")]
    capture_protocol: Option<String>,
    #[arg(long, value_name = "N")]
    max_concurrent: Option<String>,
    #[arg(long, value_name = "SECONDS")]
    drain_timeout: Option<String>,
    #[arg(long, value_name = "GIB")]
    memory_budget_gib: Option<String>,
    #[arg(long, value_name = "HIERARCHY_PATH")]
    memory_cgroup: Option<String>,
    #[arg(long, value_name = "DIR")]
    browser_profile: Option<String>,
    #[arg(long, value_name = "PATH")]
    browser_executable: Option<String>,
    #[arg(long)]
    browser_headless: bool,
}

impl ServeArgs {
    fn options(&self, store: Option<&Path>) -> Result<ServeOptions> {
        if self.data_dir.is_some() && store.is_some() {
            anyhow::bail!("--store and --data-dir select the same endpoint root; give only one");
        }
        let flags = [
            ("--listen", self.listen.as_deref()),
            ("--capture-protocol", self.capture_protocol.as_deref()),
            ("--max-concurrent", self.max_concurrent.as_deref()),
            ("--drain-timeout", self.drain_timeout.as_deref()),
            ("--memory-budget-gib", self.memory_budget_gib.as_deref()),
            ("--memory-cgroup", self.memory_cgroup.as_deref()),
            ("--browser-profile", self.browser_profile.as_deref()),
            ("--browser-executable", self.browser_executable.as_deref()),
        ];
        let arguments = flags
            .into_iter()
            .filter_map(|(flag, value)| value.map(|value| [flag.to_owned(), value.to_owned()]))
            .flatten()
            .chain(
                self.browser_headless
                    .then(|| "--browser-headless".to_owned()),
            );
        let mut options = ServeOptions::from_env(arguments)?;
        if let Some(root) = self.data_dir.as_deref().or(store) {
            options.data_dir = root.to_path_buf();
        }
        Ok(options)
    }
}

pub(super) async fn run_serve(cli: &Cli, args: &ServeArgs) -> Result<()> {
    serve_command(args.options(cli.store.as_deref())?).await
}

#[cfg(test)]
mod tests;
