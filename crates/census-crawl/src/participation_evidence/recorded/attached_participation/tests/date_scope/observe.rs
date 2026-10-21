use super::*;
use crate::milesplit::SourceRowLocator;
use crate::participation_evidence::recorded::{
    discover_current_html_results, with_current_attached_participation, AttachedParticipationV1,
    DiscoveredIdentityLinkedHtmlResultDocumentV1,
};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1 as Grant;

pub(super) struct ObservedRow {
    original: (String, SourceRowLocator, String),
    day: String,
    sources: Vec<(String, String)>,
}

pub(super) fn current(
    store: &Store,
    target: &PublicAthleteId,
    packet_one: &crate::participation_evidence::IndependentParticipationPacketV1,
    packet_two: &crate::participation_evidence::IndependentParticipationPacketV1,
) -> TestResult {
    let matching = packet_one.reference()?;
    let other_day = packet_two.reference()?;
    let (roots, records) = super::withdraw::matching(store, target)?;
    let tables = store.snapshot().tables_digest(&Table::ALL)?;
    let before = rows(store)?;
    check!(eq; before.len(), 3);
    for row in &before {
        check!(eq; row.day, "2026-05-15");
        check!(row
            .sources
            .iter()
            .any(|(reference, _)| reference == matching.as_str()));
        check!(row
            .sources
            .iter()
            .all(|(reference, day)| { reference != other_day.as_str() && day == &row.day }));
    }
    for (index, record) in records.iter().enumerate() {
        super::withdraw::revoke(store, record, &format!("controlled-date-revoke-{index}"))?;
    }
    let after = rows(store)?;
    check!(before
        .iter()
        .map(|row| &row.original)
        .eq(after.iter().map(|row| &row.original)));
    check!(after.iter().all(|row| row.sources.is_empty()));
    check!(eq; store.snapshot().tables_digest(&Table::ALL)?, tables);
    super::withdraw::require_retained(store, &roots)
}

fn rows(store: &Store) -> TestResult<Vec<ObservedRow>> {
    let snapshot = store.snapshot();
    let mut grant = Grant::new();
    let owner = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    Ok(with_current_attached_participation(
        &owner,
        &mut grant,
        |sources, grant| {
            let plan = discover_current_html_results(&owner, grant)?;
            let mut rows = Vec::new();
            for index in 0..plan.group_count() {
                rows.extend(
                    plan.with_group(index, grant, |linked, grant| group(&linked, sources, grant))?,
                );
            }
            Ok(rows)
        },
    )?)
}

fn group<'s>(
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, 's>,
    sources: &AttachedParticipationV1<'_, 's>,
    grant: &mut Grant,
) -> crate::CrawlResult<Vec<ObservedRow>> {
    let mut rows = Vec::new();
    for index in 0..linked.row_count() {
        let row = linked.row(index).ok_or_else(|| {
            crate::participation_evidence::invalid("controlled result row absent")
        })?;
        let checked = row.checked_row();
        let documentary = sources.with_row_facts(linked, index, grant, |facts, _| {
            Ok(facts
                .sources()
                .iter()
                .map(|item| {
                    let source = item.source();
                    (
                        source.reference().as_str().to_owned(),
                        source.day().as_str().to_owned(),
                    )
                })
                .collect())
        })?;
        rows.push(ObservedRow {
            original: (
                checked.context_reference().to_owned(),
                checked.locator(),
                checked.original_row_digest().to_owned(),
            ),
            day: linked
                .checked_document()
                .as_milesplit()
                .ok_or_else(|| {
                    crate::participation_evidence::invalid("controlled MileSplit context absent")
                })?
                .published_start()
                .to_owned(),
            sources: documentary,
        });
    }
    Ok(rows)
}
