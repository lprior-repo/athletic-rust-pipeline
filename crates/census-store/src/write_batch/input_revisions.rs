use fjall::OwnedWriteBatch;

use super::{Page, Replacement};
use crate::{Store, StoreResult};

pub(super) fn stage(
    pages: &[Page],
    replacements: &[Replacement],
    store: &Store,
    batch: &mut OwnedWriteBatch,
) -> StoreResult<()> {
    let appended = pages
        .iter()
        .filter(|page| !page.records.is_empty())
        .map(|page| page.table);
    let replaced = replacements
        .iter()
        .filter(|replacement| !replacement.records.is_empty())
        .map(|replacement| replacement.table);
    crate::input_revisions::stage(store, batch, appended.chain(replaced))
}
