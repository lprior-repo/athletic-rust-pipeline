use crate::grouping::Admission;
use crate::report::ReportResult;

pub(super) fn base_refs<'a>(
    row: &'a census_crawl::participation_evidence::recorded::DiscoveredIdentityLinkedHtmlResultRowV1<
        '_,
        '_,
    >,
) -> [&'a str; 8] {
    let attachment = row.attachment();
    let subject = attachment.subject();
    [
        subject.capture.as_str(),
        subject.body.as_str(),
        subject.interpretation.as_str(),
        subject.locator.as_str(),
        subject.context.as_str(),
        row.checked_row().original_row_digest(),
        attachment.record().record_digest.as_str(),
        attachment.inventory().digest.as_str(),
    ]
}

pub(super) fn append_semantic(
    references: &mut Vec<String>,
    semantic: Vec<String>,
    budget: &mut Admission,
) -> ReportResult<()> {
    for reference in semantic {
        let bytes = std::mem::size_of::<String>()
            .checked_add(reference.len())
            .and_then(|bytes| bytes.checked_mul(4))
            .ok_or(census_store::StoreError::CounterOverflow)?;
        budget.charge(bytes, 1)?;
        references.try_reserve_exact(1).map_err(|_| {
            crate::grouping::GroupingError::Allocation {
                stage: "selected documentary semantic references",
            }
        })?;
        references.push(reference);
    }
    Ok(())
}

pub(super) fn build_owned(
    base: &[&str],
    semantic: Vec<String>,
    budget: &mut Admission,
) -> ReportResult<Vec<String>> {
    let mut owned = Vec::new();
    owned.try_reserve_exact(base.len()).map_err(|_| {
        crate::grouping::GroupingError::Allocation {
            stage: "selected documentary references",
        }
    })?;
    for value in base {
        budget.charge(value.len(), 0)?;
        owned.push((*value).to_owned());
    }
    append_semantic(&mut owned, semantic, budget)?;
    Ok(owned)
}

pub(super) fn admit_documentary(
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
            std::mem::size_of::<crate::export::qualification::DocumentaryParticipationRow>()
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

pub(super) fn admit_pinned(
    budget: &mut Admission,
    performance: &census_domain::model::CanonicalPerformance,
    references: &[String],
) -> ReportResult<()> {
    let bytes = references
        .iter()
        .map(|value| value.len())
        .chain([
            performance.id.as_str().len(),
            performance.event.as_str().len(),
            performance.athlete.as_str().len(),
        ])
        .try_fold(
            std::mem::size_of::<super::super::PinnedParticipationRow>()
                .checked_add(
                    std::mem::size_of::<String>()
                        .checked_mul(references.len())
                        .ok_or(census_store::StoreError::CounterOverflow)?,
                )
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

pub(super) fn push_dedup(
    references: &mut Vec<String>,
    value: &str,
    budget: &mut Admission,
) -> ReportResult<()> {
    for prior in references.iter() {
        budget.charge(0, 1)?;
        if prior.as_str() == value {
            return Ok(());
        }
    }
    budget.charge(
        std::mem::size_of::<String>()
            .checked_add(value.len())
            .ok_or(census_store::StoreError::CounterOverflow)?,
        1,
    )?;
    references
        .try_reserve_exact(1)
        .map_err(|_| crate::grouping::GroupingError::Allocation {
            stage: "selected documentary semantic references",
        })?;
    references.push(value.to_owned());
    Ok(())
}
