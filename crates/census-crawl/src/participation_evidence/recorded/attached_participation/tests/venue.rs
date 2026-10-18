use super::*;
use census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError as DomainError;
use census_store::{
    EvidenceSubjectAttachmentStoreError as StoreAttachmentError, Store, StoreError, Table,
};

pub(in crate::participation_evidence::recorded) fn verify_venue(store: &Store) -> TestResult {
    let tables = store.snapshot().tables_digest(&Table::ALL)?;
    let receipts = store.receipt_count()?;
    let cost = exercise(store, None)?;
    exercise(store, Some((cost, 0)))?;
    exercise(store, Some((cost, 1)))?;
    check!(eq; store.snapshot().tables_digest(&Table::ALL)?, tables);
    check!(eq; store.receipt_count()?, receipts);
    Ok(())
}

fn exercise(store: &Store, budget: Option<(usize, usize)>) -> TestResult<usize> {
    let snapshot = store.snapshot();
    let mut grant = Grant::new();
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let plan = crate::participation_evidence::recorded::discover_current_html_results(
        &current, &mut grant,
    )?;
    plan.with_group(0, &mut grant, |linked, grant| {
        Ok(match budget {
            None => measure(&linked, grant),
            Some((cost, short)) => boundary(&linked, grant, cost, short),
        })
    })?
}

fn measure(
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    grant: &mut Grant,
) -> TestResult<usize> {
    let before = grant.usage().encoded_bytes();
    check!(eq; linked.documentary_venue(0, grant)?, "Highland Milford High School");
    let cold = grant
        .usage()
        .encoded_bytes()
        .checked_sub(before)
        .ok_or("cold delta")?;
    let parser_cost = linked
        .checked_document()
        .original_document()
        .len()
        .checked_mul(4)
        .and_then(|n| n.checked_add(256))
        .ok_or("parser cost")?;
    check!(linked.row_count() > 1);
    for index in 0..linked.row_count() {
        let before = grant.usage().encoded_bytes();
        check!(eq; linked.documentary_venue(index, grant)?, "Highland Milford High School");
        let warm = grant
            .usage()
            .encoded_bytes()
            .checked_sub(before)
            .ok_or("warm delta")?;
        check!(eq; cold.checked_sub(warm), Some(parser_cost));
    }
    let mut foreign = Grant::new();
    let refusal = linked.documentary_venue(0, &mut foreign);
    check!(matches!(
        refusal,
        Err(crate::CrawlError::Store(
            StoreError::EvidenceSubjectAttachment(StoreAttachmentError::Domain(
                DomainError::Invalid("checked command grant differs")
            ))
        ))
    ));
    check!(matches!(
        foreign.ensure_available(),
        Err(DomainError::Poisoned)
    ));
    check!(linked.documentary_venue(0, &mut foreign).is_err());
    grant.ensure_available()?;
    check!(linked.documentary_venue(linked.row_count(), grant).is_err());
    check!(grant.ensure_available().is_err());
    check!(linked.documentary_venue(0, grant).is_err());
    Ok(cold)
}

fn boundary(
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    grant: &mut Grant,
    cost: usize,
    short: usize,
) -> TestResult<usize> {
    let limit = Grant::MAX_ENCODED_BYTES;
    let remaining = limit
        .checked_sub(grant.usage().encoded_bytes())
        .and_then(|n| n.checked_sub(cost))
        .and_then(|n| n.checked_add(short))
        .ok_or("venue boundary")?;
    grant.charge_bytes(remaining)?;
    let result = linked.documentary_venue(0, grant);
    if short == 1 {
        let expected = limit.checked_add(1).ok_or("venue overflow")?;
        check!(
            matches!(result, Err(crate::CrawlError::Store(StoreError::EvidenceSubjectAttachment(
            StoreAttachmentError::Domain(DomainError::Capacity { resource: "aggregate encoded bytes", needed, available })
        ))) if needed == expected && available == limit)
        );
        check!(grant.ensure_available().is_err());
        check!(linked.documentary_venue(0, grant).is_err());
    } else {
        check!(eq; result?, "Highland Milford High School");
        check!(eq; grant.usage().encoded_bytes(), limit);
        grant.ensure_available()?;
    }
    Ok(cost)
}
