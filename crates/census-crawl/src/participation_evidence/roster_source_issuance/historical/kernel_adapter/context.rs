use crate::participation_evidence::{bound, json};
use census_store::StoreResult;
use serde::Deserialize;
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};

const MAX_DOCUMENT: usize = 64 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KernelDoc<'a> {
    #[serde(rename = "schema", borrow)]
    _schema: &'a RawValue,
    #[serde(rename = "source_digest", borrow)]
    _source_digest: &'a RawValue,
    #[serde(rename = "sources", borrow)]
    _sources: &'a RawValue,
    #[serde(borrow)]
    configuration: &'a RawValue,
    #[serde(borrow)]
    dependencies: &'a RawValue,
}

pub(in crate::participation_evidence::roster_source_issuance) fn digest(
    document: &[u8],
) -> StoreResult<[u8; 32]> {
    bound(document.len(), MAX_DOCUMENT, "kernel context document")?;
    let kernel: KernelDoc<'_> = serde_json::from_slice(document).map_err(json)?;
    let configuration_bytes = kernel.configuration.get().as_bytes();
    let dependencies_bytes = kernel.dependencies.get().as_bytes();
    let configuration_length = u64::try_from(configuration_bytes.len())
        .map_err(|_| census_store::StoreError::CounterOverflow)?;
    let mut hasher = Sha256::new();
    hasher.update(configuration_length.to_le_bytes());
    hasher.update(configuration_bytes);
    hasher.update(dependencies_bytes);
    Ok(hasher.finalize().into())
}
