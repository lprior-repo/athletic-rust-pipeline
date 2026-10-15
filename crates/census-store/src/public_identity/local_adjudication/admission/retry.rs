use super::{codec, invalid, Reservation, StoreResult, MAX_JOURNAL_VALUE_BYTES};
use census_domain::public_identity_history::local_adjudication::{
    LocalIdentityAdjudicationRecordV2, LocalIdentitySplitV1,
};
use census_domain::public_identity_history::local_membership_withholding::{
    LocalMembershipWithholdV1, LocalMembershipWithholdingRecordV1,
};
use census_domain::public_identity_history::RegistryHead;

impl Reservation {
    pub(in crate::public_identity) fn split_record(
        &self,
    ) -> StoreResult<&LocalIdentityAdjudicationRecordV2> {
        self.decision.split()
    }

    pub(in crate::public_identity) fn withholding_record(
        &self,
    ) -> StoreResult<&LocalMembershipWithholdingRecordV1> {
        self.decision.withholding()
    }

    pub(in crate::public_identity) fn require_request(
        &self,
        operation: &str,
        request: &LocalIdentitySplitV1,
        expected: crate::StoreRevision<'_>,
        head: &RegistryHead,
    ) -> StoreResult<()> {
        let original = self.split_record()?;
        self.decision
            .binding()
            .require_current(operation, expected, head)?;
        if codec::encode(&original.request, MAX_JOURNAL_VALUE_BYTES)?
            != codec::encode(request, MAX_JOURNAL_VALUE_BYTES)?
        {
            return Err(invalid());
        }
        Ok(())
    }

    pub(in crate::public_identity) fn require_withholding_request(
        &self,
        operation: &str,
        request: &LocalMembershipWithholdV1,
        expected: crate::StoreRevision<'_>,
        head: &RegistryHead,
    ) -> StoreResult<()> {
        let original = self.withholding_record()?;
        self.decision
            .binding()
            .require_current(operation, expected, head)?;
        if codec::encode(&original.request, MAX_JOURNAL_VALUE_BYTES)?
            != codec::encode(request, MAX_JOURNAL_VALUE_BYTES)?
        {
            return Err(invalid());
        }
        Ok(())
    }

    pub(in crate::public_identity) fn restore_affected(
        &self,
        affected: &mut super::super::affected::Inputs,
    ) -> StoreResult<()> {
        affected.restore_admission_binding(&self.decision.binding())
    }
}
