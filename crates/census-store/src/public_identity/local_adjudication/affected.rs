use crate::public_identity::{context, PublicIdentityError};
use crate::{StoreError, StoreResult};
use census_domain::model::StoreIdentity;
use census_domain::public_identity_history::local_adjudication::LocalIdentityAdjudicationRecordV2;
use serde::{Deserialize, Serialize};

mod capture;
mod decode;
mod keys;
pub(in crate::public_identity) use keys::KeyInventory;

pub(in crate::public_identity) use decode::{from_bytes, read};

const SCHEMA: &str = "local-identity-affected-records/v1";
const MAX_KEY_BYTES: usize = 1024;
const MAX_KEY_BASE64_BYTES: usize = MAX_KEY_BYTES.div_ceil(3) * 4;

pub(in crate::public_identity) struct Inputs {
    header: Header,
    performances: Vec<Physical>,
    relays: Vec<Physical>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    schema: String,
    store: StoreIdentity,
    authority_context: String,
    evidence_generation: u64,
    derived_generation: u64,
    snapshot_sequence: u64,
    counts: [usize; 2],
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::public_identity) struct Physical {
    physical_key_base64: String,
    raw_json: String,
}

impl Inputs {
    pub(in crate::public_identity) fn validate(
        &self,
        identity: &context::Inputs,
        record: &LocalIdentityAdjudicationRecordV2,
    ) -> StoreResult<()> {
        if self.header.store != record.store
            || self.header.authority_context != record.authority_context
            || self.header.evidence_generation != record.evidence_generation
            || self.header.derived_generation != record.derived_generation
            || self.header.snapshot_sequence != record.snapshot_sequence
        {
            return Err(invalid("manual affected snapshot binding"));
        }
        if self.record_keys(identity, &record.members)? != record.request.affected_record_keys {
            return Err(invalid("manual affected record inventory"));
        }
        Ok(())
    }
    pub(in crate::public_identity) fn validate_withholding(
        &self,
        identity: &context::Inputs,
        record: &census_domain::public_identity_history::local_membership_withholding::LocalMembershipWithholdingRecordV1,
    ) -> StoreResult<()> {
        if self.header.store != record.store
            || self.header.authority_context != record.authority_context
            || self.header.evidence_generation != record.evidence_generation
            || self.header.derived_generation != record.derived_generation
            || self.header.snapshot_sequence != record.snapshot_sequence
        {
            return Err(invalid("membership affected snapshot binding"));
        }
        if self.record_keys(identity, &record.members)? != record.request.affected_record_keys {
            return Err(invalid("membership affected record inventory"));
        }
        Ok(())
    }
}

fn invalid(detail: &'static str) -> StoreError {
    PublicIdentityError::Corrupt(detail).into()
}

fn allocation(error: std::collections::TryReserveError) -> StoreError {
    StoreError::Refused {
        detail: format!("manual affected inventory allocation: {error}"),
    }
}

#[cfg(test)]
mod tests;
