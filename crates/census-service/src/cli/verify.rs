use anyhow::{Context, Result};
use census_report::workbook::publication::{current_workbook, verify_published};
use clap::Args;
use std::path::{Path, PathBuf};

#[derive(Debug, Args)]
#[command(
    about = "Verify the complete published workbook against its durable frozen input and generation manifest"
)]
pub struct VerifyArgs {
    #[arg(
        long,
        help = "Manifested generation workbook. Defaults to out/publication/current/workbook.xlsx"
    )]
    pub workbook: Option<PathBuf>,
}

pub fn run_verify(store_root: &Path, args: &VerifyArgs) -> Result<()> {
    let path = match &args.workbook {
        Some(path) => path.clone(),
        None => current_workbook(&store_root.join("out/publication"))
            .context("resolving the published generation")?,
    };
    verify_published(&path).context("verifying the complete frozen publication")?;
    println!(
        "verify: OK (complete frozen generation)\t{}",
        path.display()
    );
    Ok(())
}
