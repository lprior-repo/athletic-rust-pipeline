use super::*;
use crate::bests::SharedSelection;
use crate::export::ExportDataset;
use census_domain::model::{AthleteId, CanonicalPerformance, SchoolId};
use std::path::Path;
mod artifacts;
mod normalized;
mod selection;
mod workbook;

struct Expected<'a> {
    public: &'a PublicAthleteId,
    school: &'a SchoolId,
    performance: &'a CanonicalPerformance,
    selection: &'a SharedSelection,
}

pub(super) fn verify(
    dataset: &ExportDataset,
    workbook: &Path,
    public: &PublicAthleteId,
    target: &AthleteId,
    school: &SchoolId,
) -> TestResult {
    let (performance, selection) = selection::observe(dataset, public, school)?;
    check!(eq; &performance.athlete,target);
    let expected = Expected {
        public,
        school,
        performance: &performance,
        selection: &selection,
    };
    let directory = workbook.parent().ok_or("published generation directory")?;
    normalized::observe(dataset, directory, &expected)?;
    workbook::observe(workbook, &expected)
}
