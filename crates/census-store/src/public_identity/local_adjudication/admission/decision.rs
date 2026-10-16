use super::{invalid, StoreIdentity, StoreResult};
use census_domain::public_identity_history::local_adjudication::{
    LocalIdentityAdjudicationRecordV2, LOCAL_ADJUDICATION_POLICY,
};
use census_domain::public_identity_history::local_membership_withholding::{
    LocalMembershipWithholdingRecordV1, LOCAL_MEMBERSHIP_WITHHOLDING_POLICY,
};
#[cfg(test)]
use census_domain::public_identity_history::RegistryHead;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", content = "record", deny_unknown_fields)]
pub(super) enum Decision {
    SplitV2(LocalIdentityAdjudicationRecordV2),
    MembershipWithholdingV1(LocalMembershipWithholdingRecordV1),
}

pub(in crate::public_identity) struct Binding<'a> {
    pub(in crate::public_identity) store: &'a StoreIdentity,
    pub(in crate::public_identity) operation: &'a str,
    #[cfg(test)]
    pub(in crate::public_identity) expected_head: &'a RegistryHead,
    #[cfg(test)]
    pub(in crate::public_identity) evidence_generation: u64,
    #[cfg(test)]
    pub(in crate::public_identity) derived_generation: u64,
    #[cfg(test)]
    pub(in crate::public_identity) snapshot_sequence: u64,
    pub(in crate::public_identity) authority_context: &'a str,
    pub(in crate::public_identity) affected_context: &'a str,
}

impl Decision {
    pub(super) fn binding(&self) -> Binding<'_> {
        match self {
            Self::SplitV2(record) => Binding {
                store: &record.store,
                operation: &record.operation,
                #[cfg(test)]
                expected_head: &record.expected_head,
                #[cfg(test)]
                evidence_generation: record.evidence_generation,
                #[cfg(test)]
                derived_generation: record.derived_generation,
                #[cfg(test)]
                snapshot_sequence: record.snapshot_sequence,
                authority_context: &record.authority_context,
                affected_context: &record.affected_context,
            },
            Self::MembershipWithholdingV1(record) => Binding {
                store: &record.store,
                operation: &record.operation,
                #[cfg(test)]
                expected_head: &record.expected_head,
                #[cfg(test)]
                evidence_generation: record.evidence_generation,
                #[cfg(test)]
                derived_generation: record.derived_generation,
                #[cfg(test)]
                snapshot_sequence: record.snapshot_sequence,
                authority_context: &record.authority_context,
                affected_context: &record.affected_context,
            },
        }
    }

    pub(super) fn require_schema(&self) -> StoreResult<()> {
        let valid = match self {
            Self::SplitV2(record) => {
                record.version == 2 && record.policy == LOCAL_ADJUDICATION_POLICY
            }
            Self::MembershipWithholdingV1(record) => {
                record.version == 1 && record.policy == LOCAL_MEMBERSHIP_WITHHOLDING_POLICY
            }
        };
        if !valid {
            return Err(invalid());
        }
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn split(&self) -> StoreResult<&LocalIdentityAdjudicationRecordV2> {
        match self {
            Self::SplitV2(record) => Ok(record),
            _ => Err(invalid()),
        }
    }

    #[cfg(test)]
    pub(super) fn withholding(&self) -> StoreResult<&LocalMembershipWithholdingRecordV1> {
        match self {
            Self::MembershipWithholdingV1(record) => Ok(record),
            _ => Err(invalid()),
        }
    }
}

#[cfg(test)]
impl Binding<'_> {
    pub(super) fn require_current(
        &self,
        operation: &str,
        expected: crate::StoreRevision<'_>,
        head: &RegistryHead,
    ) -> StoreResult<()> {
        if self.operation != operation
            || self.expected_head != head
            || self.evidence_generation != expected.evidence_generation()
            || self.derived_generation != expected.derived_generation()
        {
            return Err(invalid());
        }
        Ok(())
    }
}
