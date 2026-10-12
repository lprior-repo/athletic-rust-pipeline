use chrono::Datelike;
use restate_sdk::prelude::*;

use census_crawl::Recorded;
use census_domain::UsJurisdiction;

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

#[derive(Debug, PartialEq, Eq)]
pub(super) struct RecordedChunk {
    pub batch: usize,
    pub chunk: census_store::ObservationChunk,
}

pub(super) fn admit_recorded(recorded: &Recorded) -> census_store::StoreResult<Vec<RecordedChunk>> {
    census_store::ObservationBudget::admit(
        recorded.rows.iter().flat_map(|batch| &batch.rows),
        0,
        0,
    )?;
    let work = recorded
        .rows
        .len()
        .checked_add(recorded.journal.len())
        .and_then(|work| work.checked_add(recorded.effects.len()))
        .ok_or(census_store::StoreError::CounterOverflow)?;
    if work > census_crawl::recording::MAX_RECORDED_WORK {
        return Err(census_store::StoreError::BudgetExceeded {
            what: "recorded ingest work",
            needed: u64::try_from(work).map_err(|_| census_store::StoreError::CounterOverflow)?,
            available: u64::try_from(census_crawl::recording::MAX_RECORDED_WORK)
                .map_err(|_| census_store::StoreError::CounterOverflow)?,
        });
    }
    census_store::ObservationBudget::request_bytes(recorded)?;
    let mut manifest = Vec::new();
    for (batch, rows) in recorded.rows.iter().enumerate() {
        for chunk in census_store::ObservationBudget::chunks(&rows.rows)? {
            manifest
                .try_reserve(1)
                .map_err(|_| census_store::StoreError::Refused {
                    detail: "allocating bounded recorded ingest manifest".to_string(),
                })?;
            manifest.push(RecordedChunk { batch, chunk });
        }
    }
    Ok(manifest)
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

    #[test]
    fn recorded_ingest_refuses_an_oversized_record_with_a_typed_error() {
        let recorded = Recorded {
            rows: vec![census_crawl::RecordedBatch {
                table: census_store::Table::Schools,
                rows: vec![serde_json::json!({"id":"s1", "payload":"x".repeat(1_048_576)})],
            }],
            ..Recorded::default()
        };
        assert!(matches!(
            admit_recorded(&recorded),
            Err(census_store::StoreError::RowTooLarge { max: 1_048_576, .. })
        ));
    }

    #[test]
    fn recorded_ingest_refuses_aggregate_bytes_across_tables() {
        let row = serde_json::json!({"id":"s1", "payload":"x".repeat(524_288)});
        let recorded = Recorded {
            rows: vec![
                census_crawl::RecordedBatch {
                    table: census_store::Table::Schools,
                    rows: vec![row.clone(); 8],
                },
                census_crawl::RecordedBatch {
                    table: census_store::Table::Meets,
                    rows: vec![row; 8],
                },
            ],
            ..Recorded::default()
        };
        assert!(matches!(
            admit_recorded(&recorded),
            Err(census_store::StoreError::BudgetExceeded {
                what: "observation batch bytes",
                available: 8_388_608,
                ..
            })
        ));
    }

    #[test]
    fn recorded_manifests_reproduce_bounded_row_references_without_changing_payloads(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let recorded = Recorded {
            rows: vec![
                census_crawl::RecordedBatch {
                    table: census_store::Table::Schools,
                    rows: vec![serde_json::json!({"id":"é\\n"})],
                },
                census_crawl::RecordedBatch {
                    table: census_store::Table::Meets,
                    rows: vec![
                        serde_json::json!({"id":"m1"}),
                        serde_json::json!({"id":"m2"}),
                    ],
                },
            ],
            ..Recorded::default()
        };
        let before = serde_json::to_vec(&recorded)?;
        let manifest = admit_recorded(&recorded)?;
        check!(eq; admit_recorded(&recorded)?, manifest);
        check!(eq; manifest.len(), 2);
        for entry in manifest {
            let batch = recorded.rows.get(entry.batch).ok_or("batch missing")?;
            let rows = batch
                .rows
                .get(entry.chunk.rows.clone())
                .ok_or("rows missing")?;
            let bytes: usize = rows
                .iter()
                .map(serde_json::to_vec)
                .collect::<Result<Vec<_>, _>>()?
                .iter()
                .map(Vec::len)
                .sum();
            check!(eq; bytes, entry.chunk.encoded_bytes);
            check!(eq; entry.chunk.rows, 0..batch.rows.len());
            check!(bytes <= census_store::MAX_OBSERVATION_BATCH_BYTES);
        }
        check!(eq; serde_json::to_vec(&recorded)?, before);
        Ok(())
    }

    #[test]
    fn journal_only_requests_cannot_bypass_the_encoded_request_ceiling() {
        let recorded = Recorded {
            journal: vec![census_crawl::RecordedJournal {
                phase: "phase".to_string(),
                key: "key".to_string(),
                payload: serde_json::Value::String("x".repeat(8_388_608)),
            }],
            ..Recorded::default()
        };
        assert!(matches!(
            admit_recorded(&recorded),
            Err(census_store::StoreError::BudgetExceeded {
                what: "observation request bytes",
                available: 8_388_608,
                ..
            })
        ));
    }
}
