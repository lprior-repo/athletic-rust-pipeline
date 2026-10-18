use super::super::{codec, format, RosterSourceIssuanceStoreError as Error};
use super::super::{RosterStartupValidationBudgetV1, LIFECYCLE};
use super::{PreparedRosterEnrollmentV1, RosterEnrollmentCommandV1, RosterReviewActorV1};
use crate::{Store, StoreResult, StoreRevision};
use census_domain::model::StoreIdentity;
use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;
use chrono::{DateTime, Utc};

pub struct RosterEnrollmentInputsV1<'s> {
    revision: StoreRevision<'s>,
    store: StoreIdentity,
    token_digest: RosterAuthorityTextV1,
}

pub struct RosterEnrollmentReviewV1 {
    pub operation: RosterAuthorityTextV1,
    pub actor: RosterReviewActorV1,
    pub reviewed_at: DateTime<Utc>,
    pub rationale: RosterAuthorityTextV1,
}

impl RosterEnrollmentInputsV1<'_> {
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.store
    }
    pub fn evidence_generation(&self) -> u64 {
        self.revision.evidence_generation()
    }
    pub fn derived_generation(&self) -> u64 {
        self.revision.derived_generation()
    }
    pub fn input_token_digest(&self) -> &str {
        self.token_digest.as_str()
    }
}

impl Store {
    pub fn inspect_roster_source_enrollment_v1(&self) -> StoreResult<RosterEnrollmentInputsV1<'_>> {
        let _guard = self.lock_appends();
        let current = self.store_format()?;
        let lifecycle = format::validated_lifecycle(
            &self.db.snapshot(),
            &self.meta,
            &self.journal,
            &self.receipts,
            &current,
            &mut RosterStartupValidationBudgetV1::new(),
        )?;
        if current.lifecycle_revision != Some(7) && lifecycle != Some(LIFECYCLE) {
            return Err(Error::EnrollmentRequired.into());
        }
        let view = self.snapshot_locked();
        let bytes = view.attachment_input_revision_bytes()?;
        let token_digest = RosterAuthorityTextV1::parse(&codec::digest(&bytes))
            .map_err(|_| Error::Corrupt("input token digest"))?;
        Ok(RosterEnrollmentInputsV1 {
            revision: view.revision(),
            store: view.store_identity()?,
            token_digest,
        })
    }

    pub fn prepare_roster_source_enrollment_v1<'s>(
        &'s self,
        inputs: RosterEnrollmentInputsV1<'s>,
        review: RosterEnrollmentReviewV1,
    ) -> StoreResult<PreparedRosterEnrollmentV1<'s>> {
        let command = RosterEnrollmentCommandV1 {
            version: 1,
            store: inputs.store,
            operation: review.operation,
            actor: review.actor,
            reviewed_at: review.reviewed_at,
            rationale: review.rationale,
            evidence_generation: inputs.revision.evidence_generation(),
            derived_generation: inputs.revision.derived_generation(),
            input_token_digest: inputs.token_digest,
        };
        self.prepare_roster_source_enrollment_command_v1(inputs.revision, command)
    }
}
