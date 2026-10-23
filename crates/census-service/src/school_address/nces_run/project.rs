use anyhow::{bail, Context, Result};
use census_domain::school_directory::nces_address::NcesAddressRunReceiptV1;
use census_store::Store;
use std::path::Path;

use super::{build_manifest, prepare, NcesSelectedInputV1};

pub(super) fn run(
    store: &Store,
    prepared: &prepare::PreparedInput,
    manifest: build_manifest::ManifestOutput,
    selection: &NcesSelectedInputV1,
) -> Result<NcesAddressRunReceiptV1> {
    if let Some(existing) = store.nces_address_run_v1()? {
        if existing.run_manifest_digest() != selection.run_manifest_digest
            || existing.scope() != selection.scope
            || existing.ccd() != &manifest.ccd_ref
            || existing.pss() != &manifest.pss_ref
            || existing.input_manifest_digest() != manifest.input_manifest_digest
        {
            bail!("a different immutable NCES address receipt is already bound to this Store");
        }
        return Ok(super::reopen(store, &selection.run_manifest_digest)?.0);
    }

    stage_and_project(store, prepared, manifest, selection)
}

fn stage_and_project(
    store: &Store,
    prepared: &prepare::PreparedInput,
    manifest: build_manifest::ManifestOutput,
    selection: &NcesSelectedInputV1,
) -> Result<NcesAddressRunReceiptV1> {
    let staging = tempfile::tempdir().context("creating bounded NCES input staging area")?;
    let ccd_name = Path::new("ccd.csv");
    let pss_name = Path::new("pss.csv");
    std::fs::write(staging.path().join(ccd_name), &prepared.ccd.bytes)
        .context("staging verified CCD bytes")?;
    std::fs::write(staging.path().join(pss_name), &prepared.pss.bytes)
        .context("staging verified PSS bytes")?;
    let manifest_path = staging.path().join("manifest.json");
    std::fs::write(&manifest_path, &manifest.manifest_bytes)
        .context("writing replay manifest for verified NCES inputs")?;

    let out = store.out_dir().join("nces-address");
    if out.exists() {
        bail!("NCES address output exists without a matching immutable Store receipt");
    }
    let replay = super::super::replay::run_scoped(
        &manifest_path,
        &out,
        super::super::CorpusScope::Jurisdictions(&selection.scope),
    )?;
    let frozen = super::super::verify_current(&out)?;
    super::retention::verify_generation(
        &frozen,
        &selection.scope,
        &manifest.ccd_ref,
        &manifest.pss_ref,
        Some(replay.generation_digest()),
    )?;
    let receipt = NcesAddressRunReceiptV1::new(
        selection.run_manifest_digest.clone(),
        selection.scope.clone(),
        manifest.ccd_ref,
        manifest.pss_ref,
        manifest.input_manifest_digest,
        replay.generation_digest().to_string(),
    )?;
    store.bind_nces_address_run_v1(&receipt)?;
    Ok(receipt)
}
