mod retained_io;
mod retention;
#[cfg(test)]
mod tests;
mod verified_input;

use anyhow::{bail, Context, Result};
use census_domain::model::{serialized_digest, RunManifest};
use census_domain::school_directory::nces_address::{
    NcesAddressRunReceiptV1, NcesArtifactProviderV1, NcesArtifactReferenceV1,
};
use census_domain::UsJurisdiction;
use census_store::Store;
use std::path::{Path, PathBuf};

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
    let run = store
        .run_manifest()?
        .context("NCES address intake requires a bound Store run")?;
    validate_selection(selection, &run)?;
    let ccd = verified_input::read_verified(&selection.source_root, &selection.ccd)?;
    let pss = verified_input::read_verified(&selection.source_root, &selection.pss)?;

    let ccd_ref = NcesArtifactReferenceV1::new(
        NcesArtifactProviderV1::Ccd,
        ccd.sha256.clone(),
        ccd.byte_length,
        selection.ccd.source_url.clone(),
        selection.ccd.captured_on.clone(),
    )?;
    let pss_ref = NcesArtifactReferenceV1::new(
        NcesArtifactProviderV1::Pss,
        pss.sha256.clone(),
        pss.byte_length,
        selection.pss.source_url.clone(),
        selection.pss.captured_on.clone(),
    )?;

    let replay_manifest = retention::ReplayManifest {
        schema_revision: 1,
        month: "2026-10".to_string(),
        captures: vec![
            retention::ReplayCapture {
                format: retention::ReplayFormat::NcesCcd,
                path: PathBuf::from("ccd.csv"),
                url: selection.ccd.source_url.clone(),
                retrieved_at: selection.ccd.captured_on.clone(),
                sha256: ccd.sha256.clone(),
            },
            retention::ReplayCapture {
                format: retention::ReplayFormat::NcesPss,
                path: PathBuf::from("pss.csv"),
                url: selection.pss.source_url.clone(),
                retrieved_at: selection.pss.captured_on.clone(),
                sha256: pss.sha256.clone(),
            },
        ],
    };
    let manifest_bytes = serde_json::to_vec(&replay_manifest)?;
    let input_manifest_digest = verified_input::sha256_hex(&manifest_bytes);

    if let Some(existing) = store.nces_address_run_v1()? {
        if existing.run_manifest_digest() != selection.run_manifest_digest
            || existing.scope() != selection.scope
            || existing.ccd() != &ccd_ref
            || existing.pss() != &pss_ref
            || existing.input_manifest_digest() != input_manifest_digest
        {
            bail!("a different immutable NCES address receipt is already bound to this Store");
        }
        return Ok(reopen(store, &selection.run_manifest_digest)?.0);
    }

    let staging = tempfile::tempdir().context("creating bounded NCES input staging area")?;
    let ccd_name = Path::new("ccd.csv");
    let pss_name = Path::new("pss.csv");
    std::fs::write(staging.path().join(ccd_name), &ccd.bytes)
        .context("staging verified CCD bytes")?;
    std::fs::write(staging.path().join(pss_name), &pss.bytes)
        .context("staging verified PSS bytes")?;
    let manifest_path = staging.path().join("manifest.json");
    std::fs::write(&manifest_path, &manifest_bytes)
        .context("writing replay manifest for verified NCES inputs")?;

    let out = store.out_dir().join("nces-address");
    if out.exists() {
        bail!("NCES address output exists without a matching immutable Store receipt");
    }
    let replay = super::replay::run(&manifest_path, &out)?;
    let frozen = super::verify_current(&out)?;
    retention::verify_generation(
        &frozen,
        &selection.scope,
        &ccd_ref,
        &pss_ref,
        Some(replay.generation_digest()),
    )?;
    let receipt = NcesAddressRunReceiptV1::new(
        selection.run_manifest_digest.clone(),
        selection.scope.clone(),
        ccd_ref,
        pss_ref,
        input_manifest_digest,
        replay.generation_digest().to_string(),
    )?;
    store.bind_nces_address_run_v1(&receipt)?;
    Ok(receipt)
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

fn validate_selection(selection: &NcesSelectedInputV1, run: &RunManifest) -> Result<()> {
    let digest = serialized_digest(run)?;
    if selection.run_manifest_digest != digest {
        bail!("selected NCES artifacts belong to a different run manifest");
    }
    if selection.scope.as_slice() != FIVE_STATE_SCOPE || selection.scope != run.jurisdictions {
        bail!("selected NCES artifacts must use the exact MI/MN/IA/IL/CO run scope");
    }
    Ok(())
}
