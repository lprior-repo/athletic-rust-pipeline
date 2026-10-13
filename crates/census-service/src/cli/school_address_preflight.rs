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
    let url_pairs = parse_source_pairs(&args.evidence_urls)?;
    let date_pairs = parse_source_pairs(&args.evidence_dates)?;
    let mut acquisitions = Vec::new();
    for (source, url) in url_pairs {
        let (provider, path) = match source.split_once('@') {
            Some((p, rest)) => (p.to_string(), rest.to_string()),
            None => (source.clone(), String::new()),
        };
        let entry = u32::try_from(acquisitions.len()).unwrap_or(u32::MAX);
        let retrieved_at = date_pairs.get(&source).cloned();
        acquisitions.push(census_service::school_address::Acquisition {
            entry,
            provider,
            path,
            url,
            retrieved_at,
        });
    }
    let overrides = Overrides { acquisitions };
    let digest = preflight_generation(&args.generation, overrides)
        .context("checking the school-address input before source discovery")?;
    println!("school_address_generation_sha256\t{digest}");
    Ok(())
}
