use anyhow::Result;
use serde_json::json;
use std::path::PathBuf;

mod host;

#[derive(clap::Args)]
pub struct Args {
    #[arg(long)]
    pub manifest: PathBuf,
    #[arg(long)]
    pub host_manifest: Option<PathBuf>,
}

pub fn run(args: Args) -> Result<()> {
    let captures = super::captures::verify(&args.manifest)?;
    let evidence = match args.host_manifest.as_deref() {
        Some(path) => json!({"captures": captures, "host": host::verify_host_manifest(path)?}),
        None => captures,
    };
    println!("{}", serde_json::to_string_pretty(&evidence)?);
    Ok(())
}
