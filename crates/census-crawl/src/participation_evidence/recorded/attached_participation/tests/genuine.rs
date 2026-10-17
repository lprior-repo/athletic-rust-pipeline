use super::*;
use crate::participation_evidence::recorded::{
    discover_current_html_results, discover_frozen_html_results,
};
use census_store::{Store, Table};

pub(in crate::participation_evidence::recorded) fn verify_genuine(store: &Store) -> TestResult {
    let snapshot = store.snapshot();
    let tables = snapshot.tables_digest(&Table::ALL)?;
    let receipts = store.receipt_count()?;
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("genuine accepted authority")?;
    let mut grant = Grant::new();
    trace("current-owner-start", &grant);
    let current = snapshot.read_current_evidence_subject_attachment_references_with_accepted_v1(
        &accepted, &mut grant,
    )?;
    trace("current-owner-ready", &grant);
    let observed = with_current_attached_participation(&current, &mut grant, |sources, grant| {
        trace("current-status-ready", grant);
        let plan = discover_current_html_results(&current, grant)?;
        trace("current-discovery-ready", grant);
        plan.with_group(0, grant, |linked, grant| {
            trace("current-result-ready", grant);
            observe(sources, &linked, grant)
        })
    })?;
    check!(eq; observed.len(), 3);
    trace("current-observed-freeze-start", &grant);
    let bundle = current.freeze_closure(&mut grant)?;
    trace("current-freeze-ready", &grant);
    let mut replay_grant = Grant::new();
    trace("frozen-replay-start", &replay_grant);
    let frozen = census_store::replay_frozen_evidence_subject_attachment_references_v1(
        &bundle,
        &accepted,
        &mut replay_grant,
    )?;
    trace("frozen-replay-ready", &replay_grant);
    let restored =
        with_frozen_attached_participation(&frozen, &mut replay_grant, |sources, grant| {
            trace("frozen-status-ready", grant);
            let plan = discover_frozen_html_results(&frozen, grant)?;
            trace("frozen-discovery-ready", grant);
            plan.with_group(0, grant, |linked, grant| {
                trace("frozen-result-ready", grant);
                observe(sources, &linked, grant)
            })
        })?;
    check!(eq; restored, observed);
    check!(eq; snapshot.tables_digest(&Table::ALL)?, tables);
    check!(eq; store.receipt_count()?, receipts);
    eprintln!("attached_participation_trace stage=venue-boundary-controls");
    super::venue::verify_venue(store)
}

type Observed = (
    i16,
    String,
    String,
    String,
    String,
    DocumentarySeasonApplicabilityV1,
);

fn observe(
    sources: &AttachedParticipationV1<'_, '_>,
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    grant: &mut Grant,
) -> CrawlResult<Vec<Observed>> {
    let mut output = Vec::new();
    for index in 0..linked.row_count() {
        eprintln!(
            "attached_participation_trace row={index} before={:?}",
            grant.usage()
        );
        let fact = sources.with_row_facts(linked, index, grant, |facts, _| {
            if facts.sources().len() != 1 {
                return Err(invalid("genuine semantic source count differs").into());
            }
            let item = facts
                .sources()
                .first()
                .ok_or_else(|| invalid("genuine status source absent"))?;
            let checked = item.source();
            if checked.cohort().get() != 2026
                || checked.day().as_str() != "2026-05-15"
                || item.season() != DocumentarySeasonApplicabilityV1::CompetitionAndVenueMismatch
            {
                return Err(invalid("genuine semantic facts changed").into());
            }
            Ok((
                checked.cohort().get(),
                checked.day().as_str().to_owned(),
                checked.reference().as_str().to_owned(),
                item.attachment_record_digest().to_owned(),
                checked.status_proof().capture().to_owned(),
                item.season(),
            ))
        })?;
        eprintln!(
            "attached_participation_trace row={index} after={:?}",
            grant.usage()
        );
        output.push(fact);
    }
    let mut foreign = Grant::new();
    let refusal = sources.with_row_facts(linked, 0, &mut foreign, |_, _| Ok(()));
    if refusal.is_ok() || foreign.ensure_available().is_ok() {
        return Err(invalid("foreign documentary grant was accepted or left usable").into());
    }
    grant
        .ensure_available()
        .map_err(census_store::EvidenceSubjectAttachmentStoreError::from)
        .map_err(census_store::StoreError::from)?;
    Ok(output)
}

fn trace(stage: &str, grant: &Grant) {
    eprintln!(
        "attached_participation_trace stage={stage} usage={:?}",
        grant.usage()
    );
}
