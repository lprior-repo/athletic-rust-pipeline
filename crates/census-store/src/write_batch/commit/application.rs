use super::{
    count_rows, receipt, Application, OwnedWriteBatch, Page, Store, StoreError, StoreResult,
};
use chrono::{DateTime, Utc};

pub(super) fn stage(
    store: &Store,
    once: Option<(&str, &str)>,
    pages: &[Page],
    batch: &mut OwnedWriteBatch,
    at: Option<DateTime<Utc>>,
) -> StoreResult<Option<Application>> {
    once.map(|identity| first_application_at(store, identity, count_rows(pages)?, batch, at))
        .transpose()
}

pub(in crate::write_batch) fn first_application(
    store: &Store,
    identity: (&str, &str),
    appended: u64,
    batch: &mut OwnedWriteBatch,
) -> StoreResult<Application> {
    first_application_at(store, identity, appended, batch, None)
}

pub(super) fn first_application_at(
    store: &Store,
    identity: (&str, &str),
    appended: u64,
    batch: &mut OwnedWriteBatch,
    at: Option<DateTime<Utc>>,
) -> StoreResult<Application> {
    let receipt = match at {
        Some(at) => receipt::first_at(identity.0, identity.1, appended, at),
        None => receipt::first(identity.0, identity.1, appended),
    };
    let value = serde_json::to_vec(&receipt).map_err(|source| StoreError::Json {
        detail: "serializing a receipt".to_string(),
        source,
    })?;
    batch.insert(&store.receipts, identity.0.as_bytes(), value);
    Ok(Application::Written(receipt))
}

#[cfg(test)]
mod tests;
