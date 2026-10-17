use std::io;

#[derive(Debug, thiserror::Error)]
pub enum FixtureInputError {
    #[error("fixture I/O: {0}")]
    Io(#[from] io::Error),
    #[error("fixture JSON: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("invalid fixture: {0}")]
    Invalid(&'static str),
}

pub type Result<T> = std::result::Result<T, FixtureInputError>;
const WIRE_ENCODED: u64 = 33_554_471;
const DECODED_ENCODED_MAX: u64 = 33_554_432;
const DECODED_SIZE: u64 = 33_554_433;
pub(super) const NORMAL_BODY: &[u8] = b"wire-smoke-ok";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactManifest {
    pub encoded: u64,
    pub decoded: u64,
    pub sha256: String,
    pub decoded_sha256: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureManifest {
    #[serde(rename = "fixtureDir")]
    pub fixture_dir: String,
    #[serde(rename = "normalBody")]
    pub normal_body: String,
    pub wire: ArtifactManifest,
    pub decoded: ArtifactManifest,
    pub scope: String,
}

#[derive(Clone, Copy)]
pub enum ArtifactKind {
    Wire,
    Decoded,
}

impl ArtifactKind {
    pub(super) fn validate(self, artifact: &ArtifactManifest) -> Result<()> {
        let valid = match self {
            Self::Wire => artifact.encoded == WIRE_ENCODED && artifact.decoded == 13,
            Self::Decoded => {
                (1..=DECODED_ENCODED_MAX).contains(&artifact.encoded)
                    && artifact.decoded == DECODED_SIZE
            }
        };
        if !valid {
            return Err(FixtureInputError::Invalid("artifact dimensions"));
        }
        for digest in [&artifact.sha256, &artifact.decoded_sha256] {
            if digest.len() != 64
                || !digest
                    .bytes()
                    .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
            {
                return Err(FixtureInputError::Invalid("artifact digest"));
            }
        }
        Ok(())
    }
}

pub fn validate_manifest(manifest: &FixtureManifest) -> Result<()> {
    if manifest.normal_body.as_bytes() != NORMAL_BODY
        || manifest.fixture_dir.is_empty()
        || manifest.scope.is_empty()
    {
        return Err(FixtureInputError::Invalid("manifest identity"));
    }
    ArtifactKind::Wire.validate(&manifest.wire)?;
    ArtifactKind::Decoded.validate(&manifest.decoded)
}
