use super::*;
use crate::participation_evidence::recorded::discover_current_html_results;
use census_domain::public_identity_history::ReplayLimits;
use census_store::{Store, Table};
use std::cell::Cell;

mod frozen;

pub(in crate::participation_evidence::recorded) fn verify_adversarial(store: &Store) -> TestResult {
    let before = store.snapshot().tables_digest(&Table::ALL)?;
    let receipts = store.receipt_count()?;
    let baseline = exercise(store, Boundary::Measure)?;
    check!(baseline.0 > 0 && baseline.1 > 0);
    for boundary in [Boundary::Bytes(baseline.0), Boundary::Steps(baseline.1)] {
        exercise(store, boundary)?;
    }
    callback_refusal(store)?;
    frozen::verify(store)?;
    check!(eq; store.snapshot().tables_digest(&Table::ALL)?, before);
    check!(eq; store.receipt_count()?, receipts);
    Ok(())
}

#[derive(Clone, Copy)]
enum Boundary { Measure, Bytes(usize), Steps(usize) }

fn exercise(store: &Store, boundary: Boundary) -> TestResult<(usize, usize)> {
    let mut observed = (0, 0);
    let attempts = match boundary { Boundary::Measure => 1, _ => 2 };
    for short in 0..attempts {
        let snapshot = store.snapshot();
        let mut grant = Grant::new();
        let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
        let value = with_current_attached_participation(&current, &mut grant, |sources, grant| {
            let plan = discover_current_html_results(&current, grant)?;
            plan.with_group(0, grant, |linked, grant| {
                Ok(boundary_row(sources, &linked, grant, boundary, short))
            })
        })??;
        observed = value;
    }
    Ok(observed)
}

fn boundary_row<'s>(
    sources: &AttachedParticipationV1<'_, 's>,
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, 's>,
    grant: &mut Grant,
    boundary: Boundary,
    short: usize,
) -> TestResult<(usize, usize)> {
    let before = grant.usage();
    match boundary {
        Boundary::Measure => {}
        Boundary::Bytes(cost) => grant.charge_bytes(ReplayLimits::SMALL.total_text_bytes
            .checked_sub(before.encoded_bytes()).and_then(|n| n.checked_sub(cost))
            .and_then(|n| n.checked_add(short)).ok_or("byte boundary")?)?,
        Boundary::Steps(cost) => grant.charge_steps(ReplayLimits::SMALL.traversal_steps
            .checked_sub(before.traversal_steps()).and_then(|n| n.checked_sub(cost))
            .and_then(|n| n.checked_add(short)).ok_or("step boundary")?)?,
    }
    let start = grant.usage();
    let called = Cell::new(false);
    let result = sources.with_row_facts(linked, 0, grant, |facts, _| {
        called.set(true);
        if facts.sources().len() != 1 {
            return Err(invalid("genuine semantic boundary source missing").into());
        }
        Ok(())
    });
    if short == 1 {
        let resource = match boundary { Boundary::Bytes(_) => "aggregate encoded bytes", _ => "aggregate traversal steps" };
        check!(matches!(result, Err(crate::CrawlError::Store(census_store::StoreError::EvidenceSubjectAttachment(
            census_store::EvidenceSubjectAttachmentStoreError::Domain(census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Capacity { resource: found, .. })
        ))) if found == resource));
        check!(!called.get());
        check!(grant.ensure_available().is_err());
        return Ok((0, 0));
    }
    result?;
    check!(called.get());
    grant.ensure_available()?;
    let after = grant.usage();
    match boundary {
        Boundary::Bytes(_) => check!(eq; after.encoded_bytes(), ReplayLimits::SMALL.total_text_bytes),
        Boundary::Steps(_) => check!(eq; after.traversal_steps(), ReplayLimits::SMALL.traversal_steps),
        Boundary::Measure => {}
    }
    Ok((after.encoded_bytes().checked_sub(start.encoded_bytes()).ok_or("byte delta")?,
        after.traversal_steps().checked_sub(start.traversal_steps()).ok_or("step delta")?))
}

fn callback_refusal(store: &Store) -> TestResult {
    let snapshot = store.snapshot();
    let mut grant = Grant::new();
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let called = Cell::new(false);
    let result = with_current_attached_participation(&current, &mut grant, |sources, grant| {
        let plan = discover_current_html_results(&current, grant)?;
        plan.with_group(0, grant, |linked, grant| {
            sources.with_row_facts(&linked, 0, grant, |_, _| {
                called.set(true);
                Err::<(), _>(census_store::StoreError::Refused { detail: "controlled semantic callback refusal".into() }.into())
            })
        })
    });
    check!(called.get());
    check!(matches!(result, Err(crate::CrawlError::Store(census_store::StoreError::Refused { detail }))
        if detail == "controlled semantic callback refusal"));
    check!(grant.ensure_available().is_err());
    Ok(())
}
