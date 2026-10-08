use super::*;
#[test]
fn unsupported_settled_completion_is_rejected_by_admission_and_inspection() -> TestResult {
    let root = tempfile::tempdir()?;
    let request = request()?;
    let store = Store::open(root.path())?;
    reserve(&store, &request)?;
    let registered = ledger::register(&store, &key(&request), &request)?;
    let mut batch = store.write_batch();
    batch.journal_done(
        ledger::PHASE,
        &format!("{}/settled", key(&request)),
        &TeamsSourceOutcome::Completed {
            outcome: StageOutcome {
                records: 37,
                at: request.observed_on.clone(),
                errors: Vec::new(),
                notes: Vec::new(),
                disposition: census_crawl::CollectionDisposition::Complete,
                unfinished: Vec::new(),
            },
            progress: Vec::new(),
        },
    )?;
    batch.commit()?;
    check!(matches!(
        ledger::begin(&store, &key(&request), &request, &registered),
        Err(JobError::Terminal { .. })
    ));
    check!(matches!(
        ledger::status(&store, &key(&request), Some(&request)),
        Err(JobError::Terminal { .. })
    ));
    Ok(())
}

#[test]
fn source_completion_refuses_error_bearing_or_changed_date_outcomes() -> TestResult {
    for (at, errors) in [
        (
            "2026-10-02",
            vec!["partial source remains unfinished".to_string()],
        ),
        ("2026-10-03", Vec::new()),
    ] {
        let root = tempfile::tempdir()?;
        let request = request()?;
        let store = Store::open(root.path())?;
        let (attempt, _) = reserve(&store, &request)?;
        check!(matches!(
            ledger::finish(
                &store,
                &key(&request),
                attempt,
                Ok(StageOutcome {
                    records: 37,
                    at: at.to_string(),
                    errors,
                    notes: Vec::new(),
                    disposition: census_crawl::CollectionDisposition::Complete,
                    unfinished: Vec::new(),
                })
                .into()
            ),
            Err(JobError::Terminal { .. })
        ));
        check!(store
            .journal_payload(ledger::PHASE, &format!("{}/settled", key(&request)))?
            .is_none());
    }
    Ok(())
}
