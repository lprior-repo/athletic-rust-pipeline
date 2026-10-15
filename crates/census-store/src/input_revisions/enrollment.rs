use fjall::PersistMode;

use super::{codec, PublicIdentityInputRevisionTokenV2, PublicIdentityInputRevisionsV2, KEY};
use crate::{Store, StoreError, StoreResult, StoreRevision};

impl Store {
    pub fn enroll_public_identity_input_revisions_v2(
        &self,
        expected: StoreRevision<'_>,
    ) -> StoreResult<PublicIdentityInputRevisionTokenV2> {
        let _writes = self.lock_appends();
        expected.ensure_current(self, KEY)?;
        let format = crate::format::require_semantic_format(
            self.root(),
            &self.db,
            &self.meta,
            &self.journal,
            &self.receipts,
        )?;
        let state = crate::installation::validate_format(
            &self.db.snapshot(),
            &self.meta,
            &self.journal,
            format.lifecycle_revision,
        )?;
        crate::installation::require_legacy_execution(&state)?;
        enroll_locked(self, expected)
    }
}

pub(crate) fn enroll_locked(
    store: &Store,
    expected: StoreRevision<'_>,
) -> StoreResult<PublicIdentityInputRevisionTokenV2> {
    expected.ensure_current(store, KEY)?;
    match super::read(&store.db.snapshot(), &store.meta)? {
        PublicIdentityInputRevisionsV2::Enabled(token) => return Ok(token),
        PublicIdentityInputRevisionsV2::Disabled => {}
    }
    let token = PublicIdentityInputRevisionTokenV2 {
        baseline_evidence_generation: expected.evidence_generation(),
        baseline_derived_generation: expected.derived_generation(),
        counters: [0; 6],
    };
    let mut batch = store.db.batch();
    codec::write(&mut batch, &store.meta, token)?;
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })?;
    Ok(token)
}
