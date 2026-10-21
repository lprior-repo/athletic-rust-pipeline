use super::{CheckedRetainedTfrrsListV1, CheckedRetainedTfrrsRowV1};
use crate::participation_evidence::result_ownership::{Result, RetainedResultOwnershipError};
use crate::participation_evidence::RetainedHtmlCaptureV1;
use census_domain::model::{Gender, ProfileDate, SchoolYear, Sport};

pub struct CheckedTfrrsRowCalendarV1<'a> {
    capture: &'a RetainedHtmlCaptureV1,
    list_context_reference: &'a str,
    row: CheckedRetainedTfrrsRowV1<'a>,
    day: ProfileDate,
    year: SchoolYear,
    sport: Sport,
    category: Gender,
}

impl<'a> CheckedTfrrsRowCalendarV1<'a> {
    pub fn capture(&self) -> &RetainedHtmlCaptureV1 {
        self.capture
    }
    pub fn list_context_reference(&self) -> &str {
        self.list_context_reference
    }
    pub fn row(&self) -> &CheckedRetainedTfrrsRowV1<'a> {
        &self.row
    }
    pub fn day(&self) -> &ProfileDate {
        &self.day
    }
    pub fn school_year(&self) -> SchoolYear {
        self.year
    }
    pub fn sport(&self) -> Sport {
        self.sport
    }
    pub fn category(&self) -> Gender {
        self.category
    }
}

pub fn read_checked_tfrrs_row_calendar<'a>(
    list: &'a CheckedRetainedTfrrsListV1<'a>,
    index: usize,
) -> Result<CheckedTfrrsRowCalendarV1<'a>> {
    let row = list
        .row(index)
        .ok_or(RetainedResultOwnershipError::Invalid(
            "TFRRS calendar selected row absent",
        ))?;
    if row.year() != list.year() {
        return Err(RetainedResultOwnershipError::Invalid(
            "TFRRS calendar row and list year disagree",
        ));
    }
    let year = SchoolYear::from_date(row.published_date()).ok_or(
        RetainedResultOwnershipError::Invalid("TFRRS calendar row school year unsupported"),
    )?;
    let day = ProfileDate::parse(row.published_date()).map_err(|_| {
        RetainedResultOwnershipError::Invalid("TFRRS calendar row exact day unsupported")
    })?;
    let sport = list.sport();
    if !matches!(sport, Sport::IndoorTrack | Sport::OutdoorTrack) {
        return Err(RetainedResultOwnershipError::Invalid(
            "TFRRS calendar source sport unsupported",
        ));
    }
    let category = row.gender();
    if !matches!(category, Gender::Boys | Gender::Girls) {
        return Err(RetainedResultOwnershipError::Invalid(
            "TFRRS calendar source category unsupported",
        ));
    }
    Ok(CheckedTfrrsRowCalendarV1 {
        capture: list.capture(),
        list_context_reference: list.context_reference(),
        row,
        day,
        year,
        sport,
        category,
    })
}
