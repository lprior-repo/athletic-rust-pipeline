use super::super::canonical::Value;
use super::super::current_school::AffiliationExpectations;
use super::super::expectations::Expectations;
use super::super::labels;
use super::super::public_identity;
use super::super::read::Book;
use super::super::report::Findings;
use crate::report::ReportResult;
use crate::workbook::reported::Projection;
use census_domain::model::performance_comparison::mark_unit;

mod best;
mod review;

pub(super) fn verify(
    book: &mut Book,
    expectations: &Expectations<'_>,
    projection: &Projection<'_>,
    findings: &mut Findings,
) -> ReportResult<()> {
    let schools = AffiliationExpectations::of(expectations.dataset)?;
    let headers = headers(expectations);
    let total = expectations
        .bests
        .len()
        .checked_add(projection.bests.len())
        .and_then(|total| total.checked_add(projection.reviews.len()))
        .ok_or_else(|| super::invalid("mixed PR count overflow"))?;
    super::rows::verify(
        book,
        labels::PRS,
        &headers,
        total,
        |position, _| {
            if let Some(pr) = expectations.bests.get(position) {
                let mut row = vec![
                    Value::text("verified"),
                    Value::text("verified_eligible_best"),
                ];
                row.extend(super::super::prs::values(pr, expectations, Some(&schools))?);
                row.extend([Value::Empty, Value::Empty, Value::Empty]);
                return Ok(row);
            }
            let position = position
                .checked_sub(expectations.bests.len())
                .ok_or_else(|| super::invalid("mixed PR position underflow"))?;
            if let Some(reported) = projection.bests.get(position) {
                return best::values(projection, reported, expectations);
            }
            let position = position
                .checked_sub(projection.bests.len())
                .ok_or_else(|| super::invalid("reported review position underflow"))?;
            let review = projection.reviews.get(position).ok_or_else(|| {
                super::invalid("reported event review absent from frozen projection")
            })?;
            review::values(projection, review, expectations)
        },
        findings,
    )
}

fn headers(expectations: &Expectations<'_>) -> Vec<&'static str> {
    let mut headers = vec!["Record Basis", "Verification Status"];
    headers.extend(
        public_identity::sheet_headers(expectations.dataset, &labels::PR_HEADERS)
            .into_iter()
            .map(|header| match header {
                "School" => "School at performance",
                "State" => "State at performance",
                other => other,
            }),
    );
    headers.extend(["Current school", "Current state", "Current school as-of"]);
    headers.extend([
        "Source Athlete ID",
        "Performance Evidence",
        "Reported Event Interpretation",
    ]);
    headers
}
