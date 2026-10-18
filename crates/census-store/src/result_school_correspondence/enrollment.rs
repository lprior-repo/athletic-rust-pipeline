use super::*;
use crate::{Application, Store, StoreResult, StoreRevision};
use census_domain::evidence_subject_attachment::AttachmentPublicHeadV1;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::StoreIdentity;
use chrono::{DateTime, Utc};

mod apply;
pub(super) mod inspect;
mod prepare;
mod replay;

pub struct ResultSchoolEnrollmentInputsV1<'s> {
    revision: StoreRevision<'s>,
    store: StoreIdentity,
    prior: ResultSchoolPriorCapabilityV1,
    token: ResultSchoolDigestV1,
    head: AttachmentPublicHeadV1,
    grant: AttachmentReplayGrantV1,
}

pub struct ResultSchoolEnrollmentReviewV1 {
    pub operation: ResultSchoolTextV1,
    pub actor: ResultSchoolReviewActorV1,
    pub reviewed_at: DateTime<Utc>,
    pub rationale: ResultSchoolTextV1,
}

pub struct PreparedResultSchoolEnrollmentV1<'s> {
    revision: StoreRevision<'s>,
    command: ResultSchoolEnrollmentCommandV1,
    approval: Vec<u8>,
    approval_digest: ResultSchoolDigestV1,
    replay: Option<crate::Receipt>,
    grant: AttachmentReplayGrantV1,
}

impl ResultSchoolEnrollmentInputsV1<'_> {
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.store
    }
    pub fn prior_capability(&self) -> &ResultSchoolPriorCapabilityV1 {
        &self.prior
    }
    pub fn input_token_digest(&self) -> &ResultSchoolDigestV1 {
        &self.token
    }
    pub fn accepted_head(&self) -> &AttachmentPublicHeadV1 {
        &self.head
    }
    pub fn evidence_generation(&self) -> u64 {
        self.revision.evidence_generation()
    }
    pub fn derived_generation(&self) -> u64 {
        self.revision.derived_generation()
    }
}

impl PreparedResultSchoolEnrollmentV1<'_> {
    pub fn approval_bytes(&self) -> &[u8] {
        &self.approval
    }
    pub fn approval_digest(&self) -> &ResultSchoolDigestV1 {
        &self.approval_digest
    }
    pub fn replay_only(&self) -> bool {
        self.replay.is_some()
    }
}

impl Store {
    pub fn inspect_result_school_enrollment_v1(
        &self,
        grant: AttachmentReplayGrantV1,
    ) -> StoreResult<ResultSchoolEnrollmentInputsV1<'_>> {
        let _lock = self.lock_appends();
        let (inspected, grant) = inspect::inspect(self, grant)?;
        Ok(ResultSchoolEnrollmentInputsV1 {
            revision: inspected.revision,
            store: inspected.store,
            prior: inspected.prior,
            token: inspected.token,
            head: inspected.head,
            grant,
        })
    }

    pub fn prepare_result_school_enrollment_v1<'s>(
        &'s self,
        inputs: ResultSchoolEnrollmentInputsV1<'s>,
        review: ResultSchoolEnrollmentReviewV1,
    ) -> StoreResult<PreparedResultSchoolEnrollmentV1<'s>> {
        let _lock = self.lock_appends();
        prepare::prepare(self, inputs, review)
    }

    pub fn apply_result_school_enrollment_v1(
        &self,
        prepared: PreparedResultSchoolEnrollmentV1<'_>,
        approved: &[u8],
        approval_digest: &ResultSchoolDigestV1,
    ) -> StoreResult<Application> {
        let _lock = self.lock_appends();
        apply::apply(self, prepared, approved, approval_digest)
    }
}
