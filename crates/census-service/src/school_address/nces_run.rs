mod retained_io;
mod retention;
#[cfg(test)]
mod tests;
mod verified_input;

mod build_manifest;
mod prepare;
mod project;

use anyhow::{bail, Context, Result};
use census_domain::model::serialized_digest;
use census_domain::school_directory::nces_address::NcesAddressRunReceiptV1;
use census_domain::UsJurisdiction;
use census_store::Store;
use std::path::PathBuf;

const FIVE_STATE_SCOPE: [UsJurisdiction; 5] = [
    UsJurisdiction::Michigan,
    UsJurisdiction::Minnesota,
    UsJurisdiction::Iowa,
    UsJurisdiction::Illinois,
    UsJurisdiction::Colorado,
];

#[derive(Debug, Clone)]
pub struct NcesSelectedInputV1 {
    pub source_root: PathBuf,
    pub run_manifest_digest: String,
    pub scope: Vec<UsJurisdiction>,
    pub ccd: NcesSelectedArtifactV1,
    pub pss: NcesSelectedArtifactV1,
}

#[derive(Debug, Clone)]
pub struct NcesSelectedArtifactV1 {
    pub path: PathBuf,
    pub sha256: String,
    pub source_url: String,
    pub captured_on: String,
}

pub(crate) fn accept(
    store: &Store,
    selection: &NcesSelectedInputV1,
) -> Result<NcesAddressRunReceiptV1> {
    let prepared = prepare::run(store, selection)?;
    let manifest = build_manifest::build(&prepared, selection)?;
    project::run(store, &prepared, manifest, selection)
}

pub(crate) fn reopen(
    store: &Store,
    expected_run_manifest_digest: &str,
) -> Result<(NcesAddressRunReceiptV1, super::VerifiedGeneration)> {
    let run = store
        .run_manifest()?
        .context("NCES address reopen requires a bound Store run")?;
    if serialized_digest(&run)? != expected_run_manifest_digest {
        bail!("NCES address receipt was requested for a different Store run");
    }
    let receipt = store
        .nces_address_run_v1()?
        .context("NCES address run receipt is absent")?;
    if receipt.run_manifest_digest() != expected_run_manifest_digest {
        bail!("NCES address receipt run digest mismatch");
    }
    let out = store.out_dir().join("nces-address");
    let generation = super::verify_current(&out)?;
    retention::verify_generation(
        &generation,
        receipt.scope(),
        receipt.ccd(),
        receipt.pss(),
        Some(receipt.address_generation_digest()),
    )?;
    retention::verify_retained_inputs(&out, &receipt)?;
    Ok((receipt, generation))
}
