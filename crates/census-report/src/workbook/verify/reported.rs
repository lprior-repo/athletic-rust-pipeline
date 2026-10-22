use super::expectations::Expectations;
use super::read::Book;
use super::report::Findings;
use crate::report::{ReportError, ReportResult};
use crate::workbook::reported::Projection;

mod athletes;
mod prs;
mod rows;

pub(super) fn verify(
    book: &mut Book,
    expectations: &Expectations<'_>,
    findings: &mut Findings,
) -> ReportResult<()> {
    let projection = Projection::of(
        expectations.dataset,
        &expectations.derivation,
        &expectations.admission,
        &expectations.bests,
    )?
    .ok_or_else(|| invalid("revision 9 verification lacks Include source projection"))?;
    athletes::verify(book, expectations, &projection, findings)?;
    prs::verify(book, expectations, &projection, findings)
}

fn invalid(detail: &str) -> ReportError {
    ReportError::Invariant {
        detail: detail.into(),
    }
}
