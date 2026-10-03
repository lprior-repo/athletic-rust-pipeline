use super::*;

#[test]
fn admission_refuses_cached_digest_mismatch_before_first_attempt() -> TestResult {
    let root = tempfile::tempdir()?;
    let request = request()?;
    let operation = key(&request);
    let store = Store::open(root.path())?;
    let mut registered = ledger::register(&store, &operation, &request)?;
    let before = journal(&store)?;
    let progress_before =
        serde_json::to_value(ledger::inspection(&store, &operation, Some(&request))?)?;
    check!(eq; progress_before,
    json!({"status": "unsettled", "progress": []}));

    registered.request_digest = "0".repeat(64);
    let result = ledger::begin(&store, &operation, &request, &registered);

    check!(matches!(result, Err(JobError::Terminal { .. })));
    check!(eq; journal(&store)?, before);
    check!(eq; serde_json::to_value(ledger::inspection(&store, &operation, Some(&request))?)?,
    progress_before);
    Ok(())
}

#[test]
fn admission_refuses_cached_date_mismatch_before_first_attempt() -> TestResult {
    let root = tempfile::tempdir()?;
    let request = request()?;
    let operation = key(&request);
    let store = Store::open(root.path())?;
    let mut registered = ledger::register(&store, &operation, &request)?;
    let before = journal(&store)?;
    let progress_before =
        serde_json::to_value(ledger::inspection(&store, &operation, Some(&request))?)?;
    check!(eq; progress_before,
    json!({"status": "unsettled", "progress": []}));

    registered.observed_on = "2026-10-03".to_string();
    let result = ledger::begin(&store, &operation, &request, &registered);

    check!(matches!(result, Err(JobError::Terminal { .. })));
    check!(eq; journal(&store)?, before);
    check!(eq; serde_json::to_value(ledger::inspection(&store, &operation, Some(&request))?)?,
    progress_before);
    Ok(())
}

#[test]
fn admission_refuses_cached_digest_mismatch_with_acquisition_history() -> TestResult {
    let root = tempfile::tempdir()?;
    let request = request()?;
    let operation = key(&request);
    let store = Store::open(root.path())?;
    let (attempt, observed_on) = reserve(&store, &request)?;
    check!(eq; (attempt, observed_on), (1, "2026-10-02".to_string()));
    let completion = ledger::finish(
        &store,
        &operation,
        attempt,
        Err(JobError::Transient {
            message: "retained acquisition failure".to_string(),
        })
        .into(),
    )?;
    check!(matches!(
        completion,
        ledger::Completion::Retry(JobError::Transient { .. })
    ));
    let mut registered = ledger::register(&store, &operation, &request)?;
    let before = journal(&store)?;
    let progress_before =
        serde_json::to_value(ledger::inspection(&store, &operation, Some(&request))?)?;
    check!(eq; progress_before,
    json!({
        "status": "unsettled",
        "progress": [{
            "status": "transient",
            "attempt": 1,
            "outcome": null,
            "message": "retained acquisition failure"
        }]
    }));

    registered.request_digest = "0".repeat(64);
    let result = ledger::begin(&store, &operation, &request, &registered);

    check!(matches!(result, Err(JobError::Terminal { .. })));
    check!(eq; journal(&store)?, before);
    check!(eq; serde_json::to_value(ledger::inspection(&store, &operation, Some(&request))?)?,
    progress_before);
    Ok(())
}

#[test]
fn admission_refuses_cached_date_mismatch_with_acquisition_history() -> TestResult {
    let root = tempfile::tempdir()?;
    let request = request()?;
    let operation = key(&request);
    let store = Store::open(root.path())?;
    let (attempt, observed_on) = reserve(&store, &request)?;
    check!(eq; (attempt, observed_on), (1, "2026-10-02".to_string()));
    let completion = ledger::finish(
        &store,
        &operation,
        attempt,
        Err(JobError::Transient {
            message: "retained acquisition failure".to_string(),
        })
        .into(),
    )?;
    check!(matches!(
        completion,
        ledger::Completion::Retry(JobError::Transient { .. })
    ));
    let mut registered = ledger::register(&store, &operation, &request)?;
    let before = journal(&store)?;
    let progress_before =
        serde_json::to_value(ledger::inspection(&store, &operation, Some(&request))?)?;
    check!(eq; progress_before,
    json!({
        "status": "unsettled",
        "progress": [{
            "status": "transient",
            "attempt": 1,
            "outcome": null,
            "message": "retained acquisition failure"
        }]
    }));

    registered.observed_on = "2026-10-03".to_string();
    let result = ledger::begin(&store, &operation, &request, &registered);

    check!(matches!(result, Err(JobError::Terminal { .. })));
    check!(eq; journal(&store)?, before);
    check!(eq; serde_json::to_value(ledger::inspection(&store, &operation, Some(&request))?)?,
    progress_before);
    Ok(())
}
