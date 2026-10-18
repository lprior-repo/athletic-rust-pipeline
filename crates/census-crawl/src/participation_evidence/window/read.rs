use super::{
    hash, invalid, markup, statement, status, DocumentSubjectV1, IndependentParticipationProofV1,
    RetainedHtmlCaptureV1, SubjectOwnershipV1,
};
use census_domain::model::{GradYear, SchoolYear};
use census_store::StoreResult;

pub(super) struct ParsedWindow {
    pub(super) subject: DocumentSubjectV1,
    pub(super) ownership: SubjectOwnershipV1,
    pub(super) boundary: statement::ParsedBoundary,
    pub(super) proof: IndependentParticipationProofV1,
}

const INTERPRETATION: &str = "mhsaa_subject_dcc_full_participation_window_v1";

pub(super) fn read(capture: &RetainedHtmlCaptureV1) -> StoreResult<ParsedWindow> {
    let body = capture.body()?;
    let (status, article) = status::read_context(capture, &body)?;
    let paragraph = markup::single(markup::elements(
        article.html,
        b"p",
        Some("participation-window"),
    )?)?;
    let boundary = statement::parse(&markup::text(paragraph.html)?)?;
    let expected_year = SchoolYear::from_date(status.day.as_str())
        .ok_or_else(|| invalid("status day has no canonical school year"))?;
    if boundary.school_year != expected_year {
        return Err(invalid(
            "declared window school year differs from status day",
        ));
    }
    let expected_cohort = GradYear::of(status.grade, expected_year)
        .ok_or_else(|| invalid("source status grade and year unsupported"))?;
    if boundary.cohort != expected_cohort {
        return Err(invalid("declared window cohort differs from status grade"));
    }
    let start = article
        .span
        .start
        .checked_add(paragraph.span.start)
        .ok_or(census_store::StoreError::CounterOverflow)?;
    let end = article
        .span
        .start
        .checked_add(paragraph.span.end)
        .ok_or(census_store::StoreError::CounterOverflow)?;
    if end > body.len() {
        return Err(invalid("declared window paragraph exceeds original body"));
    }
    let locator = format!("html_bytes:{start}:{end}");
    let proof = IndependentParticipationProofV1 {
        capture: capture.body_sha256().into(),
        interpretation: hash(
            format!("{INTERPRETATION}\n{}\n{locator}", capture.body_sha256()).as_bytes(),
        ),
        locator,
        acquired_at: capture.acquired_at(),
    };
    Ok(ParsedWindow {
        subject: status.subject,
        ownership: status.ownership,
        boundary,
        proof,
    })
}
