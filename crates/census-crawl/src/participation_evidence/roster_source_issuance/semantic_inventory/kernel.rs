use super::super::historical::kernel_adapter::{self, CheckedKernelReplayV1, KernelEpoch};
use crate::participation_evidence::{bound, hash, invalid, json};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::StoreResult;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

struct CompiledKernelIdentity {
    text: &'static str,
    digest: String,
    configuration: [u8; 32],
}

static COMPILED_IDENTITY: LazyLock<Option<CompiledKernelIdentity>> = LazyLock::new(|| {
    if validate(COMPILED).is_err() {
        return None;
    }
    let configuration = match kernel_adapter::context::digest(COMPILED) {
        Ok(value) => value,
        Err(_) => return None,
    };
    match std::str::from_utf8(COMPILED) {
        Ok(text) => Some(CompiledKernelIdentity {
            text,
            digest: hash(COMPILED),
            configuration,
        }),
        Err(_) => None,
    }
});

pub(in crate::participation_evidence) fn compiled_identity(
) -> StoreResult<(&'static str, &'static str)> {
    match COMPILED_IDENTITY.as_ref() {
        Some(identity) => Ok((identity.text, identity.digest.as_str())),
        None => Err(invalid("compiled immutable semantic kernel is unsupported")),
    }
}
pub(super) fn require_historical(
    retained: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<CheckedKernelReplayV1> {
    match COMPILED_IDENTITY.as_ref() {
        Some(identity) => kernel_adapter::bind_historical_v1(
            KernelEpoch::FullV4,
            retained,
            identity.text,
            identity.digest.as_str(),
            &identity.configuration,
            grant,
        ),
        None => {
            grant.poison();
            Err(invalid("compiled immutable semantic kernel is unsupported"))
        }
    }
}

const SCHEMA: &str = "roster-semantic-kernel-dependencies/v4";
const MAX_DOCUMENT: usize = 64 * 1024;
const EXPECTED_SOURCE_DIGEST: &str =
    "333559b103fd9853ea189eb78c2b59af8f6b671d452690abdc0e06abe7f8d20a";
const COMPILED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/roster-semantic-kernel-v4.json"));

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Kernel {
    schema: String,
    source_digest: String,
    sources: Vec<Input>,
    configuration: Configuration,
    dependencies: Input,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    files: Vec<Input>,
    compiler: Input,
    environment: Vec<Input>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    path: String,
    sha256: String,
    bytes: u64,
}

fn validate(bytes: &[u8]) -> StoreResult<()> {
    bound(bytes.len(), MAX_DOCUMENT, "semantic kernel document")?;
    let kernel: Kernel = serde_json::from_slice(bytes).map_err(json)?;
    if kernel.schema != SCHEMA || kernel.source_digest != EXPECTED_SOURCE_DIGEST {
        return Err(invalid(
            "unsupported immutable semantic kernel source closure",
        ));
    }
    if kernel.sources.is_empty()
        || kernel.sources.len() > 128
        || kernel.configuration.files.len() != 8
        || kernel.configuration.environment.len() > 128
        || kernel.dependencies.path != "Cargo.lock"
    {
        return Err(invalid("semantic kernel dependency inventory shape"));
    }
    let total = kernel.sources.iter().try_fold(0_u64, |held, input| {
        input_valid(input)?;
        held.checked_add(input.bytes)
            .ok_or(census_store::StoreError::CounterOverflow)
    })?;
    if total > 64 * 1024 * 1024
        || kernel.sources.windows(2).any(|pair| {
            pair.first()
                .zip(pair.get(1))
                .is_none_or(|(a, b)| a.path >= b.path)
        })
    {
        return Err(invalid("semantic kernel source inventory bounds or order"));
    }
    for input in kernel
        .configuration
        .files
        .iter()
        .chain(&kernel.configuration.environment)
        .chain([&kernel.configuration.compiler, &kernel.dependencies])
    {
        input_valid(input)?;
    }
    let sources = serde_json::to_vec(&kernel.sources).map_err(json)?;
    if hash(&sources) != kernel.source_digest || serde_json::to_vec(&kernel).map_err(json)? != bytes
    {
        return Err(invalid("semantic kernel canonical identity mismatch"));
    }
    Ok(())
}

fn input_valid(input: &Input) -> StoreResult<()> {
    if input.path.is_empty()
        || input.path.len() > 512
        || input.path.chars().any(char::is_control)
        || input.sha256.len() != 64
        || !input
            .sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || input.bytes > 4 * 1024 * 1024
    {
        return Err(invalid("semantic kernel dependency input bounds"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn compiled_kernel_binds_digest_and_length() -> TestResult {
        let (text, digest) = compiled_identity()?;
        let mut grant = AttachmentReplayGrantV1::new();
        let checked = require_historical(text.as_bytes(), &mut grant)?;
        let bytes = u64::try_from(text.len())?;
        checked.require_reference(digest, bytes)?;
        check!(checked
            .require_reference("not-the-kernel-digest", bytes)
            .is_err());
        check!(checked
            .require_reference(digest, bytes.checked_add(1).ok_or("length")?)
            .is_err());
        Ok(())
    }

    #[test]
    fn immutable_kernel_refuses_rehashed_source_drift() -> TestResult {
        let mut kernel: Kernel = serde_json::from_slice(COMPILED)?;
        kernel.sources.first_mut().ok_or("kernel source")?.sha256 = "1".repeat(64);
        kernel.source_digest = hash(&serde_json::to_vec(&kernel.sources)?);
        let mut grant = AttachmentReplayGrantV1::new();
        check!(require_historical(&serde_json::to_vec(&kernel)?, &mut grant).is_err());
        check!(require_historical(COMPILED, &mut grant).is_err());
        Ok(())
    }

    #[test]
    fn immutable_kernel_refuses_dependency_drift_and_unknown_schema() -> TestResult {
        for field in ["dependencies", "schema", "retired_predecessor"] {
            let mut kernel: Kernel = serde_json::from_slice(COMPILED)?;
            if field == "dependencies" {
                kernel.dependencies.sha256 = "1".repeat(64);
            } else if field == "retired_predecessor" {
                kernel.schema = "roster-semantic-kernel-dependencies/v1".into();
            } else {
                kernel.schema = "unsupported-kernel".into();
            }
            let mut grant = AttachmentReplayGrantV1::new();
            check!(matches!(
                require_historical(&serde_json::to_vec(&kernel)?, &mut grant),
                Err(census_store::StoreError::Refused { .. })
            ));
        }
        Ok(())
    }
}
