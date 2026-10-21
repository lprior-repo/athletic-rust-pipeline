use super::*;
use census_domain::model::{AdjustmentKind, EventKind};

pub(super) fn read(
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    school: &school::read::CheckedResultSchoolCorrespondenceV1<'_>,
) -> TestResult {
    let source = linked.row(0).ok_or("identity-linked actual Tessa row")?;
    let row = linked
        .checked_document()
        .row(0)
        .ok_or("checked actual Tessa row")?;
    check!(eq; row.locator(), source::locator());
    let subject = row.subject()?.reference();
    let athlete = source.attachment().target().canonical_candidate.cast();
    let target = ProposedHtmlResultTargetV1 {
        locator: row.locator(),
        subject_reference: &subject,
        proposed_athlete: &athlete,
        proposed_school: school.school(),
        proposed_attachment_reference: source.attachment().record().record_digest.as_str(),
        proposed_school_binding_reference: school.record().record_digest.as_str(),
    };
    let (meet, projected) = project_checked_html_result_row(linked.checked_document(), 0, &target)?;
    check!(eq; meet.date, "2026-03-11");
    check!(eq; projected.proposed_school_binding_reference(), school.record().record_digest.as_str());
    check!(eq; projected.proposed_event().kind, EventKind::Track3200m);
    check!(eq; projected.proposed_event().gender, Gender::Girls);
    let performance = projected.proposed_performance();
    check!(eq; &projected.proposed_team().school, school.school());
    check!(eq; performance.mark, Mark::TimeSeconds(ExactSeconds::parse("683.56")?));
    check!(performance.place.is_none());
    let adjustment = performance
        .adjustment
        .as_ref()
        .ok_or("genuine provider conversion")?;
    check!(eq; adjustment.kind, AdjustmentKind::TrackSizeConversion);
    check!(eq; adjustment.original_mark, Some(Mark::TimeSeconds(ExactSeconds::parse("689.42")?)));
    let CheckedHtmlResultDocumentV1::Tfrrs(document) = linked.checked_document() else {
        return Err("genuine Tessa source namespace".into());
    };
    let row = document.row(0).ok_or("native original Tessa row")?;
    check!(eq; row.original_mark(), Some("11:29.42"));
    check!(eq; row.published_mark(), "11:23.56");
    check!(eq; adjustment.note.as_deref(), row.conversion_note());
    Ok(())
}
