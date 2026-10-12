use super::super::{
    capture_tests::{meet, scope},
    selection,
};
use super::*;
use crate::restate_services::{
    jurisdiction,
    wire::{JurisdictionRequest, JurisdictionState},
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn advancing_evidence_returns_transient_without_recapture_and_keeps_results_owed() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Arc::new(Store::open(dir.path())?);
    let jurisdiction = UsJurisdiction::Wisconsin;
    store.append(
        Table::SourceMeets,
        &meet(jurisdiction, "1", Some("2024-04-01")),
    )?;
    let mut cache = Cache::default();
    let (held, first) = cache.select(&store, scope(jurisdiction)?)?;
    check!(eq; first.capture.decoded_rows, 1);
    let request = JurisdictionRequest {
        jurisdiction,
        season: census_domain::model::SchoolYear::new(2026).ok_or("school year")?,
        revision: census_reconcile::identity::Revision(1),
        history: scope(jurisdiction)?.window,
        refresh: false,
        limit_per_state: None,
        concurrency: 1,
        observed_on: None,
        authorized_hosts: Vec::new(),
        source_parallelism: 1,
    };
    let state = JurisdictionState::default();
    let expected_message =
        "budget exceeded: results live captures would require 2 but only 1 is available";
    for index in 2..5 {
        store.append(
            Table::SourceMeets,
            &meet(jurisdiction, &index.to_string(), None),
        )?;
        let started = std::time::Instant::now();
        let result = cache.select(&store, scope(jurisdiction)?);
        let error = match result {
            Err(error) => selection::selection_error(error),
            Ok(_) => return Err("an advancing generation served a stale capture".into()),
        };
        check!(
            matches!(error, crate::restate_services::JobError::Transient { message } if message == expected_message)
        );
        check!(eq; cache.capture_attempts, 1);
        check!(jurisdiction::stage_owed(
            jurisdiction::Stage::Results,
            &state,
            &request
        ));
        let (owed, reasons) = jurisdiction::owed_work(&state, &request);
        check!(eq; owed.iter().filter(|stage| stage.as_str() == "results").count(), 1);
        check!(reasons
            .iter()
            .any(|reason| reason == "results: 2023:absent 2024:absent 2025:absent 2026:absent"));
        println!("generation advance={index} transient={expected_message}; capture_attempts={} new_capture_attempts=0 owed=results elapsed={:?}", cache.capture_attempts, started.elapsed());
    }
    drop(held);
    let (latest, measured) = cache.select(&store, scope(jurisdiction)?)?;
    check!(eq; measured.capture_attempts, 2);
    check!(eq; measured.capture.decoded_rows, 4);
    check!(eq; latest.pending.len(), 3);
    Ok(())
}
