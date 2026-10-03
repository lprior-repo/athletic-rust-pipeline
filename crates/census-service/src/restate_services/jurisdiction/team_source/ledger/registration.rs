use crate::restate_services::support::JobError;
use crate::restate_services::wire::TeamsSourceRequest;
use census_store::Store;

pub(in crate::restate_services::jurisdiction::team_source) fn register(
    store: &Store,
    key: &str,
    request: &TeamsSourceRequest,
) -> Result<super::Identity, JobError> {
    let snapshot = store.snapshot();
    let (identity, initial) = super::identity(&snapshot, key, request)?;
    let history = super::history::read(&snapshot, key, &identity)?;
    if initial && history.attempt != 0 {
        return Err(super::terminal(
            "teams source attempt history has no identity",
        ));
    }
    super::validated_settled(&snapshot, key, &history, false)?;
    if initial {
        let mut batch = store.write_batch();
        batch.journal_done(super::PHASE, &format!("{key}/identity"), &identity)?;
        batch.commit()?;
    }
    store.flush()?;
    Ok(identity)
}
