use super::*;
use crate::roster_source_issuance::{RosterEnrollmentReviewV1, RosterReviewActorV1};
use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;

pub(super) fn enroll_roster(store: &Store) -> TestResult {
    check!(eq; store.store_format()?.lifecycle_revision, Some(7));
    let inputs = store.inspect_roster_source_enrollment_v1()?;
    let prepared = store.prepare_roster_source_enrollment_v1(
        inputs,
        RosterEnrollmentReviewV1 {
            operation: RosterAuthorityTextV1::parse("controlled-school-predecessor-eight")?,
            actor: RosterReviewActorV1::LocalOperator,
            reviewed_at: chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")?.to_utc(),
            rationale: RosterAuthorityTextV1::parse("Explicit controlled predecessor enrollment")?,
        },
    )?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    let application = store.apply_roster_source_enrollment_v1(prepared, &approval, &digest)?;
    check!(application.written());
    check!(eq; store.store_format()?.lifecycle_revision, Some(8));
    Ok(())
}

pub(super) fn advance(store: &Store) -> TestResult {
    store.append(
        Table::Athletes,
        &subject("school-enrollment-stale-input", "991901")?,
    )?;
    Ok(())
}
