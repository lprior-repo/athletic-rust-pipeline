use super::*;

pub(super) fn collect(
    facts: &CheckedDocumentaryParticipationFactsV1<'_>,
    grant: &mut Grant,
) -> ReportResult<Vec<String>> {
    let count = facts
        .sources()
        .len()
        .checked_mul(9)
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
        step(grant, values.len())?;
        for value in values {
            grant.charge_bytes(value.len()).map_err(domain)?;
            output.push(value.to_owned());
        }
    }
    Ok(output)
}
