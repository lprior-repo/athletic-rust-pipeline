use super::{invalid, StoreIdentity, StoreResult};
use census_domain::public_identity_history::local_adjudication::{
    LocalIdentityAdjudicationRecordV2, LOCAL_ADJUDICATION_POLICY,
};
use census_domain::public_identity_history::local_membership_withholding::{
    LocalMembershipWithholdingRecordV1, LOCAL_MEMBERSHIP_WITHHOLDING_POLICY,
};
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
    pub(in crate::public_identity) authority_context: &'a str,
    pub(in crate::public_identity) affected_context: &'a str,
}

impl Decision {
    pub(super) fn binding(&self) -> Binding<'_> {
        match self {
            Self::SplitV2(record) => Binding {
                store: &record.store,
                operation: &record.operation,
                authority_context: &record.authority_context,
                affected_context: &record.affected_context,
            },
            Self::MembershipWithholdingV1(record) => Binding {
                store: &record.store,
                operation: &record.operation,
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
}
