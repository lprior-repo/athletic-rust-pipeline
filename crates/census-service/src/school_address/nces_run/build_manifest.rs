use anyhow::Result;
use census_domain::school_directory::nces_address::{
    NcesArtifactProviderV1, NcesArtifactReferenceV1,
};
use std::path::PathBuf;

use super::{prepare, retention, verified_input, NcesSelectedInputV1};

pub(super) struct ManifestOutput {
    pub(super) ccd_ref: NcesArtifactReferenceV1,
    pub(super) pss_ref: NcesArtifactReferenceV1,
    pub(super) manifest_bytes: Vec<u8>,
    pub(super) input_manifest_digest: String,
}

pub(super) fn build(
    prepared: &prepare::PreparedInput,
    selection: &NcesSelectedInputV1,
) -> Result<ManifestOutput> {
    let ccd_ref = NcesArtifactReferenceV1::new(
        NcesArtifactProviderV1::Ccd,
        prepared.ccd.sha256.clone(),
        prepared.ccd.byte_length,
        selection.ccd.source_url.clone(),
        selection.ccd.captured_on.clone(),
    )?;
    let pss_ref = NcesArtifactReferenceV1::new(
        NcesArtifactProviderV1::Pss,
        prepared.pss.sha256.clone(),
        prepared.pss.byte_length,
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
                sha256: prepared.ccd.sha256.clone(),
            },
            retention::ReplayCapture {
                format: retention::ReplayFormat::NcesPss,
                path: PathBuf::from("pss.csv"),
                url: selection.pss.source_url.clone(),
                retrieved_at: selection.pss.captured_on.clone(),
                sha256: prepared.pss.sha256.clone(),
            },
        ],
    };
    let manifest_bytes = serde_json::to_vec(&replay_manifest)?;
    let input_manifest_digest = verified_input::sha256_hex(&manifest_bytes);

    Ok(ManifestOutput {
        ccd_ref,
        pss_ref,
        manifest_bytes,
        input_manifest_digest,
    })
}
