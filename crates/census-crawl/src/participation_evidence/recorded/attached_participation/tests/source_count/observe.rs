use super::*;
use crate::participation_evidence::recorded::{discover_current_html_results, discover_frozen_html_results};
use std::cell::Cell;

type Observed = Vec<String>;

pub(super) fn current(store: &Store, count: usize) -> TestResult<Observed> {
    let snapshot = store.snapshot();
    let mut grant = Grant::new();
    let owner = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let called = Cell::new(0usize);
    let result = with_current_attached_participation(&owner, &mut grant, |sources, grant| {
        let plan = discover_current_html_results(&owner, grant)?;
        plan.with_group(0, grant, |linked, grant| rows(sources, &linked, grant, &called))
    });
    outcome(result, &grant, &called, count)
}

pub(super) fn frozen(store: &Store, count: usize) -> TestResult<Observed> {
    let snapshot = store.snapshot();
    let accepted = snapshot.accepted_public_identity_snapshot()?.ok_or("accepted graph")?;
    let mut grant = Grant::new();
    let owner = snapshot.read_current_evidence_subject_attachment_references_with_accepted_v1(&accepted, &mut grant)?;
    let bundle = owner.freeze_closure(&mut grant)?;
    let mut replay_grant = Grant::new();
    let frozen = census_store::replay_frozen_evidence_subject_attachment_references_v1(&bundle, &accepted, &mut replay_grant)?;
    let called = Cell::new(0usize);
    let result = with_frozen_attached_participation(&frozen, &mut replay_grant, |sources, grant| {
        let plan = discover_frozen_html_results(&frozen, grant)?;
        plan.with_group(0, grant, |linked, grant| rows(sources, &linked, grant, &called))
    });
    outcome(result, &replay_grant, &called, count)
}

fn rows<'s>(sources: &AttachedParticipationV1<'_, 's>, linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, 's>,
    grant: &mut Grant, called: &Cell<usize>) -> CrawlResult<Observed> {
    sources.with_row_facts(linked, 0, grant, |facts, _| {
        called.set(called.get().checked_add(1).ok_or(census_store::StoreError::CounterOverflow)?);
        let mut references = Vec::new();
        for item in facts.sources() {
            if item.source().cohort().get() != 2026 || item.source().day().as_str() != "2026-05-15" {
                return Err(invalid("count fixture changed documentary applicability").into());
            }
            references.push(item.source().reference().as_str().to_owned());
        }
        Ok(references)
    })
}

fn outcome(result: CrawlResult<Observed>, grant: &Grant, called: &Cell<usize>, count: usize) -> TestResult<Observed> {
    match count {
        16 => {
            let values = result?;
            check!(eq; called.get(), 1);
            check!(eq; values.len(), 16);
            check!(eq; values.iter().collect::<std::collections::BTreeSet<_>>().len(), 16);
            grant.ensure_available()?;
            return Ok(values);
        }
        17 => {
            match result {
                Err(crate::CrawlError::Store(census_store::StoreError::BudgetExceeded {
                    what: "attached participation source count", needed: 17, available: 16,
                })) => (),
                Err(error) => return Err(error.into()),
                Ok(_) => return Err("seventeenth distinct source unexpectedly accepted".into()),
            }
            check!(eq; called.get(), 0);
            check!(grant.ensure_available().is_err());
        }
        _ => return Err("source count boundary is exactly16/17".into()),
    }
    Ok(Vec::new())
}
