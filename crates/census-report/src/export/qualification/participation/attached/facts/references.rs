use super::*;

pub(super) fn collect(
    facts: &CheckedDocumentaryParticipationFactsV1<'_>,
    calendar: Option<&CheckedResultCalendarCorrespondenceV1<'_>>,
    grant: &mut Grant,
) -> ReportResult<Vec<String>> {
    let mut output = source_references(facts, grant)?;
    if let Some(calendar) = calendar {
        append_calendar(&mut output, calendar, grant)?;
    }
    Ok(output)
}

fn source_references(
    facts: &CheckedDocumentaryParticipationFactsV1<'_>,
    grant: &mut Grant,
) -> ReportResult<Vec<String>> {
    let count = facts
        .sources()
        .len()
        .checked_mul(9)
        .and_then(|count| {
            facts
                .windows()
                .len()
                .checked_mul(5)
                .and_then(|windows| count.checked_add(windows))
        })
        .ok_or(census_store::StoreError::CounterOverflow)?;
    grant
        .charge_bytes(
            count
                .checked_mul(std::mem::size_of::<String>())
                .ok_or(census_store::StoreError::CounterOverflow)?,
        )
        .map_err(domain)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(count)
        .map_err(|_| crate::grouping::GroupingError::Allocation {
            stage: "documentary semantic references",
        })?;
    for item in facts.sources() {
        let status = item.source().status_proof();
        let season = item.source().season_proof();
        let values = [
            item.source().reference().as_str(),
            item.attachment_record_digest(),
            status.capture(),
            status.interpretation(),
            status.locator(),
            season.capture(),
            season.interpretation(),
            season.locator(),
            item.source().season_scope(),
        ];
        append(&mut output, &values, grant)?;
    }
    for item in facts.windows() {
        let proof = item.source().boundary_proof();
        let values = [
            item.source().reference().as_str(),
            item.attachment_record_digest(),
            proof.capture(),
            proof.interpretation(),
            proof.locator(),
        ];
        append(&mut output, &values, grant)?;
    }
    Ok(output)
}

fn append_calendar(
    output: &mut Vec<String>,
    calendar: &census_crawl::participation_evidence::result_ownership::calendar_correspondence::read::CheckedResultCalendarCorrespondenceV1<'_>,
    grant: &mut Grant,
) -> ReportResult<()> {
    let record = calendar.record();
    let source = &record.command.source;
    let values = [
        record.record_digest.as_str(),
        source.parser_contract.digest.as_str(),
        source.calendar.capture.as_str(),
        source.calendar.interpretation.as_str(),
        source.calendar.locator.as_str(),
    ];
    grant
        .charge_bytes(
            values
                .len()
                .checked_mul(std::mem::size_of::<String>())
                .ok_or(census_store::StoreError::CounterOverflow)?,
        )
        .map_err(domain)?;
    output.try_reserve_exact(values.len()).map_err(|_| {
        crate::grouping::GroupingError::Allocation {
            stage: "reviewed calendar references",
        }
    })?;
    append(output, &values, grant)
}

fn append(output: &mut Vec<String>, values: &[&str], grant: &mut Grant) -> ReportResult<()> {
    step(grant, values.len())?;
    for value in values {
        grant.charge_bytes(value.len()).map_err(domain)?;
        output.push((*value).to_owned());
    }
    Ok(())
}
