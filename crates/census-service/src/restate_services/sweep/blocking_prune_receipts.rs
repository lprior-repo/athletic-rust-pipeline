use std::sync::Arc;

use census_store::{Pruned, Store};

use crate::spawn::Spawner;

use super::super::{blocking, JobError};

/// Prune the store's receipts past the replay window through the blocking pool.
pub(super) async fn blocking_prune_receipts(
    spawner: Arc<Spawner>,
    store: Arc<Store>,
    before: String,
) -> Result<Pruned, JobError> {
    blocking(spawner, move || store.prune_receipts(&before)).await
}
