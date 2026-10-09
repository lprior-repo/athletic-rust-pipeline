use anyhow::{Context, Result};
use census_service::school_address::{parse_source_pairs, preflight_generation, Overrides};
use clap::Args;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub(super) struct SchoolAddressPreflightArgs {
    #[arg(
        long,
        value_name = "DIR",
        help = "School-directory root holding current/ and generations/"
    )]
    generation: PathBuf,
    #[arg(
        long = "evidence-url",
        value_name = "SOURCE=URL",
        help = "Published capture URL, paired with its actual acquisition date; repeatable"
    )]
    evidence_urls: Vec<String>,
    #[arg(
        long = "evidence-date",
        value_name = "SOURCE=YYYY-MM-DD",
        help = "Actual acquisition date for the same source or capture selector; repeatable"
    )]
    evidence_dates: Vec<String>,
}

pub(super) fn run(args: &SchoolAddressPreflightArgs) -> Result<()> {
    let overrides = Overrides {
        urls: parse_source_pairs(&args.evidence_urls)?,
        dates: parse_source_pairs(&args.evidence_dates)?,
    };
    let digest = preflight_generation(&args.generation, overrides)
        .context("checking the school-address input before source discovery")?;
    println!("school_address_generation_sha256\t{digest}");
    Ok(())
}
