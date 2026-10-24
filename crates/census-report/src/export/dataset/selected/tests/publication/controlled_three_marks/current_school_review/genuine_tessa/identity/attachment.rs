use super::*;
use census_crawl::participation_evidence::result_ownership::CheckedHtmlResultDocumentV1;

pub(in super::super) fn unattached(store: &Store) -> TestResult {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    check!(eq; discover_current_html_results(&current,&mut grant)?.group_count(),0);
    Ok(())
}

pub(in super::super) fn linked(
    store: &Store,
    target: &CanonicalAthlete,
    command: &RetainedEvidenceSubjectAttachmentCommandV1,
    receipt: &census_store::Receipt,
) -> TestResult<Vec<u8>> {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let plan = discover_current_html_results(&current, &mut grant)?;
    check!(eq; plan.group_count(),1);
    check!(eq; plan.diagnostic_count(),0);
    plan.with_group(0, &mut grant, |linked, _| {
        Ok(read(&linked, target, command, receipt))
    })?
}

fn read(
    linked: &census_crawl::participation_evidence::recorded::DiscoveredIdentityLinkedHtmlResultDocumentV1<'_,'_>,
    target: &CanonicalAthlete,
    command: &RetainedEvidenceSubjectAttachmentCommandV1,
    receipt: &census_store::Receipt,
) -> TestResult<Vec<u8>> {
    let CheckedHtmlResultDocumentV1::Tfrrs(document) = linked.checked_document() else {
        return Err("actual TFRRS source document required".into());
    };
    let native = document.row(0).ok_or("original native TFRRS row")?;
    check!(eq; native.locator(),source::locator());
    check!(eq; native.athlete_id(),"8779607");
    check!(eq; native.original_row_digest(),"d33bf3fb6858f07f8e41576232afd0bf89c848e420fbe67bef779f0be6f1df8d");
    check!(eq; native.original_mark(),Some("11:29.42"));
    check!(eq; native.published_mark(),"11:23.56");
    check!(eq; native.performance_id(),"5841286");
    let row = linked
        .row(0)
        .ok_or("document mapped solely through protected attachment")?;
    let attachment = row.attachment();
    check!(eq; attachment.target().canonical_candidate,target.id.cast());
    check!(eq; &attachment.record().command,command);
    check!(eq; attachment.receipt(),receipt);
    Ok(serde_json::to_vec(&(
        linked.recorded_root(),
        native.locator(),
        native.reference(),
        native.context_reference(),
        native.athlete_id(),
        native.original_row()?,
        attachment.record(),
        attachment.receipt(),
    ))?)
}
