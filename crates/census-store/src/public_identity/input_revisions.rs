use crate::{PublicIdentityInputRevisionTokenV2, Store, StoreResult, StoreRevision};

impl super::PublicIdentityWriterV1 {
    pub fn enroll_public_identity_input_revisions_v2(
        &self,
        store: &Store,
        expected: StoreRevision<'_>,
    ) -> StoreResult<PublicIdentityInputRevisionTokenV2> {
        let _writes = store.lock_appends();
        self.verified(store)?;
        self.require_current_qualification()?;
        crate::input_revisions::enroll_locked(store, expected)
    }
}
