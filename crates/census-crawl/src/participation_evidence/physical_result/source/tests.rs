use super::*;
use crate::participation_evidence::physical_result::matches_physical_observation;
use crate::participation_evidence::result_ownership::{
    common_fixture, read_retained_source_result_rows, CheckedHtmlResultSourceV1,
};
use crate::participation_evidence::RetainedHtmlCaptureV1;
use census_domain::model::{AdjustmentKind, ExactSeconds, Mark};

mod equality;
mod fixture;
mod notes;

type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn dispatched_milesplit_rows_and_identities_equal_existing_source_projections() -> TestResult {
    let source = CheckedHtmlResultSourceV1::MileSplit;
    let capture = common_fixture::capture(source)?;
    let document =
        read_retained_source_result_rows(&capture, &common_fixture::locators(source), source)?;
    let CheckedHtmlResultDocumentV1::MileSplit(native) = &document else {
        return Err("checked MileSplit source".into());
    };
    let athlete = fixture::athlete();
    let school = fixture::school();
    for index in 0..3 {
        let common = fixture::project(&document, index)?;
        let row = native.row(index).ok_or("original MileSplit row")?;
        let subject = row.subject()?.reference();
        let target = fixture::target(row.locator(), &subject, &athlete, &school);
        let direct = project_retained_html_result_row(native, index, &target)?;
        check!(eq; serde_json::to_vec(&common.0)?, serde_json::to_vec(direct.proposed_meet())?);
        check!(eq; direct.rows().len(), 1);
        check!(eq; serde_json::to_vec(&common.1)?, serde_json::to_vec(direct.rows().first().ok_or("source projection")?)?);
        let original = project_retained_html_result_identity(native, index, &athlete)?;
        let dispatched = project_checked_html_result_identity(&document, index, &athlete)?;
        check!(eq; original.performance_id(), dispatched.performance_id());
        check!(eq; original.source_key(), dispatched.source_key());
        check!(eq; original.event_id(), dispatched.event_id());
    }
    Ok(())
}

#[test]
fn dispatched_genuine_tfrrs_rows_keep_adjustments_and_different_meet_days() -> TestResult {
    let source = CheckedHtmlResultSourceV1::Tfrrs;
    let capture = common_fixture::capture(source)?;
    let document =
        read_retained_source_result_rows(&capture, &common_fixture::locators(source), source)?;
    for (index, (day, published, original, native)) in [
        ("2026-03-21", "555.75", "562.21", "95201"),
        ("2026-03-11", "556.30", "562.77", "93917"),
    ]
    .into_iter()
    .enumerate()
    {
        let (meet, row) = fixture::project(&document, index)?;
        let performance = row.proposed_performance();
        check!(eq; meet.date, day);
        check!(eq; meet.source_identities.first().ok_or("native meet")?.id, native);
        check!(eq; performance.mark, Mark::TimeSeconds(ExactSeconds::parse(published)?));
        let adjustment = performance
            .adjustment
            .as_ref()
            .ok_or("retained conversion")?;
        check!(eq; adjustment.original_mark, Some(Mark::TimeSeconds(ExactSeconds::parse(original)?)));
        check!(eq; adjustment.kind, AdjustmentKind::TrackSizeConversion);
        check!(matches_physical_observation(performance, performance)?);
    }
    Ok(())
}

#[test]
fn cold_checked_source_replay_preserves_all_projections_and_identity_bytes() -> TestResult {
    for source in [
        CheckedHtmlResultSourceV1::MileSplit,
        CheckedHtmlResultSourceV1::Tfrrs,
    ] {
        let capture = common_fixture::capture(source)?;
        let locators = common_fixture::locators(source);
        let document = read_retained_source_result_rows(&capture, &locators, source)?;
        let expected = fixture::snapshot(&document)?;
        let bytes = serde_json::to_vec(&capture)?;
        let cold: RetainedHtmlCaptureV1 = serde_json::from_slice(&bytes)?;
        let reopened = read_retained_source_result_rows(&cold, &locators, source)?;
        check!(eq; fixture::snapshot(&reopened)?, expected);
    }
    Ok(())
}

#[test]
fn common_dispatch_does_not_admit_an_unselected_row_or_changed_target() -> TestResult {
    for source in [
        CheckedHtmlResultSourceV1::MileSplit,
        CheckedHtmlResultSourceV1::Tfrrs,
    ] {
        let capture = common_fixture::capture(source)?;
        let document =
            read_retained_source_result_rows(&capture, &common_fixture::locators(source), source)?;
        let row = document.row(0).ok_or("selected row")?;
        let subject = row.subject()?.reference();
        let athlete = fixture::athlete();
        let school = fixture::school();
        let mut target = fixture::target(row.locator(), &subject, &athlete, &school);
        check!(
            project_checked_html_result_row(&document, document.selected_count(), &target).is_err()
        );
        check!(project_checked_html_result_identity(
            &document,
            document.selected_count(),
            &athlete
        )
        .is_err());
        target.locator.ordinal += 1;
        check!(project_checked_html_result_row(&document, 0, &target).is_err());
        target.locator = row.locator();
        let forged = "0".repeat(64);
        target.subject_reference = &forged;
        check!(project_checked_html_result_row(&document, 0, &target).is_err());
    }
    Ok(())
}
