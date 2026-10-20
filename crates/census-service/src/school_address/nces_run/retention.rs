use anyhow::{bail, Context, Result};
use census_domain::school_directory::nces_address::{
    NcesAddressRunReceiptV1, NcesArtifactProviderV1, NcesArtifactReferenceV1,
};
use census_domain::school_directory::SchoolDirectoryEntry;
use census_domain::UsJurisdiction;
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::verified_input::sha256_hex;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReplayManifest {
    pub(super) schema_revision: u32,
    pub(super) month: String,
    pub(super) captures: Vec<ReplayCapture>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReplayCapture {
    pub(super) format: ReplayFormat,
    pub(super) path: std::path::PathBuf,
    pub(super) url: String,
    pub(super) retrieved_at: String,
    pub(super) sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum ReplayFormat {
    NcesCcd,
    NcesPss,
}

impl ReplayFormat {
    const fn provider(self) -> NcesArtifactProviderV1 {
        match self {
            Self::NcesCcd => NcesArtifactProviderV1::Ccd,
            Self::NcesPss => NcesArtifactProviderV1::Pss,
        }
    }
}

pub(super) fn verify_generation(
    generation: &super::super::VerifiedGeneration,
    scope: &[UsJurisdiction],
    ccd: &NcesArtifactReferenceV1,
    pss: &NcesArtifactReferenceV1,
    expected_generation_digest: Option<&str>,
) -> Result<()> {
    let report: super::super::Report =
        serde_json::from_slice(generation.artifact("pipeline_report.json")?)?;
    if report.lanes.len() != 2 {
        bail!("NCES address generation must contain exactly two input lanes");
    }
    if expected_generation_digest.is_some_and(|digest| report.manifest_digest != digest) {
        bail!("NCES address generation digest differs from its Store receipt");
    }
    for (provider, digest) in [("nces-ccd", ccd.sha256()), ("nces-pss", pss.sha256())] {
        if !report
            .lanes
            .iter()
            .any(|lane| lane.source == provider && lane.sha256 == digest)
        {
            bail!("school-address generation does not preserve {provider} digest");
        }
    }
    let entries: Vec<SchoolDirectoryEntry> =
        serde_json::from_slice(generation.artifact("school_directory.json")?)?;
    if entries.iter().any(|entry| {
        entry
            .jurisdiction()
            .is_none_or(|jurisdiction| !scope.contains(&jurisdiction))
    }) {
        bail!("NCES address generation contains a row outside its selected run scope");
    }
    Ok(())
}

pub(super) fn verify_retained_inputs(out: &Path, receipt: &NcesAddressRunReceiptV1) -> Result<()> {
    let archive = out.join("captures").join(receipt.input_manifest_digest());
    let manifest_path = archive.join("manifest.json");
    let manifest_bytes = std::fs::read(&manifest_path)
        .with_context(|| format!("reading retained NCES manifest {}", manifest_path.display()))?;
    if sha256_hex(&manifest_bytes) != receipt.input_manifest_digest()
        || !archive.join(".complete").is_file()
    {
        bail!("retained NCES input manifest is incomplete or has a digest mismatch");
    }
    let manifest: ReplayManifest = serde_json::from_slice(&manifest_bytes)?;
    if manifest.schema_revision != 1 || manifest.captures.len() != 2 {
        bail!("retained NCES input manifest has an unsupported shape");
    }
    for (capture, expected) in manifest.captures.iter().zip([
        (receipt.ccd(), ReplayFormat::NcesCcd),
        (receipt.pss(), ReplayFormat::NcesPss),
    ]) {
        let (artifact, format) = expected;
        if capture.format != format
            || capture.format.provider()
                != if format == ReplayFormat::NcesCcd {
                    receipt.ccd().provider()
                } else {
                    receipt.pss().provider()
                }
            || capture.sha256 != artifact.sha256()
            || capture.url != artifact.source_url()
            || capture.retrieved_at != artifact.captured_on()
            || capture.path.is_absolute()
            || capture.path.components().count() != 1
        {
            bail!("retained NCES capture metadata differs from its Store receipt");
        }
        let body = std::fs::read(archive.join(artifact.sha256()))?;
        if u64::try_from(body.len())? != artifact.byte_length()
            || sha256_hex(&body) != artifact.sha256()
        {
            bail!("retained NCES capture bytes differ from their Store receipt");
        }
    }
    Ok(())
}
