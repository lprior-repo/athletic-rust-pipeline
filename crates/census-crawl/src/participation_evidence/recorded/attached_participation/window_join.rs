use super::*;
use crate::participation_evidence::VerifiedIndependentParticipationWindowV1;
use census_domain::model::{Gender, Sport};

pub(super) fn row<'a, 's>(
    sources: &'a AttachedParticipationV1<'_, 's>,
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, 's>,
    index: usize,
    grant: &mut Grant,
) -> CrawlResult<Vec<CheckedDocumentaryWindowFactsV1<'a>>> {
    if sources.windows.is_empty() {
        return Ok(Vec::new());
    }
    let row = linked
        .row(index)
        .ok_or_else(|| invalid("documentary participation row missing"))?;
    let result = row.attachment();
    result.require_original_grant(grant)?;
    let mut facts: Vec<CheckedDocumentaryWindowFactsV1<'a>> = Vec::new();
    for candidate in &sources.windows {
        steps(grant, 1)?;
        candidate.binding.require_original_grant(grant)?;
        super::join::same_view(&candidate.binding, result)?;
        if candidate.binding.target().canonical_candidate != result.target().canonical_candidate {
            continue;
        }
        if candidate.binding.target() != result.target() {
            return Err(invalid("documentary status and result target disagree").into());
        }
        let source = candidate.recorded.checked();
        let cohort = applicability(linked, &row, source, grant)?;
        if contains_source(&facts, source, grant)? {
            continue;
        }
        crate::participation_evidence::bound(
            facts
                .len()
                .checked_add(1)
                .ok_or(census_store::StoreError::CounterOverflow)?,
            16,
            "attached participation source count",
        )?;
        reserve(&mut facts, grant)?;
        facts.push(CheckedDocumentaryWindowFactsV1 {
            source,
            record_digest: candidate.binding.record_digest(),
            cohort,
        });
    }
    Ok(facts)
}

fn contains_source(
    facts: &[CheckedDocumentaryWindowFactsV1<'_>],
    source: &VerifiedIndependentParticipationWindowV1,
    grant: &mut Grant,
) -> CrawlResult<bool> {
    for prior in facts {
        steps(grant, 1)?;
        if prior.source.reference() == source.reference() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn applicability(
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    row: &crate::participation_evidence::recorded::DiscoveredIdentityLinkedHtmlResultRowV1<'_, '_>,
    source: &VerifiedIndependentParticipationWindowV1,
    grant: &mut Grant,
) -> CrawlResult<DocumentaryCohortAgreementV1> {
    steps(grant, 10)?;
    let page = linked.checked_document().parsed_page();
    let parsed = row.checked_row().parsed_row();
    if source.store_identity() != row.attachment().store_identity()
        || source.snapshot_sequence() != row.attachment().snapshot_sequence()
    {
        return Err(invalid("documentary status source or attachment authority differs").into());
    }
    if source.subject().name() != parsed.name || source.subject().school() != parsed.school {
        return Err(invalid("documentary status subject or participation subject differs").into());
    }
    if row.checked_row().event().gender != Gender::Boys
        || page.sport != Some(Sport::OutdoorTrack)
        || page.region.as_deref() != Some("MI")
        || page.school_year != source.school_year()
    {
        return Err(invalid("documentary status participation context differs").into());
    }
    Ok(super::join::cohort_agreement(
        parsed.grade,
        page.school_year,
        source.cohort(),
    ))
}
