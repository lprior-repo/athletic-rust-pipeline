use super::{
    codec, header, header_payload_limit, invalid, Header, Store, StoreError, StoreIdentity,
    StoreResult, HEADER, HEADER_MARGIN, POLICY, PREFIX,
};
use fjall::{PersistMode, Readable};

pub(super) fn enroll_empty(store: &Store, identity: &StoreIdentity) -> StoreResult<()> {
    let snapshot = store.db.snapshot();
    if snapshot.prefix(&store.journal, PREFIX).next().is_some()
        || snapshot.prefix(&store.journal, b"j2\0").next().is_some()
    {
        return Err(invalid());
    }
    let header = Header {
        version: 2,
        policy: POLICY.into(),
        store: identity.clone(),
        bytes: HEADER_MARGIN,
        roots: 0,
        reservations: 0,
    };
    let encoded = codec::encode(&header, header_payload_limit()?)?;
    let mut batch = store.db.batch();
    batch.insert(&store.journal, HEADER.as_bytes(), encoded);
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })
}

impl crate::PublicIdentityWriterV1 {
    pub fn enroll_graph_admission_v1(
        &self,
        store: &Store,
        expected: crate::StoreRevision<'_>,
    ) -> StoreResult<crate::PublicIdentityInputRevisionTokenV2> {
        {
            let _guard = store.lock_appends();
            expected.ensure_current(store, HEADER)?;
            self.require_current_qualification()?;
            let ledger = self.verified(store)?;
            if codec::load(
                &store.db.snapshot(),
                &store.journal,
                HEADER,
                header_payload_limit()?,
            )?
            .is_some()
            {
                header(store, &self.identity)?;
            } else {
                if ledger.history.projection().head().sequence != 0 {
                    return Err(invalid());
                }
                enroll_empty(store, &self.identity)?;
            }
        }
        self.enroll_public_identity_input_revisions_v2(store, expected)
    }
}
