use super::super::RosterReviewActorV1;
use super::{ObjectRef, PreparedRosterReviewCore};
use crate::StoreRevision;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{ProfileDateRangeV1, SourceSubject};
use census_domain::roster_affiliation_authority::{RosterAuthorityDigestV1, RosterAuthorityTextV1};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

mod apply;
pub(super) mod io;
mod prepare;
mod source;
pub(super) mod validate;

pub(super) const BASIS_SCHEMA: &str = "roster-source-member-withdrawal-basis-v1";
pub(super) const REVIEW_SCHEMA: &str = "roster-source-member-withdrawal-review-v1";
pub(super) const INPUT_SCHEMA: &str = "roster-source-member-withdrawal-input-v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RosterMemberWithdrawalDecisionV1 {
    pub(super) source_operation: RosterAuthorityTextV1,
    pub(super) source_record: ObjectRef,
    pub(super) witness: ObjectRef,
    pub(super) member_claim: RosterAuthorityDigestV1,
    pub(super) person: SourceSubject,
    pub(super) scope: ProfileDateRangeV1,
}

impl RosterMemberWithdrawalDecisionV1 {
    pub fn source_operation(&self) -> &str {
        self.source_operation.as_str()
    }

    pub fn member_claim(&self) -> &str {
        self.member_claim.as_str()
    }

    pub fn person(&self) -> &SourceSubject {
        &self.person
    }

    pub fn scope(&self) -> &ProfileDateRangeV1 {
        &self.scope
    }
}

pub struct RosterMemberWithdrawalReviewV1 {
    pub operation: RosterAuthorityTextV1,
    pub actor: RosterReviewActorV1,
    pub reviewed_at: DateTime<Utc>,
    pub rationale: RosterAuthorityTextV1,
    pub scope: ProfileDateRangeV1,
}

pub struct PreparedRosterMemberWithdrawalV1<'s> {
    pub(super) revision: StoreRevision<'s>,
    pub(super) grant: AttachmentReplayGrantV1,
    pub(super) core: PreparedRosterReviewCore,
}

impl PreparedRosterMemberWithdrawalV1<'_> {
    pub fn approval_bytes(&self) -> &[u8] {
        &self.core.approval
    }

    pub fn approval_digest(&self) -> &str {
        &self.core.approval_digest
    }
}

#[derive(Serialize)]
pub struct ReviewedRosterMemberWithdrawalV1 {
    pub(super) decision: RosterMemberWithdrawalDecisionV1,
    pub(super) operation: RosterAuthorityTextV1,
    pub(super) review: ObjectRef,
    pub(super) committed_at: DateTime<Utc>,
    pub(super) ordinal: u64,
}

impl ReviewedRosterMemberWithdrawalV1 {
    pub fn decision(&self) -> &RosterMemberWithdrawalDecisionV1 {
        &self.decision
    }

    pub fn operation(&self) -> &str {
        self.operation.as_str()
    }

    pub fn review_digest(&self) -> &str {
        &self.review.digest
    }

    pub fn committed_at(&self) -> &DateTime<Utc> {
        &self.committed_at
    }

    pub fn ordinal(&self) -> u64 {
        self.ordinal
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WithdrawalInput {
    pub(super) schema: String,
    pub(super) store: census_domain::model::StoreIdentity,
    pub(super) sequence: u64,
    pub(super) evidence_generation: u64,
    pub(super) derived_generation: u64,
    pub(super) source_operation: RosterAuthorityTextV1,
    pub(super) source_record: ObjectRef,
    pub(super) witness: ObjectRef,
    pub(super) scope: ProfileDateRangeV1,
    pub(super) prior_head: ObjectRef,
}

fn domain(
    error: census_domain::roster_affiliation_authority::RosterAuthorityError,
) -> crate::StoreError {
    use census_domain::roster_affiliation_authority::RosterAuthorityError;
    match error {
        RosterAuthorityError::Capacity {
            resource,
            needed,
            available,
        } => crate::StoreError::BudgetExceeded {
            what: resource,
            needed,
            available,
        },
        RosterAuthorityError::Overflow => crate::StoreError::CounterOverflow,
        RosterAuthorityError::Decode(source) => crate::StoreError::Decode {
            key: "roster member witness".into(),
            source,
        },
        RosterAuthorityError::Encode(source) => source.into(),
        error => crate::StoreError::Refused {
            detail: error.to_string(),
        },
    }
}

fn require_owner(store: &crate::Store) -> crate::StoreResult<()> {
    let format = store.store_format()?;
    let lifecycle = super::super::format::validated_lifecycle(
        &store.db.snapshot(),
        &store.meta,
        &store.journal,
        &store.receipts,
        &format,
        &mut super::super::RosterStartupValidationBudgetV1::new(),
    )?;
    if lifecycle != Some(super::super::LIFECYCLE) {
        return Err(super::super::RosterSourceIssuanceStoreError::EnrollmentRequired.into());
    }
    Ok(())
}

fn check_scope(
    original: &census_domain::roster_affiliation_authority::LocallyCheckedRosterIssuanceV1,
    scope: &ProfileDateRangeV1,
) -> crate::StoreResult<()> {
    let day = original.capture().observed_day();
    if scope.from() != day || scope.through() != day {
        return Err(crate::StoreError::Refused {
            detail: "member withdrawal exceeds original exact capture-day support".to_string(),
        });
    }
    Ok(())
}
