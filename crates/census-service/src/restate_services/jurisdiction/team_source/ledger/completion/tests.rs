use super::*;
use std::cell::Cell;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

thread_local! { static REFUSE: Cell<bool> = const { Cell::new(false) }; }

pub(super) fn refuse_persistence() -> census_store::StoreResult<()> {
    if REFUSE.with(|refuse| refuse.replace(false)) {
        return Err(census_store::StoreError::Refused {
            detail: "injected completion persistence failure".into(),
        });
    }
    Ok(())
}

fn refused(result: Attempt) -> TestResult<JobError> {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let request = crate::restate_services::jurisdiction::team_source::tests::request()?;
    let key = crate::restate_services::jurisdiction::team_source::key(&request);
    let identity = super::super::register(&store, &key, &request)?;
    let attempt = match super::super::begin(&store, &key, &request, &identity)? {
        super::super::Admission::Reserved { attempt, .. } => attempt,
        super::super::Admission::Settled(_) => return Err("unexpected settlement".into()),
    };
    REFUSE.with(|refuse| refuse.set(true));
    let error = finish(&store, &key, attempt, result, &[])
        .err()
        .ok_or("completion unexpectedly persisted")?;
    check!(eq; REFUSE.with(Cell::get), false);
    let attempts = super::super::durable_attempts(&store)?;
    check!(eq; (attempts.reserved, attempts.recorded, attempts.unknown), (1, 0, 1));
    check!(eq; store.journal_payload(PHASE, &slot_key(&key, attempt, "outcome"))?, None);
    check!(eq; store.journal_payload(PHASE, &format!("{key}/settled"))?, None);
    check!(eq; store.walk_table(census_store::Table::SourceAccess)?.rows, 0);
    Ok(error)
}

#[test]
fn empty_access_persistence_failure_keeps_transient_primary_and_both_causes() -> TestResult {
    let error = refused(Attempt::Failed(JobError::Transient {
        message: "original retryable failure".into(),
    }))?;
    check!(matches!(error, JobError::Transient { message }
        if message.contains("original retryable failure")
            && message.contains("injected completion persistence failure")));
    Ok(())
}

#[test]
fn empty_access_persistence_failure_keeps_terminal_primary_and_both_causes() -> TestResult {
    let error = refused(Attempt::Failed(JobError::Terminal {
        message: "original terminal failure".into(),
    }))?;
    check!(matches!(error, JobError::Terminal { message }
        if message.contains("original terminal failure")
            && message.contains("injected completion persistence failure")));
    Ok(())
}

#[test]
fn empty_access_persistence_failure_keeps_typed_resource_primary_and_both_causes() -> TestResult {
    let error = refused(Attempt::Failed(JobError::Resource {
        source: census_report::report::ExportResourceError::BudgetExceeded {
            what: "primary budget",
            needed: 2,
            available: 1,
        },
        message: "original resource failure".into(),
    }))?;
    check!(matches!(error, JobError::Resource {
        source: census_report::report::ExportResourceError::BudgetExceeded {
            what: "primary budget", needed: 2, available: 1
        }, message
    } if message.contains("original resource failure")
        && message.contains("injected completion persistence failure")));
    Ok(())
}

#[test]
fn completed_empty_access_persistence_failure_keeps_only_the_store_failure() -> TestResult {
    let error = refused(Attempt::Completed(
        crate::restate_services::wire::StageOutcome {
            records: 0,
            at: "2026-10-02".into(),
            errors: Vec::new(),
            notes: Vec::new(),
            disposition: census_crawl::CollectionDisposition::Complete,
            unfinished: Vec::new(),
        },
    ))?;
    let expected = JobError::from(census_store::StoreError::Refused {
        detail: "injected completion persistence failure".into(),
    });
    match (error, expected) {
        (JobError::Terminal { message }, JobError::Terminal { message: expected }) => {
            check!(eq; message, expected);
        }
        _ => return Err("successful attempt changed store failure classification".into()),
    }
    Ok(())
}
