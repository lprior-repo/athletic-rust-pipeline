use super::super::{RosterAuthorityError, RosterAuthorityLimitsV1};
use super::{charge, size, Result};
use crate::model::CanonicalJsonError;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::borrow::Cow;

const SCHEMA: &str = "roster-member-semantic-witness/v1";

pub(super) fn encode(
    index: usize,
    preimage: &[u8],
    limits: RosterAuthorityLimitsV1,
) -> Result<Vec<u8>> {
    admit(preimage, limits)?;
    let value: &RawValue = serde_json::from_slice(preimage)?;
    let wire = Wire {
        schema: Cow::Borrowed(SCHEMA),
        inventory_index: index,
        semantic_preimage: value,
    };
    let bytes = serde_json::to_vec(&wire)
        .map_err(|error| CanonicalJsonError::Unsupported(error.to_string()))?;
    admit(&bytes, limits)?;
    Ok(bytes)
}

pub(super) fn decode(witness: &[u8]) -> Result<Wire<'_>> {
    let wire: Wire<'_> = serde_json::from_slice(witness)?;
    if wire.schema != SCHEMA {
        return Err(RosterAuthorityError::Invalid("witness schema"));
    }
    Ok(wire)
}

fn admit(bytes: &[u8], limits: RosterAuthorityLimitsV1) -> Result<()> {
    if bytes.is_empty() {
        return Err(RosterAuthorityError::Invalid("empty witness"));
    }
    charge(
        "witness bytes",
        size(bytes.len())?,
        size(limits.record_bytes)?,
    )
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Wire<'a> {
    #[serde(borrow)]
    schema: Cow<'a, str>,
    pub(super) inventory_index: usize,
    #[serde(borrow)]
    pub(super) semantic_preimage: &'a RawValue,
}
