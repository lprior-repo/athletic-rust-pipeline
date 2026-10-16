use crate::public_identity::{PublicIdentityError, PublicIdentityWriterV1};
use crate::{Store, StoreResult};
use census_domain::public_identity_history::local_membership_withholding::MembershipAttachmentV1;
use census_domain::public_identity_history::{HistoryRequest, PublicAthleteId};

impl PublicIdentityWriterV1 {
    pub fn preview_membership_withholding(
        &self,
        store: &Store,
        canonical: &PublicAthleteId,
        attachment: &MembershipAttachmentV1,
    ) -> StoreResult<Vec<String>> {
        let _guard = store.lock_appends();
        let ledger = self.verified(store)?;
        self.require_current_qualification()?;
        let record = ledger
            .history
            .records()
            .iter()
            .find(|record| record.sequence == attachment.sequence)
            .ok_or(PublicIdentityError::Corrupt("preview attachment missing"))?;
        if record.digest != attachment.digest
            || !matches!(&record.request,
            HistoryRequest::BindCandidateMembership { membership, .. } if membership.id == attachment.membership_id)
        {
            return Err(PublicIdentityError::Corrupt("preview attachment binding").into());
        }
        ledger
            .history
            .projection()
            .preview_withheld_memberships(canonical, attachment.sequence)
            .map_err(PublicIdentityError::from)
            .map_err(Into::into)
    }
}
