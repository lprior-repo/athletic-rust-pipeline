use super::*;
use crate::{Application, Store, StoreRevision};
use census_domain::evidence_subject_attachment::AttachmentPublicHeadV1;
use census_domain::model::StoreIdentity;
use chrono::{DateTime, Utc};
use serde_json::value::RawValue;

mod apply;
pub(super) mod inspect;
mod prepare;
mod replay;

pub struct ResultCalendarEnrollmentInputsV1<'s> {
    revision: StoreRevision<'s>,
    store: StoreIdentity,
    prior_school: Vec<u8>,
    prior_digest: ResultCalendarDigestV1,
    token: ResultCalendarDigestV1,
    head: AttachmentPublicHeadV1,
    grant: AttachmentReplayGrantV1,
}

pub struct ResultCalendarEnrollmentReviewV1 {
    pub operation: ResultCalendarTextV1,
    pub actor: ResultCalendarReviewActorV1,
    pub reviewed_at: DateTime<Utc>,
    pub rationale: ResultCalendarTextV1,
}

pub struct PreparedResultCalendarEnrollmentV1<'s> {
    revision: StoreRevision<'s>,
    command: ResultCalendarEnrollmentCommandV1,
    prior_school: Box<RawValue>,
    approval: Vec<u8>,
    approval_digest: ResultCalendarDigestV1,
    replay: Option<crate::Receipt>,
    grant: AttachmentReplayGrantV1,
}

impl ResultCalendarEnrollmentInputsV1<'_> {
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.store
    }
    pub fn prior_school_digest(&self) -> &ResultCalendarDigestV1 {
        &self.prior_digest
    }
    pub fn input_token_digest(&self) -> &ResultCalendarDigestV1 {
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

impl PreparedResultCalendarEnrollmentV1<'_> {
    pub fn approval_bytes(&self) -> &[u8] {
        &self.approval
    }
    pub fn approval_digest(&self) -> &ResultCalendarDigestV1 {
        &self.approval_digest
    }
    pub fn replay_only(&self) -> bool {
        self.replay.is_some()
    }
}

impl Store {
    pub fn inspect_result_calendar_enrollment_v1(
        &self,
        grant: AttachmentReplayGrantV1,
    ) -> StoreResult<ResultCalendarEnrollmentInputsV1<'_>> {
        let _lock = self.lock_appends();
        let (inspected, grant) = inspect::inspect(self, grant)?;
        Ok(ResultCalendarEnrollmentInputsV1 {
            revision: inspected.revision,
            store: inspected.store,
            prior_school: inspected.prior_school,
            prior_digest: inspected.prior_digest,
            token: inspected.token,
            head: inspected.head,
            grant,
        })
    }
    pub fn prepare_result_calendar_enrollment_v1<'s>(
        &'s self,
        inputs: ResultCalendarEnrollmentInputsV1<'s>,
        review: ResultCalendarEnrollmentReviewV1,
    ) -> StoreResult<PreparedResultCalendarEnrollmentV1<'s>> {
        let _lock = self.lock_appends();
        prepare::prepare(self, inputs, review)
    }
    pub fn apply_result_calendar_enrollment_v1(
        &self,
        prepared: PreparedResultCalendarEnrollmentV1<'_>,
        approved: &[u8],
        approval_digest: &ResultCalendarDigestV1,
    ) -> StoreResult<Application> {
        let _lock = self.lock_appends();
        apply::apply(self, prepared, approved, approval_digest)
    }
}
