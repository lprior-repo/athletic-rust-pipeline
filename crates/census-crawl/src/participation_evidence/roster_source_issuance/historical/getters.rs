use super::*;
use census_domain::roster_affiliation_authority::LocallyCheckedRosterIssuanceV1;

impl HistoricalReviewedRosterSourceV2<'_> {
    pub fn require_original_grant(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
        self.claim.evidence(grant)?.require_original_grant(grant)
    }
    pub fn proposal(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&LocallyCheckedRosterIssuanceV1> {
        self.claim.proposal(grant)
    }
    pub fn checked_packet(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&CheckedProviderSchoolPacketV1> {
        self.require_original_grant(grant)?;
        Ok(&self.checked)
    }
    pub fn school(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&CanonicalSchool> {
        self.require_original_grant(grant)?;
        Ok(&self.school)
    }
    pub fn program(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&CanonicalTeam> {
        self.require_original_grant(grant)?;
        Ok(&self.program)
    }
    pub fn roster_body(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&str> {
        self.require_original_grant(grant)?;
        Ok(&self.body)
    }
    pub fn metadata_bytes(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&[u8]> {
        self.objects.metadata_bytes(grant)
    }
    pub fn interpretation_bytes(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&[u8]> {
        self.require_original_grant(grant)?;
        Ok(self.interpretation)
    }
    pub fn qualification_object(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<(&str, &[u8])> {
        self.require_original_grant(grant)?;
        self.claim.qualification_object(grant)
    }
    pub fn activation_object(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<(&str, &[u8])> {
        self.require_original_grant(grant)?;
        self.claim.activation_object(grant)
    }
}
