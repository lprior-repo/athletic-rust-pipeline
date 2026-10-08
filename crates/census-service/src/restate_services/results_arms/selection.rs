use super::super::{history_stage::HistoricalStageScope, job_error, jobs};
use census_domain::model::SourceMeetRef;
use census_store::{Store, StoreError, Table};
use chrono::Datelike;
use restate_sdk::prelude::HandlerError;

pub(super) struct Selected {
    pub meets: Vec<SourceMeetRef>,
    pub pending: Vec<String>,
}

pub(super) fn select(store: &Store, scope: HistoricalStageScope) -> Result<Selected, HandlerError> {
    let mut selected = Selected {
        meets: Vec::new(),
        pending: Vec::new(),
    };
    store
        .snapshot()
        .for_each_merged(Table::SourceMeets, |meet: SourceMeetRef| {
            if meet.jurisdiction != scope.jurisdiction {
                return Ok(());
            }
            match assess(&meet, scope) {
                Admission::Admitted => append(&mut selected.meets, meet)?,
                Admission::Unknown => append(&mut selected.pending, meet.results_url)?,
                Admission::Excluded => (),
            }
            Ok(())
        })
        .map_err(|error| job_error(error.into()))?;
    selected.pending.sort();
    selected.pending.dedup();
    Ok(selected)
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Admission {
    Admitted,
    Unknown,
    Excluded,
}

pub(super) fn assess(meet: &SourceMeetRef, scope: HistoricalStageScope) -> Admission {
    let date = meet
        .date
        .as_deref()
        .and_then(|raw| chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d").ok());
    match date {
        Some(date)
            if date.year() == i32::from(scope.year)
                && scope.window.admits_date(&date.to_string()) =>
        {
            Admission::Admitted
        }
        Some(_) => Admission::Excluded,
        None if meet.year == scope.year => Admission::Unknown,
        None => Admission::Excluded,
    }
}

pub(super) fn academic_year(
    meet: &SourceMeetRef,
) -> Result<census_domain::model::SchoolYear, HandlerError> {
    let date = meet
        .date
        .as_deref()
        .ok_or_else(|| jobs::invariant("result date remains unknown"))?;
    let date = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| jobs::invariant("result date remains malformed"))?;
    let year = if date.month() < 8 {
        date.year().checked_sub(1)
    } else {
        Some(date.year())
    }
    .and_then(|year| u16::try_from(year).ok())
    .ok_or_else(|| jobs::invariant("result academic year overflow"))?;
    super::super::meets_arms::season_of(year)
}

fn append<T>(rows: &mut Vec<T>, value: T) -> Result<(), StoreError> {
    if rows.len() >= 65536 {
        return Err(StoreError::Invariant {
            detail: "historical selection exceeds 65536 locators".to_string(),
        });
    }
    rows.try_reserve(1).map_err(|_| StoreError::Invariant {
        detail: "historical selection allocation failed".to_string(),
    })?;
    rows.push(value);
    Ok(())
}
