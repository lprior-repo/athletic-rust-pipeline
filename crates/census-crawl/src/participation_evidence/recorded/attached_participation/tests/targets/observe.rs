use super::*;
use crate::participation_evidence::recorded::{discover_current_html_results, discover_frozen_html_results};
use std::cell::Cell;

type Observed = (String, i16, String, String, DocumentarySeasonApplicabilityV1);

pub(super) fn current(store: &Store, case: TargetCase) -> TestResult<Vec<Observed>> {
    let snapshot = store.snapshot();
    let mut grant = Grant::new();
    let owner = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let called = Cell::new(0usize);
    let result = with_current_attached_participation(&owner, &mut grant, |sources, grant| {
        let plan = discover_current_html_results(&owner, grant)?;
        plan.with_group(0, grant, |linked, grant| rows(sources, &linked, grant, &called))
    });
    outcome(result, &grant, &called, case)
}

pub(super) fn frozen(store: &Store, case: TargetCase) -> TestResult<Vec<Observed>> {
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
    outcome(result, &replay_grant, &called, case)
}

fn rows<'s>(sources: &AttachedParticipationV1<'_, 's>, linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, 's>,
    grant: &mut Grant, called: &Cell<usize>) -> CrawlResult<Vec<Observed>> {
    let mut values = Vec::new();
    for index in 0..linked.row_count() {
        let row = sources.with_row_facts(linked, index, grant, |facts, _| {
            called.set(called.get().checked_add(1).ok_or(census_store::StoreError::CounterOverflow)?);
            let [item] = facts.sources() else { return Err(invalid("exact genuine status source count").into()); };
            Ok((item.source().reference().as_str().to_owned(), item.source().cohort().get(),
                item.source().day().as_str().to_owned(), item.attachment_record_digest().to_owned(), item.season()))
        })?;
        values.push(row);
    }
    Ok(values)
}

fn outcome(result: CrawlResult<Vec<Observed>>, grant: &Grant, called: &Cell<usize>, case: TargetCase) -> TestResult<Vec<Observed>> {
    match case {
        TargetCase::Same => {
            check!(matches!(result, Err(crate::CrawlError::Store(census_store::StoreError::Refused { detail }))
                if detail == "documentary target has unsupported retained status authority"));
            check!(eq; called.get(), 0);
            check!(grant.ensure_available().is_err());
            Ok(Vec::new())
        }
        TargetCase::Other => {
            let values = result?;
            check!(eq; called.get(), 3);
            check!(eq; values.len(), 3);
            check!(values.iter().all(|value| value.1 == 2026 && value.2 == "2026-05-15"
                && value.4 == DocumentarySeasonApplicabilityV1::CompetitionAndVenueMismatch));
            grant.ensure_available()?;
            Ok(values)
        }
    }
}
