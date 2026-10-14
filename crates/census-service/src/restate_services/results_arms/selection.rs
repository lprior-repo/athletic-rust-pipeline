use super::super::{history_stage::HistoricalStageScope, job_error, jobs};
use super::{budget, capture};
use census_domain::model::SourceMeetRef;
use census_store::{Store, StoreError, StoreResult};
use chrono::Datelike;
use restate_sdk::prelude::HandlerError;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

pub(super) struct Selected {
    pub meets: Vec<SourceMeetRef>,
    pub pending: Vec<String>,
    _capture: Arc<capture::Dataset>,
    _permit: budget::Permit,
}

#[tracing::instrument(skip_all)]
pub(super) async fn select(
    store: &Arc<Store>,
    scope: HistoricalStageScope,
) -> Result<Selected, HandlerError> {
    static WORKER: LazyLock<Arc<tokio::sync::Semaphore>> =
        LazyLock::new(|| Arc::new(tokio::sync::Semaphore::new(1)));
    let permit = Arc::clone(&WORKER)
        .acquire_owned()
        .await
        .map_err(|_| jobs::invariant("result capture worker closed"))?;
    let cancel = Arc::new(AtomicBool::new(false));
    let _cancel_on_drop = CancelOnDrop(Arc::clone(&cancel));
    let store = Arc::clone(store);
    let (selected, measurement) = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        select_captured(&store, scope, &cancel)
    })
    .await
    .map_err(|error| jobs::invariant(&format!("result capture worker failed: {error}")))?
    .map_err(|error| job_error(selection_error(error)))?;
    tracing::debug!(
        key_visits = measurement.capture.key_visits,
        decoded_rows = measurement.capture.decoded_rows,
        encoded_bytes = measurement.capture.encoded_bytes,
        partition_visits = measurement.partition_visits,
        retained_bytes = measurement.retained_bytes,
        spill_decoded_rows = measurement.spill_decoded_rows,
        spill_read_bytes = measurement.spill_read_bytes,
        spill_read_calls = measurement.spill_read_calls,
        spill_disk_bytes = measurement.spill_disk_bytes,
        spill_write_calls = measurement.spill_write_calls,
        spill_written_bytes = measurement.spill_written_bytes,
        spill_peak_scratch_bytes = measurement.spill_peak_scratch_bytes,
        capture_attempts = measurement.capture_attempts,
        "results selection"
    );
    Ok(selected)
}

struct CancelOnDrop(Arc<AtomicBool>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

fn select_captured(
    store: &Arc<Store>,
    scope: HistoricalStageScope,
    cancel: &AtomicBool,
) -> StoreResult<(Selected, capture::Measurement)> {
    static CACHE: LazyLock<Mutex<capture::Cache>> =
        LazyLock::new(|| Mutex::new(capture::Cache::default()));
    let mut cache = CACHE.lock().map_err(|_| StoreError::Refused {
        detail: "result capture lock poisoned".to_string(),
    })?;
    cache.select(store, scope, cancel)
}

pub(super) fn selection_error(error: StoreError) -> super::super::JobError {
    match error {
        StoreError::BudgetExceeded {
            what: "results live captures",
            ..
        } => super::super::JobError::Transient {
            message: error.to_string(),
        },
        other => other.into(),
    }
}

pub(super) fn partition(
    scope: HistoricalStageScope,
    admission: Arc<budget::Admission>,
    dataset: Arc<capture::Dataset>,
    cancel: &AtomicBool,
) -> StoreResult<(Selected, capture::Visits)> {
    let mut permit = budget::Permit::new(admission);
    permit.reserve(0, budget::slots::<Selected>(1)?)?;
    let mut meets = Vec::new();
    let mut pending = Vec::new();
    let visits = dataset.visit(scope.jurisdiction, cancel, |row, charge| {
        capture::check_cancel(cancel)?;
        match assess(row, scope) {
            Admission::Admitted => append(&mut meets, charge.meet_heap, &mut permit, || {
                budget::copy_meet(row)
            })?,
            Admission::Unknown => append(&mut pending, charge.pending_heap, &mut permit, || {
                budget::copy(&row.results_url)
            })?,
            Admission::Excluded => (),
        }
        Ok(())
    })?;
    pending.sort();
    pending.dedup();
    Ok((
        Selected {
            meets,
            pending,
            _capture: dataset,
            _permit: permit,
        },
        visits,
    ))
}

fn append<T>(
    rows: &mut Vec<T>,
    heap_bytes: u64,
    permit: &mut budget::Permit,
    build: impl FnOnce() -> StoreResult<T>,
) -> StoreResult<()> {
    let growth = if rows.len() == rows.capacity() {
        budget::VECTOR_CHUNK
    } else {
        0
    };
    let bytes = heap_bytes
        .checked_add(budget::slots::<T>(growth)?)
        .ok_or(StoreError::CounterOverflow)?;
    permit.reserve(1, bytes)?;
    let old_capacity = rows.capacity();
    if growth != 0 {
        rows.try_reserve_exact(growth)
            .map_err(|_| budget::allocation())?;
    }
    let expected = old_capacity
        .checked_add(growth)
        .ok_or(StoreError::CounterOverflow)?;
    if rows.capacity() > expected {
        let extra = rows
            .capacity()
            .checked_sub(expected)
            .ok_or(StoreError::CounterOverflow)?;
        permit.reserve(0, budget::slots::<T>(extra)?)?;
    }
    rows.push(build()?);
    Ok(())
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
