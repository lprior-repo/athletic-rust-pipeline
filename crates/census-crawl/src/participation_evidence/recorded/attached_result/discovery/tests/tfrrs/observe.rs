use super::*;
use census_domain::model::{AdjustmentKind, CompetitionLevel};

pub(super) fn current(store: &Store) -> TestResult<Vec<u8>> {
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("accepted native targets")?;
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_evidence_subject_attachment_references_with_accepted_v1(
        &accepted, &mut grant,
    )?;
    let plan = discover_current_html_results(&current, &mut grant)?;
    check!(eq; plan.group_count(), 1);
    check!(eq; plan.diagnostic_count(), 0);
    plan.with_group(0, &mut grant, |document, grant| {
        Ok(document_view(document, grant))
    })?
}

pub(super) fn freeze(store: &Store) -> TestResult<FrozenEvidenceSubjectAttachmentBundleV1> {
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("accepted native targets")?;
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_evidence_subject_attachment_references_with_accepted_v1(
        &accepted, &mut grant,
    )?;
    Ok(current.freeze_closure(&mut grant)?)
}

pub(super) fn frozen(
    store: &Store,
    bundle: &FrozenEvidenceSubjectAttachmentBundleV1,
) -> TestResult<Vec<u8>> {
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("accepted native targets")?;
    let mut grant = AttachmentReplayGrantV1::new();
    let frozen = census_store::replay_frozen_evidence_subject_attachment_references_v1(
        bundle, &accepted, &mut grant,
    )?;
    let plan = discover_frozen_html_results(&frozen, &mut grant)?;
    check!(eq; plan.group_count(), 1);
    check!(eq; plan.diagnostic_count(), 0);
    plan.with_group(0, &mut grant, |document, grant| {
        Ok(document_view(document, grant))
    })?
}

fn document_view(
    linked: DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult<Vec<u8>> {
    check!(eq; linked.row_count(), 2);
    check!(linked.row(2).is_none());
    let CheckedHtmlResultDocumentV1::Tfrrs(checked) = linked.checked_document() else {
        return Err("discovered original TFRRS list".into());
    };
    check!(eq; checked.context_reference(), linked.recorded_root().key);
    check!(eq; checked.list_id(), "5489");
    let mut rows = Vec::new();
    let mut publics = Vec::new();
    for index in 0..2 {
        let row = linked.row(index).ok_or("discovered original row")?;
        let source = checked.row(index).ok_or("checked native row")?;
        let attachment = row.attachment();
        attachment.require_original_grant(grant)?;
        check!(eq; row.checked_row().locator(), setup::locators().get(index).copied().ok_or("genuine locator")?);
        check!(eq; attachment.subject().namespace.as_str(), "tfrrs_result_document_v1");
        check!(eq; attachment.subject().capture, linked.recorded_root().payload.digest);
        check!(eq; attachment.subject().interpretation.as_str(), source.reference());
        check!(eq; attachment.subject().subject.as_str(), source.subject()?.reference());
        check!(eq; attachment.subject().context.as_str(), source.context_reference());
        check!(ne; source.context_reference(), checked.context_reference());
        check!(eq; row.gaps(), &[crate::participation_evidence::recorded::AttachedResultGapV1::SchoolBindingUnresolved]);
        let projection = project(linked.checked_document(), index, attachment)?;
        publics.push(attachment.target().public_id.clone());
        rows.push(serde_json::to_vec(&(
            source.locator(),
            source.reference(),
            source.original_row()?,
            source.original_row_digest(),
            source.context_reference(),
            source.meet_id(),
            source.published_date(),
            attachment.subject(),
            attachment.target(),
            attachment.record(),
            attachment.receipt(),
            attachment.inventory(),
            attachment.accepted_public_head(),
            attachment.snapshot_sequence(),
            projection,
        ))?);
    }
    check!(ne; publics.first(), publics.get(1));
    Ok(serde_json::to_vec(&(
        linked.recorded_root(),
        checked.context_reference(),
        rows,
    ))?)
}

fn project(
    document: &CheckedHtmlResultDocumentV1<'_>,
    index: usize,
    attachment: &crate::participation_evidence::recorded::DiscoveredResultAttachmentV1<'_, '_>,
) -> TestResult<Vec<u8>> {
    let CheckedHtmlResultDocumentV1::Tfrrs(checked) = document else {
        return Err("checked TFRRS source".into());
    };
    let row = checked.row(index).ok_or("native TFRRS row")?;
    let target = setup::target(&row)?;
    check!(eq; attachment.target().canonical_candidate, target.id.cast());
    let subject = row.subject()?.reference();
    let unresolved_school = objects::reference(b"controlled-unadmitted-school-projection-only")?;
    let proposed = ProposedHtmlResultTargetV1 {
        locator: row.locator(),
        subject_reference: &subject,
        proposed_athlete: &target.id,
        proposed_school: &target.school,
        proposed_attachment_reference: attachment.record().record_digest.as_str(),
        proposed_school_binding_reference: unresolved_school.digest.as_str(),
    };
    let (meet, result) = project_checked_html_result_row(document, index, &proposed)?;
    let expected = [
        ("95201", "2026-03-21", "555.75", "562.21", "8650211"),
        ("93917", "2026-03-11", "556.30", "562.77", "8792017"),
    ];
    let (native_meet, day, published, original, native_athlete) =
        expected.get(index).copied().ok_or("expected source row")?;
    check!(eq; row.meet_id(), native_meet);
    check!(eq; meet.date, day);
    check!(eq; meet.state, None);
    check!(eq; meet.level, CompetitionLevel::Unknown);
    let native = meet
        .source_identities
        .first()
        .ok_or("original native meet")?;
    check!(eq; native.namespace, SourceNamespace::TfrrsMeet);
    check!(eq; native.id, native_meet);
    let performance = result.proposed_performance();
    check!(eq; performance.meet, meet.id);
    check!(eq; performance.mark, Mark::TimeSeconds(ExactSeconds::parse(published)?));
    check!(eq; performance.source_athlete.as_ref().ok_or("original native athlete")?.id, native_athlete);
    let adjustment = performance.adjustment.as_ref().ok_or("source conversion")?;
    check!(eq; adjustment.kind, AdjustmentKind::TrackSizeConversion);
    check!(eq; adjustment.original_mark, Some(Mark::TimeSeconds(ExactSeconds::parse(original)?)));
    check!(eq; adjustment.note.as_deref(), row.conversion_note());
    check!(eq; performance.date, row.published_date());
    let evidence = performance
        .evidence
        .first()
        .ok_or("original row provenance")?;
    let note: serde_json::Value =
        serde_json::from_str(evidence.note.as_deref().ok_or("original row note")?)?;
    check!(eq; note["admission"], "unadmitted_projection");
    Ok(serde_json::to_vec(&(meet, result))?)
}
