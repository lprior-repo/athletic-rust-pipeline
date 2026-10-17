use super::{codec, format, hash, json};
use census_domain::roster_affiliation_authority::RosterParserClaimV1;
use census_store::{StoreError, StoreResult};
use serde::Serialize;

#[derive(Serialize)]
struct Reference {
    digest: String,
    bytes: u64,
}

#[derive(Serialize)]
struct Claim<'a> {
    artifact: Reference,
    interpretation: &'a str,
    schema: Reference,
    configuration: Reference,
    dependencies: Reference,
}

pub(super) fn derive(value: &format::Manifest, original: &[u8]) -> StoreResult<RosterParserClaimV1> {
    let claim = Claim {
        artifact: reference(original)?,
        interpretation: &value.semantic_policy,
        schema: reference(&codec::encode(&value.schema, format::MAX_MANIFEST)?)?,
        configuration: reference(&codec::encode(&value.configuration, format::MAX_MANIFEST)?)?,
        dependencies: reference(&codec::encode(&value.dependencies, format::MAX_MANIFEST)?)?,
    };
    serde_json::from_slice(&codec::encode(&claim, 4096)?).map_err(json)
}

fn reference(bytes: &[u8]) -> StoreResult<Reference> {
    Ok(Reference {
        digest: hash(bytes),
        bytes: u64::try_from(bytes.len()).map_err(|_| StoreError::CounterOverflow)?,
    })
}
