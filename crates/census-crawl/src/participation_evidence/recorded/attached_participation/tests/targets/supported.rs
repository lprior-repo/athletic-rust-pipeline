use super::super::source_count::{context, material};
use super::*;
use crate::participation_evidence::recorded::{
    discover_current_html_results, discover_frozen_html_results,
    record_independent_participation_packet,
};

type SourceObservation = (
    i16,
    String,
    String,
    String,
    DocumentarySeasonApplicabilityV1,
    DocumentaryCohortAgreementV1,
);
type Observed = Vec<BTreeMap<String, SourceObservation>>;

pub(super) fn scenario(closed_root: &Path, case: TargetCase) -> TestResult {
    let directory = tempfile::tempdir()?;
    let backup = directory.path().join("backup");
    let root = directory.path().join("private-store");
    Store::backup(closed_root, &backup)?;
    Store::restore(&backup, &root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    let original_target = target(&store)?;
    let other = second_target(&store, &writer)?;
    check!(original_target != other);
    let baseline = current(&store, 1)?;
    check!(eq; baseline, frozen(&store, 1)?);
    check!(baseline.iter().all(|row| row.len() == 1));
    let selected = match case {
        TargetCase::Same => &original_target,
        TargetCase::Other => &other,
    };
    let packet = material::packet(1, &material::schedule(&store)?)?;
    let reference = packet.reference()?.as_str().to_owned();
    check!(baseline.iter().all(|row| !row.contains_key(&reference)));
    attach(&store, selected, &packet)?;
    let tables = store.snapshot().tables_digest(&Table::ALL)?;
    let receipts = store.receipt_count()?;
    let observed = current(&store, 2)?;
    check!(eq; observed, frozen(&store, 2)?);
    check!(eq; observed.len(), baseline.len());
    for (mut row, expected) in observed.into_iter().zip(&baseline) {
        match case {
            TargetCase::Same => {
                check!(eq; row.len(), 2);
                let added = row
                    .remove(&reference)
                    .ok_or("supported same-target source missing")?;
                check!(eq; added.0, 2026);
                check!(eq; added.1, "2026-05-15");
            }
            TargetCase::Other => check!(!row.contains_key(&reference)),
        }
        check!(eq; &row, expected);
    }
    check!(eq; store.snapshot().tables_digest(&Table::ALL)?, tables);
    check!(eq; store.receipt_count()?, receipts);
    drop(writer);
    drop(store);
    directory.close()?;
    Ok(())
}

fn attach(
    store: &Store,
    target: &PublicAthleteId,
    packet: &IndependentParticipationPacketV1,
) -> TestResult {
    let recording = crate::Recording::new();
    record_independent_participation_packet(store, &recording, packet)?;
    recording
        .drain()
        .prepare("controlled-supported-target-intake")?
        .apply(store)?;
    let inspection =
        store.inspect_evidence_subject_attachment_v1(AttachmentInspectionRequestV1::Attach {
            target: target.clone(),
            prior: AttachmentPredecessorV1::First,
        })?;
    check!(eq; &inspection.target().public_id, target);
    let context = context::Context::synthetic("controlled-supported-target", store, packet)?;
    let command = context.command("controlled-supported-target-attach", &inspection)?;
    let objects = context.borrowed();
    let prepared = store.prepare_evidence_subject_attachment_v1(
        command,
        EvidenceSubjectAttachmentPreparationV1::New {
            inspection,
            context: &objects,
        },
    )?;
    check!(!prepared.is_replay());
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    check!(matches!(
        store.apply_evidence_subject_attachment_v1(prepared, &approval, &digest)?,
        Application::Written(_)
    ));
    Ok(())
}

fn current(store: &Store, source_count: usize) -> TestResult<Observed> {
    let snapshot = store.snapshot();
    let mut grant = Grant::new();
    let owner = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let observed = with_current_attached_participation(&owner, &mut grant, |sources, grant| {
        let plan = discover_current_html_results(&owner, grant)?;
        plan.with_group(0, grant, |linked, grant| {
            rows(sources, &linked, grant, source_count)
        })
    })?;
    grant.ensure_available()?;
    check!(eq; observed.len(), 3);
    Ok(observed)
}

fn frozen(store: &Store, source_count: usize) -> TestResult<Observed> {
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("accepted two-target graph")?;
    let mut grant = Grant::new();
    let owner = snapshot.read_current_evidence_subject_attachment_references_with_accepted_v1(
        &accepted, &mut grant,
    )?;
    let bundle = owner.freeze_closure(&mut grant)?;
    let mut replay = Grant::new();
    let owner = census_store::replay_frozen_evidence_subject_attachment_references_v1(
        &bundle,
        &accepted,
        &mut replay,
    )?;
    let observed = with_frozen_attached_participation(&owner, &mut replay, |sources, grant| {
        let plan = discover_frozen_html_results(&owner, grant)?;
        plan.with_group(0, grant, |linked, grant| {
            rows(sources, &linked, grant, source_count)
        })
    })?;
    replay.ensure_available()?;
    check!(eq; observed.len(), 3);
    Ok(observed)
}

fn rows<'s>(
    sources: &AttachedParticipationV1<'_, 's>,
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, 's>,
    grant: &mut Grant,
    source_count: usize,
) -> CrawlResult<Observed> {
    if sources.sources.len() != source_count || !sources.unsupported.is_empty() {
        return Err(
            invalid("supported target control did not decode every supported status").into(),
        );
    }
    let mut observed = Vec::new();
    for index in 0..linked.row_count() {
        observed.push(sources.with_row_facts(linked, index, grant, |facts, _| {
            let mut row = BTreeMap::new();
            for item in facts.sources() {
                let source = item.source();
                if source.cohort().get() != 2026
                    || source.day().as_str() != "2026-05-15"
                    || item.season()
                        != DocumentarySeasonApplicabilityV1::CompetitionAndVenueMismatch
                    || item.cohort_agreement() != DocumentaryCohortAgreementV1::Agreed
                {
                    return Err(invalid(
                        "supported target control changed genuine CO2026 source facts",
                    )
                    .into());
                }
                let previous = row.insert(
                    source.reference().as_str().to_owned(),
                    (
                        source.cohort().get(),
                        source.day().as_str().to_owned(),
                        source.status_proof().capture().to_owned(),
                        item.attachment_record_digest().to_owned(),
                        item.season(),
                        item.cohort_agreement(),
                    ),
                );
                if previous.is_some() {
                    return Err(
                        invalid("supported target control duplicated source authority").into(),
                    );
                }
            }
            Ok(row)
        })?);
    }
    Ok(observed)
}
