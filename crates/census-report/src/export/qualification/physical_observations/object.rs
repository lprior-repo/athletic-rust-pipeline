use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ObjectRef {
    pub(super) digest: String,
    pub(super) bytes: u64,
}

impl ObjectRef {
    pub(super) fn validate(&self, ceiling: u64) -> StoreResult<()> {
        if !digest(&self.digest) || self.bytes == 0 || self.bytes > ceiling {
            return Err(invalid("physical proof object reference is invalid"));
        }
        Ok(())
    }
}

pub(super) fn encode(directory: &Path, value: &impl Serialize) -> ReportResult<ObjectRef> {
    let bytes = census_domain::model::serialized_bytes(value).map_err(StoreError::from)?;
    let length = u64::try_from(bytes.len()).map_err(|_| StoreError::CounterOverflow)?;
    if length > INVENTORY_BYTES {
        return Err(StoreError::BudgetExceeded {
            what: "physical proof inventory bytes",
            needed: length,
            available: INVENTORY_BYTES,
        }
        .into());
    }
    Ok(ObjectRef {
        digest: proof_objects::install(directory, &bytes)?,
        bytes: length,
    })
}

pub(super) fn decode<T: serde::de::DeserializeOwned>(
    directory: &Path,
    reference: &ObjectRef,
) -> ReportResult<T> {
    let bytes = read(directory, reference, INVENTORY_BYTES)?;
    serde_json::from_slice(&bytes).map_err(|source| {
        StoreError::Decode {
            key: reference.digest.clone(),
            source,
        }
        .into()
    })
}

pub(super) fn read(directory: &Path, reference: &ObjectRef, ceiling: u64) -> ReportResult<Vec<u8>> {
    reference.validate(ceiling)?;
    let bytes = proof_objects::read(directory, &reference.digest, ceiling)?;
    if u64::try_from(bytes.len()).map_err(|_| StoreError::CounterOverflow)? != reference.bytes {
        return Err(invalid("physical proof object differs from its exact byte count").into());
    }
    Ok(bytes)
}
