use super::{RunManifest, SchoolYear};
use crate::UsJurisdiction;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

const FIVE_STATE_SCOPE: [UsJurisdiction; 5] = [
    UsJurisdiction::Michigan,
    UsJurisdiction::Minnesota,
    UsJurisdiction::Iowa,
    UsJurisdiction::Illinois,
    UsJurisdiction::Colorado,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NcesArtifactProviderV1 {
    Ccd,
    Pss,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NcesArtifactReferenceV1 {
    provider: NcesArtifactProviderV1,
    sha256: String,
    byte_length: u64,
    source_url: String,
    captured_on: String,
}

impl NcesArtifactReferenceV1 {
    pub fn new(
        provider: NcesArtifactProviderV1,
        sha256: impl Into<String>,
        byte_length: u64,
        source_url: impl Into<String>,
        captured_on: impl Into<String>,
    ) -> Result<Self, NcesArtifactReceiptError> {
        let value = Self {
            provider,
            sha256: sha256.into(),
            byte_length,
            source_url: source_url.into(),
            captured_on: captured_on.into(),
        };
        value.validate()?;
        Ok(value)
    }

    pub const fn provider(&self) -> NcesArtifactProviderV1 {
        self.provider
    }

    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    pub const fn byte_length(&self) -> u64 {
        self.byte_length
    }

    pub fn source_url(&self) -> &str {
        &self.source_url
    }

    pub fn captured_on(&self) -> &str {
        &self.captured_on
    }

    fn validate(&self) -> Result<(), NcesArtifactReceiptError> {
        if self.sha256.len() != 64
            || !self
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(NcesArtifactReceiptError::Digest);
        }
        if self.byte_length == 0 || self.byte_length > 128 * 1024 * 1024 {
            return Err(NcesArtifactReceiptError::ByteLength);
        }
        let url = url::Url::parse(&self.source_url)
            .map_err(|_| NcesArtifactReceiptError::SourceUrl)?;
        if url.scheme() != "https" || url.host_str().is_none() || !url.username().is_empty() {
            return Err(NcesArtifactReceiptError::SourceUrl);
        }
        NaiveDate::parse_from_str(&self.captured_on, "%Y-%m-%d")
            .map_err(|_| NcesArtifactReceiptError::CaptureDate)?;
        Ok(())
    }
}

/// Immutable lineage for the selected run's CCD/PSS school-address inputs.
///
/// This is stored separately from `RunManifest` so old run JSON and its canonical digest remain
/// byte-for-byte compatible. The receipt repeats the run manifest digest and the fixed selected
/// scope so it cannot be rebound to another run or profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NcesAddressRunReceiptV1 {
    schema_revision: u32,
    run_manifest_digest: String,
    scope: Vec<UsJurisdiction>,
    ccd: NcesArtifactReferenceV1,
    pss: NcesArtifactReferenceV1,
    input_manifest_digest: String,
    address_generation_digest: String,
}

impl NcesAddressRunReceiptV1 {
    pub fn new(
        run_manifest_digest: impl Into<String>,
        scope: Vec<UsJurisdiction>,
        ccd: NcesArtifactReferenceV1,
        pss: NcesArtifactReferenceV1,
        input_manifest_digest: impl Into<String>,
        address_generation_digest: impl Into<String>,
    ) -> Result<Self, NcesArtifactReceiptError> {
        let value = Self {
            schema_revision: 1,
            run_manifest_digest: run_manifest_digest.into(),
            scope,
            ccd,
            pss,
            input_manifest_digest: input_manifest_digest.into(),
            address_generation_digest: address_generation_digest.into(),
        };
        value.validate()?;
        Ok(value)
    }

    pub const fn schema_revision(&self) -> u32 {
        self.schema_revision
    }

    pub fn run_manifest_digest(&self) -> &str {
        &self.run_manifest_digest
    }

    pub fn scope(&self) -> &[UsJurisdiction] {
        &self.scope
    }

    pub fn ccd(&self) -> &NcesArtifactReferenceV1 {
        &self.ccd
    }

    pub fn pss(&self) -> &NcesArtifactReferenceV1 {
        &self.pss
    }

    pub fn input_manifest_digest(&self) -> &str {
        &self.input_manifest_digest
    }

    pub fn address_generation_digest(&self) -> &str {
        &self.address_generation_digest
    }

    pub fn validate_for_run(
        &self,
        run: &RunManifest,
    ) -> Result<(), NcesArtifactReceiptError> {
        self.validate()?;
        if self.scope != run.jurisdictions
            || run.jurisdictions.as_slice() != FIVE_STATE_SCOPE.as_slice()
            || run.run.season() != SchoolYear::new(2026).ok_or(NcesArtifactReceiptError::Run)?
            || run.cohort.get() != 2027
        {
            return Err(NcesArtifactReceiptError::Run);
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), NcesArtifactReceiptError> {
        if self.schema_revision != 1
            || !valid_digest(&self.run_manifest_digest)
            || !valid_digest(&self.input_manifest_digest)
            || !valid_digest(&self.address_generation_digest)
            || self.ccd.provider != NcesArtifactProviderV1::Ccd
            || self.pss.provider != NcesArtifactProviderV1::Pss
        {
            return Err(NcesArtifactReceiptError::Receipt);
        }
        self.ccd.validate()?;
        self.pss.validate()?;
        if self.scope.as_slice() != FIVE_STATE_SCOPE.as_slice() {
            return Err(NcesArtifactReceiptError::Scope);
        }
        Ok(())
    }
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NcesArtifactReceiptError {
    #[error("NCES artifact digest is not lowercase SHA-256")]
    Digest,
    #[error("NCES artifact byte length is outside the accepted bound")]
    ByteLength,
    #[error("NCES artifact source URL must be an absolute HTTPS URL without credentials")]
    SourceUrl,
    #[error("NCES artifact capture date must be YYYY-MM-DD")]
    CaptureDate,
    #[error("NCES receipt schema or digest fields are invalid")]
    Receipt,
    #[error("NCES receipt must bind exactly the selected MI/MN/IA/IL/CO scope")]
    Scope,
    #[error("NCES receipt does not match a 2026-27, class-of-2027 selected run")]
    Run,
}

