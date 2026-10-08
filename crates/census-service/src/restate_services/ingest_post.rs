use chrono::Datelike;
use restate_sdk::prelude::*;

use census_crawl::Recorded;
use census_domain::UsJurisdiction;

use super::ingest::IngestClient;
use super::wire::ingest::{RecordedIngestRequest, WindowRequest};

pub(super) fn endpoint_of(slug: &str, jurisdiction: UsJurisdiction) -> String {
    format!("{slug}_{}", jurisdiction.code().to_ascii_lowercase())
}

pub(super) fn window_of(at: &str) -> Result<String, HandlerError> {
    let day = chrono::NaiveDate::parse_from_str(at, "%Y-%m-%d").map_err(|source| {
        TerminalError::new(format!("run date {at} is not a calendar day: {source}"))
    })?;
    let week = day.iso_week();
    Ok(format!("{}-W{:02}", week.year(), week.week()))
}

#[tracing::instrument(skip_all)]
pub(super) async fn post(
    ctx: &ObjectContext<'_>,
    endpoint: &str,
    window: &str,
    recorded: Recorded,
) -> Result<u64, HandlerError> {
    let object = ctx.object_client::<IngestClient>(endpoint);
    let operation_id = operation_of(endpoint, window)?;
    let Json(reply) = object
        .recorded(Json(RecordedIngestRequest {
            recorded,
            operation_id,
            cursor: None,
        }))
        .call()
        .await?;
    object
        .complete_window(Json(WindowRequest {
            window: window.to_string(),
        }))
        .call()
        .await?;
    Ok(reply.appended)
}

fn operation_of(endpoint: &str, window: &str) -> Result<String, HandlerError> {
    let length = endpoint
        .len()
        .checked_add(window.len())
        .and_then(|length| length.checked_add(1))
        .ok_or_else(|| TerminalError::new("source operation length overflow"))?;
    let max = census_store::MAX_OPERATION_BYTES
        .checked_sub(65)
        .ok_or_else(|| TerminalError::new("source operation digest capacity"))?;
    if length > max {
        return Err(TerminalError::new("source operation prefix capacity exceeded").into());
    }
    let mut operation = String::new();
    operation
        .try_reserve_exact(length)
        .map_err(|_| TerminalError::new("allocating source operation identity"))?;
    operation.push_str(endpoint);
    operation.push(':');
    operation.push_str(window);
    Ok(operation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::restate_services::tests::sdk_error;

    #[test]
    fn a_window_is_the_iso_week_of_the_run_day() -> Result<(), Box<dyn std::error::Error>> {
        for (day, expected) in [
            ("2026-09-23", "2026-W39"),
            ("2027-01-01", "2026-W53"),
            ("2026-01-01", "2026-W01"),
        ] {
            let window = window_of(day).map_err(sdk_error)?;
            if window != expected {
                return Err(format!("{day} ISO week: left={window:?}, right={expected:?}").into());
            }
        }
        Ok(())
    }

    #[test]
    fn a_run_day_that_is_not_a_calendar_day_refuses() {
        assert!(window_of("2026-09-31").is_err());
        assert!(window_of("yesterday").is_err());
    }
}
