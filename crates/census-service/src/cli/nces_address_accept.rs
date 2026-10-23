use super::Cli;
use anyhow::{bail, Context, Result};
use census_domain::model::serialized_digest;
use census_service::school_address::{
    accept_nces_address_run, NcesSelectedArtifactV1, NcesSelectedInputV1,
};
use census_store::Store;
use clap::Args;
use std::path::PathBuf;

#[cfg(test)]
mod tests;

#[derive(Debug, Args)]
pub(super) struct NcesAddressAcceptArgs {
    #[arg(
        long,
        value_name = "DIR",
        help = "Root directory containing the selected input files"
    )]
    source_root: PathBuf,
    #[arg(
        long,
        value_name = "PATH",
        help = "Contained path to the decompressed CCD CSV"
    )]
    ccd: PathBuf,
    #[arg(long, help = "Expected lowercase SHA-256 of the CCD bytes")]
    ccd_sha256: String,
    #[arg(long, help = "Published CCD source URL (HTTPS)")]
    ccd_url: String,
    #[arg(long, value_name = "YYYY-MM-DD", help = "Actual CCD capture date")]
    ccd_captured_on: String,
    #[arg(long, value_name = "PATH", help = "Contained path to the PSS CSV")]
    pss: PathBuf,
    #[arg(long, help = "Expected lowercase SHA-256 of the PSS bytes")]
    pss_sha256: String,
    #[arg(long, help = "Published PSS source URL (HTTPS)")]
    pss_url: String,
    #[arg(long, value_name = "YYYY-MM-DD", help = "Actual PSS capture date")]
    pss_captured_on: String,
    #[arg(
        long,
        help = "Persist the verified address generation and immutable run receipt"
    )]
    apply: bool,
}

pub(super) fn run(cli: &Cli, args: &NcesAddressAcceptArgs) -> Result<()> {
    if !args.apply {
        bail!("NCES address acceptance writes an immutable Store receipt; pass --apply to proceed");
    }
    let root = cli
        .store
        .as_deref()
        .context("nces-address-accept requires an explicit --store and a stopped store owner")?;
    let (store, _writer) = Store::open_public_identity_qualification_v3(root)
        .context("opening the existing current qualification Store; stop its owner first")?
        .into_parts();
    let run = store
        .run_manifest()
        .context("reading the selected Store run")?
        .context("nces-address-accept requires a bound selected Store run")?;
    let run_manifest_digest =
        serialized_digest(&run).context("digesting the selected Store run")?;
    let selection = NcesSelectedInputV1 {
        source_root: args.source_root.clone(),
        run_manifest_digest,
        scope: run.jurisdictions.clone(),
        ccd: NcesSelectedArtifactV1 {
            path: args.ccd.clone(),
            sha256: args.ccd_sha256.clone(),
            source_url: args.ccd_url.clone(),
            captured_on: args.ccd_captured_on.clone(),
        },
        pss: NcesSelectedArtifactV1 {
            path: args.pss.clone(),
            sha256: args.pss_sha256.clone(),
            source_url: args.pss_url.clone(),
            captured_on: args.pss_captured_on.clone(),
        },
    };
    let receipt = accept_nces_address_run(&store, &selection)
        .context("accepting selected hash-pinned NCES address inputs")?;
    println!("nces_address_run_receipt\taccepted");
    println!("run_manifest_sha256\t{}", receipt.run_manifest_digest());
    println!(
        "address_generation_sha256\t{}",
        receipt.address_generation_digest()
    );
    println!("input_manifest_sha256\t{}", receipt.input_manifest_digest());
    println!("scope\tMI,MN,IA,IL,CO");
    Ok(())
}
