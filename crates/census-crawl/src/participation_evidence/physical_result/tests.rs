use super::super::result_ownership::read_retained_html_result_rows;
use super::*;
use crate::milesplit::SourceRowLocator;
use census_domain::model::{
    AthleteId, EventKind, ExactSeconds, Gender, Mark, SchoolId, TimingMethod,
};

mod error;
mod fixture;
mod identity;
mod refusals;
mod replay;
mod selected_row;

type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn genuine_original_rows_project_with_truthful_content_keys_and_unadmitted_targets() -> TestResult {
    let capture = fixture::capture()?;
    check!(eq; capture.body_sha256(), "95350f1389f9f81fc0354d73637a59cf508413f8bea530a91eb0de7c28cc19e5");
    let document = read_retained_html_result_rows(&capture, &fixture::locators())?;
    let subjects = fixture::subjects(&document)?;
    let athlete = fixture::athlete();
    let school = fixture::school();
    let targets = fixture::targets(&subjects, &athlete, &school);
    let output = project_retained_html_results(&document, &targets)?;
    check!(eq; output.rows().len(), 3);
    check!(std::ptr::eq(output.document(), &document));
    check!(eq; output.document().original_document().as_bytes(), fixture::BODY);
    check!(eq; output.proposed_meet().date, "2026-05-15");
    check!(eq; output.proposed_meet().source_identities.len(), 1);
    check!(eq; output.proposed_meet().source_identities.first().ok_or("meet owner")?.id, "763888");
    let expected = [
        (
            "13.57",
            Some(0.2),
            EventKind::Track110mHurdles,
            "947d8be3d1cc9625154b5ebf345c4e4a578dee9640a4852ea26ccbad80a34849",
        ),
        (
            "14.10",
            None,
            EventKind::Track110mHurdles,
            "fb41f4eb4d8c25c466ee3e36a50d45b0c45b575b8bbfe927d5d4ad858d6db8ee",
        ),
        (
            "36.23",
            None,
            EventKind::Track300mHurdles,
            "7fc4b5a2210606f6b926eb3636aeeb68648c294d51b106aaa3a9242e6a55ba70",
        ),
    ];
    for (index, (row, (mark, wind, kind, digest))) in output.rows().iter().zip(expected).enumerate()
    {
        let performance = row.proposed_performance();
        check!(eq; performance.athlete, athlete);
        check!(eq; performance.mark, Mark::TimeSeconds(ExactSeconds::parse(mark)?));
        check!(eq; performance.wind_mps, wind);
        check!(eq; performance.timing, Some(TimingMethod::Fat));
        check!(eq; performance.observed_grade.map(|grade| grade.get()), Some(12));
        check!(eq; performance.place, Some(1));
        check!(eq; performance.source_athlete, None);
        check!(eq; performance.source_key, format!("milesplit_html_row_v1:{digest}"));
        check!(eq; row.locator(), *fixture::locators().get(index).ok_or("locator")?);
        check!(eq; row.proposed_team().school, school);
        check!(row.proposed_team().source_identities.is_empty());
        check!(eq; row.proposed_event().gender, Gender::Boys);
        check!(eq; row.proposed_event().kind, kind);
        check!(row.proposed_event().specification.hurdles.is_none());
        check!(row.proposed_event().source_bound_kind(&kind));
        let note = performance
            .evidence
            .first()
            .and_then(|value| value.note.as_deref())
            .ok_or("row note")?;
        check!(note.contains("unadmitted_projection"));
        check!(note.contains(document.context_reference()));
        check!(note.contains(fixture::ATTACHMENT));
    }
    Ok(())
}
