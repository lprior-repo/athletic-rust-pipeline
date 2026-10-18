use crate::participation_evidence::{bound, hash, invalid, json};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::{EvidenceSubjectAttachmentStoreError, StoreResult};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

struct CompiledKernelIdentity {
    text: &'static str,
    digest: String,
}

static COMPILED_IDENTITY: LazyLock<Option<CompiledKernelIdentity>> = LazyLock::new(|| {
    if validate(COMPILED).is_err() {
        return None;
    }
    match std::str::from_utf8(COMPILED) {
        Ok(text) => Some(CompiledKernelIdentity {
            text,
            digest: hash(COMPILED),
        }),
        Err(_) => None,
    }
});

pub(in crate::participation_evidence) fn compiled() -> StoreResult<&'static [u8]> {
    compiled_identity()?;
    Ok(COMPILED)
}

pub(in crate::participation_evidence) fn compiled_identity(
) -> StoreResult<(&'static str, &'static str)> {
    match COMPILED_IDENTITY.as_ref() {
        Some(identity) => Ok((identity.text, identity.digest.as_str())),
        None => Err(invalid("compiled immutable semantic kernel is unsupported")),
    }
}

const SCHEMA: &str = "roster-semantic-kernel-dependencies/v3";
const MAX_DOCUMENT: usize = 64 * 1024;
const EXPECTED_SOURCE_DIGEST: &str =
    "1226f147c451534ac2c613d5683156570cdae3529f9f52d2bf489523a2d518df";
const COMPILED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/roster-semantic-kernel-v3.json"));

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

pub(in crate::participation_evidence) fn require_retained(
    retained: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let result = (|| {
        bound(
            retained.len(),
            MAX_DOCUMENT,
            "retained semantic kernel document",
        )?;
        grant
            .charge_steps(1)
            .map_err(EvidenceSubjectAttachmentStoreError::Domain)?;
        grant
            .charge_bytes(retained.len())
            .map_err(EvidenceSubjectAttachmentStoreError::Domain)?;
        if retained != compiled()? {
            return Err(invalid(
                "retained semantic kernel dependencies differ from supported build",
            ));
        }
        Ok(())
    })();
    if result.is_err() {
        grant.poison();
    }
    result
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
    fn immutable_kernel_accepts_only_exact_compiled_dependency_evidence() -> TestResult {
        let bytes = compiled()?;
        let mut grant = AttachmentReplayGrantV1::new();
        require_retained(bytes, &mut grant)?;
        Ok(())
    }

    #[test]
    fn immutable_kernel_refuses_rehashed_source_drift() -> TestResult {
        let mut kernel: Kernel = serde_json::from_slice(compiled()?)?;
        kernel.sources.first_mut().ok_or("kernel source")?.sha256 = "1".repeat(64);
        kernel.source_digest = hash(&serde_json::to_vec(&kernel.sources)?);
        let mut grant = AttachmentReplayGrantV1::new();
        check!(require_retained(&serde_json::to_vec(&kernel)?, &mut grant).is_err());
        check!(require_retained(compiled()?, &mut grant).is_err());
        Ok(())
    }

    #[test]
    fn immutable_kernel_refuses_dependency_drift_and_unknown_schema() -> TestResult {
        for field in ["dependencies", "schema", "retired_predecessor"] {
            let mut kernel: Kernel = serde_json::from_slice(compiled()?)?;
            if field == "dependencies" {
                kernel.dependencies.sha256 = "1".repeat(64);
            } else if field == "retired_predecessor" {
                kernel.schema = "roster-semantic-kernel-dependencies/v2".into();
            } else {
                kernel.schema = "unsupported-kernel".into();
            }
            let mut grant = AttachmentReplayGrantV1::new();
            check!(matches!(
                require_retained(&serde_json::to_vec(&kernel)?, &mut grant),
                Err(census_store::StoreError::Refused { .. })
            ));
        }
        Ok(())
    }
}
