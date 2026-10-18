use super::*;
use census_crawl::participation_evidence::physical_result::project_retained_html_result_identity;
use census_crawl::participation_evidence::recorded::{
    AttachedParticipationV1, CheckedDocumentaryParticipationFactsV1,
    DiscoveredIdentityLinkedHtmlResultDocumentV1,
};
use census_domain::affiliation_candidates::{KnownBounds, ObjectRef};
use census_domain::evidence_subject_attachment::AttachmentRecordReferenceV1;
use census_domain::model::{AthleteId, ProfileDate, ProfileDateRangeV1};
use census_domain::participation_candidates::{
    assess_participation_with_in_season, CandidateOutcome, CohortFact, InSeasonEvidence,
    ParticipationObservation, ParticipationReasons, PerformanceDateFact, ProofReference,
    ResultOwnership,
};

mod evidence;
mod references;

pub(super) fn assess(
    sources: &AttachedParticipationV1<'_, '_>,
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    index: usize,
    query: CandidateQuery<'_>,
    grant: &mut Grant,
) -> ReportResult<(CandidateOutcome, ParticipationReasons, Vec<String>)> {
    sources
        .with_row_facts(linked, index, grant, |facts, grant| {
            Ok(evaluate(linked, index, query, facts, grant))
        })
        .map_err(source)?
}

fn evaluate(
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    index: usize,
    query: CandidateQuery<'_>,
    facts: CheckedDocumentaryParticipationFactsV1<'_>,
    grant: &mut Grant,
) -> ReportResult<(CandidateOutcome, ParticipationReasons, Vec<String>)> {
    let row = linked
        .row(index)
        .ok_or_else(|| invalid("documentary fact row missing"))?;
    let attachment = row.attachment();
    attachment.require_original_grant(grant)?;
    let athlete: AthleteId = attachment.target().canonical_candidate.cast();
    let identity =
        project_retained_html_result_identity(linked.checked_document(), index, &athlete)
            .map_err(source)?;
    let date = ProfileDate::parse(identity.date()).map_err(source)?;
    let record = record(attachment, grant)?;
    let proof = result_proof(
        &row,
        &athlete,
        query,
        linked.checked_document().capture().acquired_at(),
    )?;
    let intervals = evidence::intervals(&facts, grant)?;
    let statuses = evidence::statuses(&facts, &intervals, &athlete, query)?;
    let windows = evidence::windows(&facts, &intervals, &athlete, query)?;
    let full_windows = evidence::full_windows(&facts, &athlete, query, grant)?;
    let contexts = evidence::contexts(&facts, &athlete, query)?;
    let observation = ParticipationObservation {
        binding: query.binding,
        performance: identity.performance_id(),
        athlete: &athlete,
        ownership: ResultOwnership::DocumentaryBound {
            performance: identity.performance_id(),
            source_result: ObjectRef::parse(row.checked_row().reference()).map_err(source)?,
            subject: attachment.subject(),
            attachment_record: &record,
            attachment_inventory: attachment.inventory(),
            proof,
        },
        date: PerformanceDateFact::Exact(&date),
        cohort: evidence::cohort(&facts, &athlete, query)?,
        contexts: &contexts,
        high_school_status: &statuses,
        eligibility_windows: &full_windows,
    };
    step(grant, 1)?;
    let assessed = assess_participation_with_in_season(
        query,
        &observation,
        InSeasonEvidence::Uncontested(&windows),
    )
    .map_err(source)?;
    Ok((
        assessed.outcome,
        assessed.reasons,
        references::collect(&facts, grant)?,
    ))
}

fn record(
    attachment: &census_crawl::participation_evidence::recorded::DiscoveredResultAttachmentV1<
        '_,
        '_,
    >,
    grant: &mut Grant,
) -> ReportResult<AttachmentRecordReferenceV1> {
    let count = attachment
        .record()
        .command
        .operation
        .as_str()
        .len()
        .checked_add(attachment.record().record_digest.as_str().len())
        .ok_or(census_store::StoreError::CounterOverflow)?;
    grant.charge_bytes(count).map_err(domain)?;
    Ok(AttachmentRecordReferenceV1 {
        operation: attachment.record().command.operation.clone(),
        digest: attachment.record().record_digest.clone(),
    })
}

fn result_proof<'a>(
    row: &'a census_crawl::participation_evidence::recorded::DiscoveredIdentityLinkedHtmlResultRowV1<'_, '_>,
    athlete: &'a AthleteId,
    query: CandidateQuery<'a>,
    acquired_at: DateTime<Utc>,
) -> ReportResult<ProofReference<'a>> {
    let subject = row.attachment().subject();
    Ok(ProofReference {
        fact: ObjectRef::parse(row.checked_row().original_row_digest()).map_err(source)?,
        capture: ObjectRef::parse(subject.capture.as_str()).map_err(source)?,
        interpretation: ObjectRef::parse(subject.interpretation.as_str()).map_err(source)?,
        locator: ObjectRef::parse(subject.locator.as_str()).map_err(source)?,
        athlete,
        known: KnownBounds::new(
            query.binding.frozen.lineage,
            query.binding.frozen.revision,
            query.binding.frozen.revision,
        )
        .map_err(source)?,
        acquired_at,
    })
}
