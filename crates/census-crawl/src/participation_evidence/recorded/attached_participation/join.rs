use super::*;
use crate::participation_evidence::recorded::DiscoveredResultAttachmentV1;
use census_domain::model::{Gender, GradYear, Sport};

pub(super) fn row<'a, 's>(
    sources: &'a AttachedParticipationV1<'_, 's>,
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, 's>,
    index: usize,
    grant: &mut Grant,
) -> CrawlResult<Vec<CheckedDocumentarySourceFactsV1<'a>>> {
    let row = linked
        .row(index)
        .ok_or_else(|| invalid("documentary participation row missing"))?;
    let result = row.attachment();
    result.require_original_grant(grant)?;
    unsupported(sources, result, grant)?;
    let mut facts: Vec<CheckedDocumentarySourceFactsV1<'a>> = Vec::new();
    for candidate in &sources.sources {
        steps(grant, 1)?;
        candidate.binding.require_original_grant(grant)?;
        same_view(&candidate.binding, result)?;
        if candidate.binding.target().canonical_candidate != result.target().canonical_candidate {
            continue;
        }
        if candidate.binding.target() != result.target() {
            return Err(invalid("documentary status and result target disagree").into());
        }
        let source = candidate.recorded.checked();
        let Some(cohort) = applicability(linked, &row, source, grant)? else {
            continue;
        };
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
        let venue = linked.documentary_venue(index, grant)?;
        let season = season(
            &linked.checked_document().parsed_page().meet.name,
            venue,
            source,
        );
        reserve(&mut facts, grant)?;
        facts.push(CheckedDocumentarySourceFactsV1 {
            source,
            record_digest: candidate.binding.record_digest(),
            season,
            cohort,
        });
    }
    Ok(facts)
}

fn contains_source(
    facts: &[CheckedDocumentarySourceFactsV1<'_>],
    source: &VerifiedIndependentParticipationV1,
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

pub(super) fn same_view(
    status: &StatusBinding<'_, '_>,
    result: &DiscoveredResultAttachmentV1<'_, '_>,
) -> CrawlResult<()> {
    if status.store_identity() != result.store_identity()
        || status.snapshot_sequence() != result.snapshot_sequence()
        || status.accepted_public_head() != result.accepted_public_head()
        || status.inventory() != result.inventory()
    {
        return Err(invalid("documentary status and result held authority differ").into());
    }
    Ok(())
}

fn applicability(
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    row: &crate::participation_evidence::recorded::DiscoveredIdentityLinkedHtmlResultRowV1<'_, '_>,
    source: &VerifiedIndependentParticipationV1,
    grant: &mut Grant,
) -> CrawlResult<Option<DocumentaryCohortAgreementV1>> {
    steps(grant, 10)?;
    let document = linked.checked_document();
    let page = document.parsed_page();
    let parsed = row.checked_row().parsed_row();
    if source.store_identity() != row.attachment().store_identity()
        || source.snapshot_sequence() != row.attachment().snapshot_sequence()
    {
        return Err(invalid("documentary status source or attachment authority differs").into());
    }
    if document.published_start() != source.day().as_str()
        || document
            .published_end()
            .is_some_and(|end| end != source.day().as_str())
    {
        return Ok(None);
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
    Ok(Some(cohort_agreement(
        parsed.grade,
        page.school_year,
        source.cohort(),
    )))
}

pub(super) fn season(
    competition: &str,
    venue: &str,
    source: &VerifiedIndependentParticipationV1,
) -> DocumentarySeasonApplicabilityV1 {
    match (competition == source.competition(), venue == source.venue()) {
        (true, true) => DocumentarySeasonApplicabilityV1::Exact,
        (false, true) => DocumentarySeasonApplicabilityV1::CompetitionMismatch,
        (true, false) => DocumentarySeasonApplicabilityV1::VenueMismatch,
        (false, false) => DocumentarySeasonApplicabilityV1::CompetitionAndVenueMismatch,
    }
}

fn unsupported(
    sources: &AttachedParticipationV1<'_, '_>,
    result: &DiscoveredResultAttachmentV1<'_, '_>,
    grant: &mut Grant,
) -> CrawlResult<()> {
    for status in &sources.unsupported {
        steps(grant, 1)?;
        status.require_original_grant(grant)?;
        same_view(status, result)?;
        if status.target().canonical_candidate == result.target().canonical_candidate {
            return Err(
                invalid("documentary target has unsupported retained status authority").into(),
            );
        }
    }
    Ok(())
}

pub(super) fn cohort_agreement(
    grade: Option<census_domain::model::Grade>,
    year: census_domain::model::SchoolYear,
    expected: GradYear,
) -> DocumentaryCohortAgreementV1 {
    if grade
        .and_then(|grade| GradYear::of(grade, year))
        .is_some_and(|observed| observed != expected)
    {
        DocumentaryCohortAgreementV1::Conflicting
    } else {
        DocumentaryCohortAgreementV1::Agreed
    }
}
