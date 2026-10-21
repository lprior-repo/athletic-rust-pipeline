use super::*;
use census_domain::model::{AthleteId, CanonicalPerformance, SchoolId};
use serde_json::{Map, Value};

pub(super) const ATTACHMENT: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub(super) const SCHOOL: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

pub(super) fn athlete() -> AthleteId {
    AthleteId::mint("ath_subject", &["controlled checked source target"])
}

pub(super) fn school() -> SchoolId {
    SchoolId::mint("sch", &["controlled checked source target"])
}

pub(super) fn target<'a>(
    locator: crate::milesplit::SourceRowLocator,
    subject: &'a str,
    athlete: &'a AthleteId,
    school: &'a SchoolId,
) -> ProposedHtmlResultTargetV1<'a> {
    ProposedHtmlResultTargetV1 {
        locator,
        subject_reference: subject,
        proposed_athlete: athlete,
        proposed_school: school,
        proposed_attachment_reference: ATTACHMENT,
        proposed_school_binding_reference: SCHOOL,
    }
}

pub(super) fn project(
    document: &CheckedHtmlResultDocumentV1<'_>,
    index: usize,
) -> TestResult<(CanonicalMeet, UnadmittedHtmlResultRowV1)> {
    let row = document.row(index).ok_or("controlled checked row")?;
    let subject = row.subject()?.reference();
    let athlete = athlete();
    let school = school();
    let target = target(row.locator(), &subject, &athlete, &school);
    Ok(project_checked_html_result_row(document, index, &target)?)
}

pub(super) fn snapshot(document: &CheckedHtmlResultDocumentV1<'_>) -> TestResult<Vec<u8>> {
    let mut rows = Vec::new();
    let athlete = athlete();
    for index in 0..document.selected_count() {
        let identity = project_checked_html_result_identity(document, index, &athlete)?;
        let (meet, row) = project(document, index)?;
        let performance = row.proposed_performance();
        check!(eq; identity.performance_id(), &performance.id);
        check!(eq; identity.meet_id(), &meet.id);
        check!(eq; identity.event_id(), &performance.event);
        check!(eq; identity.source_key(), performance.source_key);
        check!(eq; identity.date(), performance.date);
        rows.push(serde_json::json!({"meet":meet,"row":row,
            "identity":[identity.performance_id().as_str(),identity.athlete().as_str(),
                identity.meet_id().as_str(),identity.event_id().as_str(),identity.date(),identity.source_key()]}));
    }
    Ok(serde_json::to_vec(&rows)?)
}

pub(super) fn tfrrs() -> TestResult<CanonicalPerformance> {
    let source = CheckedHtmlResultSourceV1::Tfrrs;
    let capture = common_fixture::capture(source)?;
    let document =
        read_retained_source_result_rows(&capture, &common_fixture::locators(source), source)?;
    let (_, row) = project(&document, 0)?;
    Ok(row.into_proposed_entities().2)
}

pub(super) fn note(
    performance: &CanonicalPerformance,
    mutate: impl FnOnce(&mut Map<String, Value>) -> TestResult,
) -> TestResult<CanonicalPerformance> {
    let mut changed = performance.clone();
    let evidence = changed
        .evidence
        .first_mut()
        .ok_or("controlled retained evidence")?;
    let mut value: Value = serde_json::from_str(evidence.note.as_deref().ok_or("retained note")?)?;
    let before = value.clone();
    mutate(value.as_object_mut().ok_or("retained note object")?)?;
    check!(ne; value, before);
    evidence.note = Some(serde_json::to_string(&value)?);
    Ok(changed)
}

pub(super) fn refused(
    actual: &CanonicalPerformance,
    projected: &CanonicalPerformance,
) -> TestResult {
    check!(!matches!(
        matches_physical_observation(actual, projected),
        Ok(true)
    ));
    check!(!matches!(
        matches_physical_observation(projected, actual),
        Ok(true)
    ));
    Ok(())
}
