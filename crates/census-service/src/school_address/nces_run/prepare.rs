use anyhow::{bail, Context, Result};
use census_domain::model::{serialized_digest, RunManifest};
use census_domain::school_directory::nces_address::NcesAddressRunReceiptV1;
use census_store::Store;

use super::{verified_input, NcesSelectedInputV1};

pub(super) struct PreparedInput {
    pub(super) ccd: verified_input::VerifiedBytes,
    pub(super) pss: verified_input::VerifiedBytes,
}

pub(super) fn run(store: &Store, selection: &NcesSelectedInputV1) -> Result<PreparedInput> {
    let run = store
        .run_manifest()?
        .context("NCES address intake requires a bound Store run")?;
    validate_selection(selection, &run)?;
    let ccd = verified_input::read_verified(&selection.source_root, &selection.ccd)?;
    let pss = verified_input::read_verified(&selection.source_root, &selection.pss)?;
    Ok(PreparedInput { ccd, pss })
}

fn validate_selection(selection: &NcesSelectedInputV1, run: &RunManifest) -> Result<()> {
    let digest = serialized_digest(run)?;
    if selection.run_manifest_digest != digest {
        bail!("selected NCES artifacts belong to a different run manifest");
    }
    NcesAddressRunReceiptV1::validate_scope(&selection.scope)?;
    if selection.scope != run.jurisdictions {
        bail!("selected NCES artifacts must preserve the bound run jurisdiction order");
    }
    Ok(())
}
