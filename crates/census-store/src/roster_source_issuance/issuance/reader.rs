use super::super::{inspection::RosterSourceEvidenceV1, RosterSourceIssuanceStoreError as Error};
use super::ReviewRecord;
use crate::{StoreResult, StoreSnapshot};
use census_domain::evidence_subject_attachment::{
    AttachmentReplayContinuationV1, AttachmentReplayGrantV1,
};
use census_domain::model::StoreIdentity;
use census_domain::roster_affiliation_authority::LocallyCheckedRosterIssuanceV1;
use serde::{Deserialize, Serialize};

mod bundle;
mod current;
mod frozen_source;
mod reconstruct;

pub use bundle::{
    replay_frozen_roster_reviewed_claims_v1, FrozenRosterReviewedClaimsBundleV1,
    RosterFrozenReviewedClaimsReferenceV1,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Archive {
    schema: String,
    store: StoreIdentity,
    selected_snapshot_sequence: u64,
    source_store_lifecycle: u32,
    current_input_token: Vec<u8>,
    metadata: Vec<(Vec<u8>, Vec<u8>)>,
    records: Vec<(Vec<u8>, Vec<u8>)>,
    receipts: Vec<(Vec<u8>, Vec<u8>)>,
    physical: Vec<PhysicalRow>,
    external: Vec<frozen_source::CapturedExternalPrefix>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PhysicalRow {
    table: crate::Table,
    key: Vec<u8>,
    value: Vec<u8>,
}

pub struct CurrentRosterReviewedClaimsV1 {
    archive: Archive,
    claims: Vec<ReviewedRosterSourceClaimV1>,
    continuation: AttachmentReplayContinuationV1,
}

pub struct ReplayedRosterReviewedClaimsV1(CurrentRosterReviewedClaimsV1);

impl ReplayedRosterReviewedClaimsV1 {
    pub fn store_identity(&self) -> &StoreIdentity {
        self.0.store_identity()
    }
    pub fn selected_snapshot_sequence(&self) -> u64 {
        self.0.selected_snapshot_sequence()
    }
    pub fn source_store_lifecycle(&self) -> u32 {
        self.0.source_store_lifecycle()
    }
    pub fn admission_policy(&self) -> &'static str {
        self.0.admission_policy()
    }
    pub fn require_original_grant(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
        self.0.require_original_grant(grant)
    }
    pub fn claims(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&[ReviewedRosterSourceClaimV1]> {
        self.0.claims(grant)
    }
    pub fn authority_head_bytes(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&[u8]> {
        self.0.authority_head_bytes(grant)
    }
}

pub struct ReviewedRosterSourceClaimV1 {
    proposal: LocallyCheckedRosterIssuanceV1,
    evidence: RosterSourceEvidenceV1,
    record: ReviewRecord,
    objects: super::objects::Objects,
}

impl CurrentRosterReviewedClaimsV1 {
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.archive.store
    }
    pub fn selected_snapshot_sequence(&self) -> u64 {
        self.archive.selected_snapshot_sequence
    }
    pub fn source_store_lifecycle(&self) -> u32 {
        self.archive.source_store_lifecycle
    }
    pub fn admission_policy(&self) -> &'static str {
        "exact-inspected-closure"
    }
    pub fn require_original_grant(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
        self.continuation
            .require(grant)
            .map_err(super::super::source_error)
    }
    pub fn claims(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&[ReviewedRosterSourceClaimV1]> {
        self.require_original_grant(grant)?;
        Ok(&self.claims)
    }
    pub fn authority_head_bytes(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&[u8]> {
        self.require_original_grant(grant)?;
        self.archive
            .metadata
            .iter()
            .find(|(key, _)| key == super::super::HEAD.as_bytes())
            .map(|(_, raw)| raw.as_slice())
            .ok_or_else(|| Error::Corrupt("selected authority head absent").into())
    }
}

impl ReviewedRosterSourceClaimV1 {
    pub fn evidence(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&RosterSourceEvidenceV1> {
        self.evidence.require_original_grant(grant)?;
        Ok(&self.evidence)
    }
    pub fn proposal(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&LocallyCheckedRosterIssuanceV1> {
        self.evidence.require_original_grant(grant)?;
        Ok(&self.proposal)
    }
    pub fn operation(&self) -> &str {
        self.record.basis.operation.as_str()
    }
    pub fn object(&self, digest: &str, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&[u8]> {
        self.evidence.require_original_grant(grant)?;
        self.objects
            .get(digest)
            .map(Vec::as_slice)
            .ok_or_else(|| Error::Corrupt("review source object absent").into())
    }
    pub fn semantic_review_bytes(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&[u8]> {
        self.object(&self.record.basis.review.digest, grant)
    }
    pub fn approval_bytes(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&[u8]> {
        self.evidence.require_original_grant(grant)?;
        Ok(&self.record.approval)
    }
    pub fn qualification_object<'a>(
        &'a self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<(&'a str, &'a [u8])> {
        self.review_object(true, grant)
    }
    pub fn activation_object<'a>(
        &'a self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<(&'a str, &'a [u8])> {
        self.review_object(false, grant)
    }
    fn review_object<'a>(
        &'a self,
        qualification: bool,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<(&'a str, &'a [u8])> {
        let bytes = self.semantic_review_bytes(grant)?;
        grant
            .charge_bytes(bytes.len())
            .map_err(super::super::source_error)?;
        grant.charge_steps(1).map_err(super::super::source_error)?;
        let review: super::SemanticReview = super::super::codec::decode(bytes)?;
        let reference = if qualification {
            review.qualification
        } else {
            review.activation
        };
        let (digest, bytes) = self
            .objects
            .get_key_value(&reference.digest)
            .ok_or(Error::Corrupt("review authority object absent"))?;
        self.evidence.require_original_grant(grant)?;
        Ok((digest.as_str(), bytes))
    }
    pub fn original_input_token(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&[u8]> {
        self.object(&self.record.basis.input_token.digest, grant)
    }
}

impl StoreSnapshot<'_> {
    pub fn read_current_roster_reviewed_claims_v1(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<CurrentRosterReviewedClaimsV1> {
        let result = current::read(self, grant);
        if result.is_err() {
            grant.poison();
        }
        result
    }
}
