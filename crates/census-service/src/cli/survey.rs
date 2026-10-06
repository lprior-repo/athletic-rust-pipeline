use anyhow::{Context, Result};
use census_crawl::coach_directories::{
    parse_state_filter, report_json, selected_associations, survey, table_line, SOURCE_ID,
};
use census_store::Store;
use clap::Args;
use std::path::PathBuf;

use super::Cli;

#[derive(Debug, Args)]
#[command(about = "Arguments for the `survey` subcommand")]
pub(super) struct SurveyArgs {
    #[arg(help = "Comma-separated jurisdiction codes; empty means every association")]
    #[arg(long, value_name = "STATES")]
    pub(super) states: Option<String>,

    #[arg(help = "Decide from retained crawl-cache bytes only; a cache miss fails")]
    #[arg(long)]
    pub(super) offline: bool,

    #[arg(help = "Path to write the prototype-shaped probe report JSON")]
    #[arg(long, value_name = "PATH")]
    pub(super) out: PathBuf,
}

pub(super) async fn run_survey(cli: &Cli, store: &Store, args: &SurveyArgs) -> Result<()> {
    let wanted = parse_state_filter(args.states.as_deref().map_or("", |value| value));
    let associations = selected_associations(&wanted);
    let fetcher = super::build_fetcher(cli, store)?.with_source(SOURCE_ID.to_string());
    let fetcher = if args.offline {
        fetcher.with_offline(true)
    } else {
        fetcher
    };
    let records = survey(&fetcher, &associations).await;
    for record in &records {
        println!("{}", table_line(record));
    }
    let body = report_json(&records).context("encoding the coach-directory probe report")?;
    std::fs::write(&args.out, &body).with_context(|| format!("writing {}", args.out.display()))?;
    println!(
        "wrote {} probe record(s) to {}",
        records.len(),
        args.out.display()
    );
    Ok(())
}
