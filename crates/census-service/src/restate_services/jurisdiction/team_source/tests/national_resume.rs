use super::*;

type ResultOf<T> = Result<T, Box<dyn Error>>;

pub(crate) fn exhaust(
    store: &Store,
    operation: &str,
    request: &TeamsSourceRequest,
) -> ResultOf<()> {
    for expected in 1..=3 {
        let registered = ledger::register(store, operation, request)?;
        let ledger::AdmissionReplay::Reserved { attempt, .. } = ledger::begin_at(
            store,
            operation,
            request,
            &registered,
            ledger::baseline(store, operation)?,
        )?
        else {
            return Err("source exhausted before three recorded attempts".into());
        };
        check!(eq; attempt, expected);
        ledger::finish(
            store,
            operation,
            attempt,
            ledger::Attempt::Failed(JobError::Transient {
                message: format!("retained acquisition refusal {attempt}"),
            }),
            &[],
        )?;
    }
    store.flush()?;
    Ok(())
}

pub(crate) fn replay(
    store: &Store,
    operation: &str,
    request: &TeamsSourceRequest,
) -> ResultOf<TeamsSourceOutcome> {
    let before = store.journal_keys(ledger::PHASE)?;
    let registered = ledger::register(store, operation, request)?;
    let ledger::AdmissionReplay::Settled(outcome) = ledger::begin_at(
        store,
        operation,
        request,
        &registered,
        ledger::baseline(store, operation)?,
    )?
    else {
        return Err("resumed national invocation granted a new source attempt".into());
    };
    check!(
        matches!(&outcome, TeamsSourceOutcome::Exhausted { attempts: 3, last_failure, .. } if last_failure == "retained acquisition refusal 3")
    );
    check!(eq; store.journal_keys(ledger::PHASE)?, before);
    check!(!store.journal_contains(ledger::PHASE, &format!("{operation}/attempt/4/reserved"))?);
    Ok(outcome)
}
