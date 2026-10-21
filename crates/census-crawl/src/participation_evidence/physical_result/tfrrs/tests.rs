use super::*;
use crate::milesplit::SourceRowLocator;
use crate::participation_evidence::result_ownership::tfrrs::read_retained_tfrrs_list_rows;
use crate::participation_evidence::RetainedHtmlCaptureV1;
use census_domain::model::{
    AdjustmentBasis, AdjustmentKind, AthleteId, CompetitionLevel, EventKind, ExactSeconds, Gender,
    Mark, SchoolId, SourceNamespace, Sport,
};

mod fixture;
mod refusal;
mod replay;

type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn genuine_justin_projection_keeps_published_and_original_conversion_marks() -> TestResult {
    let capture = fixture::capture(fixture::BODY)?;
    let list = read_retained_tfrrs_list_rows(&capture, &fixture::locators())?;
    let source = list.row(0).ok_or("Justin source")?;
    let subject = source.subject()?.reference();
    let athlete = fixture::athlete();
    let school = fixture::school();
    let target = fixture::target(source.locator(), &subject, &athlete, &school);
    let (meet, row) = project_retained_tfrrs_result_row(&list, 0, &target)?;
    check!(eq; meet.name, source.meet_name());
    check!(eq; meet.date, "2026-03-21");
    check!(eq; meet.state, None);
    check!(eq; meet.level, CompetitionLevel::Unknown);
    check!(eq; meet.sports, vec![Sport::IndoorTrack]);
    let native = meet.source_identities.first().ok_or("native meet")?;
    check!(eq; native.namespace, SourceNamespace::TfrrsMeet);
    check!(eq; native.id, "95201");
    check!(eq; row.subject_reference(), subject);
    check!(eq; row.locator(), source.locator());
    check!(eq; row.proposed_attachment_reference(), fixture::ATTACHMENT);
    check!(eq; row.proposed_school_binding_reference(), fixture::SCHOOL);
    let performance = row.proposed_performance();
    check!(eq; performance.mark, Mark::TimeSeconds(ExactSeconds::parse("555.75")?));
    check!(eq; performance.date, "2026-03-21");
    check!(eq; performance.athlete, athlete);
    check!(eq; performance.observed_grade.map(|grade| grade.get()), Some(11));
    check!(eq; performance.timing, None);
    check!(eq; performance.wind_mps, None);
    check!(eq; performance.place, None);
    let adjustment = performance.adjustment.as_ref().ok_or("conversion")?;
    check!(eq; adjustment.kind, AdjustmentKind::TrackSizeConversion);
    check!(eq; adjustment.basis, AdjustmentBasis::Known);
    check!(eq; adjustment.original_mark, Some(Mark::TimeSeconds(ExactSeconds::parse("562.21")?)));
    check!(eq; adjustment.adjusted_mark, performance.mark);
    check!(eq; adjustment.note.as_deref(), source.conversion_note());
    check!(performance.source_key.starts_with("tfrrs_html_row_v1:"));
    check!(eq; performance.source_athlete.as_ref().ok_or("native athlete")?.id, "8650211");
    check!(eq; row.proposed_event().kind, EventKind::Track3200m);
    check!(eq; row.proposed_event().gender, Gender::Boys);
    check!(row
        .proposed_event()
        .source_bound_kind(&EventKind::Track3200m));
    check!(eq; row.proposed_team().school, school);
    check!(eq; row.proposed_team().school_year.get(), 2025);
    check!(eq; row.proposed_team().source_identities.first().ok_or("native team")?.id, "Columbus_North");
    let evidence = performance.evidence.first().ok_or("source evidence")?;
    let note: serde_json::Value =
        serde_json::from_str(evidence.note.as_deref().ok_or("source note")?)?;
    check!(eq; note["admission"], "unadmitted_projection");
    check!(eq; note["conversion_note"], "Converted from 9:22.21 for Track Size.");
    check!(eq; note["original_row_digest"], source.original_row_digest());
    check!(eq; note["context_reference"], source.context_reference());
    check!(eq; note["conversion_note_span"], serde_json::to_value(source.conversion_note_span())?);
    Ok(())
}

#[test]
fn adjacent_genuine_rows_project_different_native_meets_and_exact_source_days() -> TestResult {
    let capture = fixture::capture(fixture::BODY)?;
    let list = read_retained_tfrrs_list_rows(&capture, &fixture::locators())?;
    let athlete = fixture::athlete();
    let school = fixture::school();
    let mut outputs = Vec::new();
    for index in 0..2 {
        let source = list.row(index).ok_or("adjacent row")?;
        let subject = source.subject()?.reference();
        let target = fixture::target(source.locator(), &subject, &athlete, &school);
        outputs.push(project_retained_tfrrs_result_row(&list, index, &target)?);
    }
    let mut outputs = outputs.into_iter();
    let (justin_meet, justin) = outputs.next().ok_or("Justin")?;
    let (isaac_meet, isaac) = outputs.next().ok_or("Isaac")?;
    check!(eq; isaac_meet.date, "2026-03-11");
    check!(eq; isaac_meet.name, "Brownsburg HCC Indoor");
    check!(eq; isaac_meet.source_identities.first().ok_or("Isaac meet")?.id, "93917");
    check!(justin_meet.id != isaac_meet.id);
    check!(eq; justin.proposed_performance().meet, justin_meet.id);
    check!(eq; isaac.proposed_performance().meet, isaac_meet.id);
    check!(justin.proposed_event().id != isaac.proposed_event().id);
    check!(justin.proposed_performance().source_key != isaac.proposed_performance().source_key);
    check!(eq; isaac.proposed_performance().mark, Mark::TimeSeconds(ExactSeconds::parse("556.30")?));
    Ok(())
}
