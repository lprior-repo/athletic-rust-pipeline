use super::*;
use census_crawl::participation_evidence::physical_result::project_retained_html_result_identity;
use census_crawl::participation_evidence::recorded::DiscoveredIdentityLinkedHtmlResultDocumentV1;
use census_domain::affiliation_candidates::{KnownBounds, ObjectRef};
use census_domain::evidence_subject_attachment::AttachmentRecordReferenceV1;
use census_domain::model::{AthleteId, ProfileDate};
use census_domain::participation_candidates::{
    assess_participation_with_in_season, CohortFact, InSeasonEvidence, ParticipationObservation,
    PerformanceDateFact, ProofReference, ResultOwnership,
};

pub(super) fn row(
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    index: usize,
    query: CandidateQuery<'_>,
    sources: AssessmentSources<'_, '_, '_>,
    grant: &mut Grant,
    budget: &mut Admission,
) -> ReportResult<DocumentaryParticipationRow> {
    let row = linked
        .row(index)
        .ok_or_else(|| invalid("completed documentary row is missing"))?;
    let attachment = row.attachment();
    attachment.require_original_grant(grant)?;
    if attachment.store_identity().as_str() != query.binding.frozen.lineage.as_str()
        || attachment.snapshot_sequence() != query.binding.frozen.revision.get()
    {
        return Err(invalid("documentary source differs from selected held identity").into());
    }
    step(grant, 1)?;
    budget.charge(attachment.target().canonical_candidate.as_str().len(), 1)?;
    let athlete: AthleteId = attachment.target().canonical_candidate.cast();
    let identity =
        project_retained_html_result_identity(linked.checked_document(), index, &athlete)
            .map_err(source)?;
    let (outcome, reasons, semantic_refs) = match sources {
        AssessmentSources::HistoricalWithheld => {
            let assessed = evaluate(&row, &identity, query, grant, linked.checked_document().capture().acquired_at())?;
            (assessed.0, assessed.1, Vec::new())
        }
        AssessmentSources::Current(sources) => facts::assess(sources, linked, index, query, grant)?,
    };
    let subject = attachment.subject();
    let references = [
        subject.capture.as_str(),
        subject.body.as_str(),
        subject.interpretation.as_str(),
        subject.locator.as_str(),
        subject.context.as_str(),
        row.checked_row().original_row_digest(),
        attachment.record().record_digest.as_str(),
        attachment.inventory().digest.as_str(),
    ];
    admit_row(budget, &identity, &references)?;
    let mut owned_refs = Vec::new();
    owned_refs
        .try_reserve_exact(references.len())
        .map_err(|_| crate::grouping::GroupingError::Allocation {
            stage: "selected documentary references",
        })?;
    owned_refs.extend(references.into_iter().map(str::to_owned));
    append_semantic_references(&mut owned_refs, semantic_refs, budget)?;
    Ok(DocumentaryParticipationRow::school_unresolved(
        identity.performance_id().clone(),
        identity.event_id().clone(),
        athlete,
        outcome,
        reasons,
        owned_refs,
    ))
}

fn append_semantic_references(
    references: &mut Vec<String>,
    semantic: Vec<String>,
    budget: &mut Admission,
) -> ReportResult<()> {
    for reference in semantic {
        budget.charge(reference.len(), 1)?;
        references.try_reserve_exact(1).map_err(|_| crate::grouping::GroupingError::Allocation {
            stage: "selected documentary semantic references",
        })?;
        references.push(reference);
    }
    Ok(())
}

fn admit_row(
    budget: &mut Admission,
    identity: &census_crawl::participation_evidence::physical_result::ProposedHtmlResultIdentityV1,
    references: &[&str],
) -> ReportResult<()> {
    let bytes = references
        .iter()
        .map(|value| value.len())
        .chain([
            identity.performance_id().as_str().len(),
            identity.event_id().as_str().len(),
            identity.athlete().as_str().len(),
        ])
        .try_fold(
            std::mem::size_of::<DocumentaryParticipationRow>()
                .checked_add(std::mem::size_of_val(references))
                .ok_or(census_store::StoreError::CounterOverflow)?,
            |held, bytes| {
                held.checked_add(bytes)
                    .ok_or(census_store::StoreError::CounterOverflow)
            },
        )?
        .checked_mul(4)
        .ok_or(census_store::StoreError::CounterOverflow)?;
    budget.charge(bytes, 1)?;
    Ok(())
}

fn evaluate(
    row: &census_crawl::participation_evidence::recorded::DiscoveredIdentityLinkedHtmlResultRowV1<
        '_,
        '_,
    >,
    identity: &census_crawl::participation_evidence::physical_result::ProposedHtmlResultIdentityV1,
    query: CandidateQuery<'_>,
    grant: &mut Grant,
    acquired_at: DateTime<Utc>,
) -> ReportResult<(
    census_domain::participation_candidates::CandidateOutcome,
    census_domain::participation_candidates::ParticipationReasons,
)> {
    let attachment = row.attachment();
    let athlete = identity.athlete();
    let subject = attachment.subject();
    let proof = proof(
        subject,
        athlete,
        row.checked_row().original_row_digest(),
        query,
        acquired_at,
    )?;
    let bytes = attachment
        .record()
        .command
        .operation
        .as_str()
        .len()
        .checked_add(attachment.record().record_digest.as_str().len())
        .ok_or(census_store::StoreError::CounterOverflow)?;
    grant.charge_bytes(bytes).map_err(domain)?;
    let record = AttachmentRecordReferenceV1 {
        operation: attachment.record().command.operation.clone(),
        digest: attachment.record().record_digest.clone(),
    };
    let date = ProfileDate::parse(identity.date()).map_err(source)?;
    let observation = ParticipationObservation {
        binding: query.binding,
        performance: identity.performance_id(),
        athlete,
        ownership: ResultOwnership::DocumentaryBound {
            performance: identity.performance_id(),
            source_result: ObjectRef::parse(row.checked_row().reference()).map_err(source)?,
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
    step(grant, 1)?;
    let assessed =
        assess_participation_with_in_season(query, &observation, InSeasonEvidence::Unavailable)
            .map_err(source)?;
    Ok((assessed.outcome, assessed.reasons))
}

fn proof<'a>(
    subject: &'a census_domain::evidence_subject_attachment::RetainedDocumentSubjectReferenceV1,
    athlete: &'a AthleteId,
    fact: &'a str,
    query: CandidateQuery<'a>,
    acquired_at: DateTime<Utc>,
) -> ReportResult<ProofReference<'a>> {
    Ok(ProofReference {
        fact: ObjectRef::parse(fact).map_err(source)?,
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
