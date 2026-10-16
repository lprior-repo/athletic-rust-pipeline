use super::*;
use crate::participation_evidence::physical_result::project_retained_html_result_identity;
use census_domain::affiliation_candidates::{DeclaredFrozenView, KnownBounds, ObjectRef};
use census_domain::model::{AthleteId, ProfileDate, QueryProfileV1};
use census_domain::participation_candidates::{
    assess_participation_with_in_season, CandidateBinding, CandidateOutcome, CandidateQuery,
    CohortFact, InSeasonEvidence, ParticipationObservation, ParticipationReason,
    PerformanceDateFact, ProofReference, ResultOwnership,
};
use std::num::NonZeroU64;

pub(super) fn verify(
    linked: &IdentityLinkedHtmlResultDocumentV1<'_, '_>,
    installed: &super::super::super::accepted_provider::installed::Installed,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult {
    let profile = profile::controlled(installed.athlete.grad_year)?;
    for index in 0..linked.row_count() {
        assess_row(linked, index, &profile, &installed.athlete.id, grant)?;
    }
    Ok(())
}

fn assess_row(
    linked: &IdentityLinkedHtmlResultDocumentV1<'_, '_>,
    index: usize,
    profile: &QueryProfileV1,
    installed_athlete: &AthleteId,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult {
    let row = linked.row(index).ok_or("complete linked row")?;
    let attachment = row.attachment();
    attachment.require_original_grant(grant)?;
    let athlete: AthleteId = attachment.target().canonical_candidate.cast();
    check!(eq; &athlete, installed_athlete);
    let identity =
        project_retained_html_result_identity(linked.checked_document(), index, &athlete)?;
    check!(eq; identity.athlete(), &athlete);
    check!(eq; identity.date(), linked.checked_document().published_start());
    let profile_id = profile.id()?;
    let revision = NonZeroU64::new(attachment.snapshot().sequence()).ok_or("held sequence")?;
    let lineage = ObjectRef::parse(attachment.store_identity().as_str())?;
    let binding = CandidateBinding {
        profile: &profile_id,
        participation_rule: &profile.input().policies.participation,
        frozen: DeclaredFrozenView {
            lineage,
            revision,
            captured_at: DateTime::parse_from_rfc3339("2026-10-16T12:00:00Z")?.with_timezone(&Utc),
            closure_object: ObjectRef::parse(attachment.record().record_digest.as_str())?,
            family_inventory: ObjectRef::parse(attachment.inventory().digest.as_str())?,
        },
    };
    let subject = attachment.subject();
    let proof = ProofReference {
        fact: ObjectRef::parse(row.checked_row().original_row_digest())?,
        capture: ObjectRef::parse(subject.capture.as_str())?,
        interpretation: ObjectRef::parse(subject.interpretation.as_str())?,
        locator: ObjectRef::parse(subject.locator.as_str())?,
        athlete: &athlete,
        known: KnownBounds::new(lineage, revision, revision)?,
        acquired_at: linked.checked_document().capture().acquired_at(),
    };
    let record = AttachmentRecordReferenceV1 {
        operation: attachment.record().command.operation.clone(),
        digest: attachment.record().record_digest.clone(),
    };
    let date = ProfileDate::parse(identity.date())?;
    let observation = ParticipationObservation {
        binding,
        performance: identity.performance_id(),
        athlete: &athlete,
        ownership: ResultOwnership::DocumentaryBound {
            performance: identity.performance_id(),
            source_result: ObjectRef::parse(row.checked_row().reference())?,
            subject,
            attachment_record: &record,
            attachment_inventory: attachment.inventory(),
            proof,
        },
        date: PerformanceDateFact::Exact(&date),
        cohort: CohortFact::Missing,
        contexts: &[],
        high_school_status: &[],
        eligibility_windows: &[],
    };
    let assessment = assess_participation_with_in_season(
        CandidateQuery { profile, binding },
        &observation,
        InSeasonEvidence::Unavailable,
    )?;
    check!(eq; assessment.outcome, CandidateOutcome::Unresolved);
    check!(eq; assessment.reasons.iter().collect::<Vec<_>>(), vec![
        ParticipationReason::CohortUnresolved,
        ParticipationReason::ParticipationContextUnresolved,
        ParticipationReason::HighSchoolStatusMissing,
        ParticipationReason::SummerBoundaryUnresolved,
    ]);
    Ok(())
}
