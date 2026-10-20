use anyhow::{Context, Result};
use census_service::school_address::{preflight_generation, Overrides};
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
}

pub(super) fn run(args: &SchoolAddressPreflightArgs) -> Result<()> {
    let overrides = Overrides::default();
    let digest = preflight_generation(&args.generation, overrides)
        .context("checking the school-address input before source discovery")?;
    println!("school_address_generation_sha256\t{digest}");
    Ok(())
}
