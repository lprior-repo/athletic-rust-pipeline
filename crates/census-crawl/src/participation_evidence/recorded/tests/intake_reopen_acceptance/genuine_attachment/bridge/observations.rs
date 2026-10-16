use super::*;

pub(super) fn verify(
    linked: &IdentityLinkedHtmlResultDocumentV1<'_, '_>,
    sources: &Sources,
    installed: &super::super::super::accepted_provider::installed::Installed,
    target: &IdentityCommitOutcomeV1,
) -> TestResult {
    let document = linked.checked_document();
    check!(eq; linked.row_count(), 3);
    check!(eq; linked.recorded_root(), sources.result.root());
    check!(eq; document.context_reference(), sources.result.root().key);
    check!(eq; document.original_document().as_bytes(), HTML);
    check!(eq; document.meet_id(), "763888");
    check!(eq; document.result_document_id(), "1304896");
    check!(eq; document.published_start(), "2026-05-15");
    let held_sequence = linked.snapshot().sequence();
    check!(!document.original_document().contains("7223777"));
    check!(!document.original_document().contains("12439"));
    for (index, (mark, wind, digest)) in [
        (
            "13.57",
            Some(0.2),
            "bc7c7f83bcfc715c9c3343e09cf45f82bee7651ac86838f3427e1097de010b3d",
        ),
        (
            "14.10",
            None,
            "6bf39ca146977ced76ba63e226c1d41786e1914947f492c148fe3ca8708cd582",
        ),
        (
            "36.23",
            None,
            "8595ae83c23900e31b747b8bc0b3c6adb7c842139e0344f15e361a71e718d736",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let linked = linked.row(index).ok_or("complete checked attached row")?;
        let row = linked.checked_row();
        let attachment = linked.attachment();
        let subject = attachment.subject();
        check!(eq; row.locator(), *locators().get(index).ok_or("locator")?);
        check!(eq; row.parsed_row().mark, Mark::TimeSeconds(ExactSeconds::parse(mark)?));
        check!(eq; row.parsed_row().wind_mps, wind);
        check!(eq; row.parsed_row().timing, Some(TimingMethod::Fat));
        check!(eq; row.parsed_row().grade.map(|grade| grade.get()), Some(12));
        check!(eq; row.event().gender, Gender::Boys);
        check!(eq; row.event().kind, if index == 2 { EventKind::Track300mHurdles } else { EventKind::Track110mHurdles });
        check!(eq; row.original_row_digest(), digest);
        check!(eq; row.reference(), subject.interpretation.as_str());
        check!(eq; row.subject()?.reference(), subject.subject.as_str());
        check!(eq; hash(row.subject()?.locator().as_bytes()), subject.locator.as_str());
        check!(eq; subject.capture, sources.result.root().payload.digest);
        check!(eq; subject.body.as_str(), document.capture().body_sha256());
        check!(eq; subject.context.as_str(), document.context_reference());
        check!(eq; subject.role, AttachmentSubjectRoleV1::ResultRow);
        check!(eq; &attachment.target().public_id, target.public_id());
        check!(eq; attachment.target().canonical_candidate, installed.candidate);
        check!(eq; attachment.record().command.operation.as_str(), RESULTS_OPERATION);
        check!(eq; attachment.inventory().records, 2);
        check!(eq; attachment.receipt().operation, format!("evidence_subject_attachment_v1:{RESULTS_OPERATION}"));
        check!(eq; attachment.snapshot().sequence(), held_sequence);
        check!(eq; linked.gaps(), &[AttachedResultGapV1::SchoolBindingUnresolved]);
    }
    Ok(())
}
