#[cfg(test)]
use super::{
    add, charge, key, require_capacity, validate, ContextCharge, Decision, Reservation,
    MAX_JOURNAL_VALUE_BYTES,
};
use super::{
    codec, header, header_payload_limit, invalid, Header, Store, StoreError, StoreIdentity,
    StoreResult, HEADER, HEADER_MARGIN, POLICY, PREFIX,
};
#[cfg(test)]
use census_domain::public_identity_history::local_adjudication::LocalIdentityAdjudicationRecordV2;
use fjall::{PersistMode, Readable};

#[cfg(test)]
pub(in crate::public_identity) fn reserve(
    store: &Store,
    record: LocalIdentityAdjudicationRecordV2,
    charges: [ContextCharge; 2],
) -> StoreResult<Reservation> {
    reserve_decision(store, Decision::SplitV2(record), charges)
}

#[cfg(test)]
pub(in crate::public_identity) fn reserve_withholding(
    store: &Store,
    record: census_domain::public_identity_history::local_membership_withholding::LocalMembershipWithholdingRecordV1,
    charges: [ContextCharge; 2],
) -> StoreResult<Reservation> {
    reserve_decision(store, Decision::MembershipWithholdingV1(record), charges)
}

#[cfg(test)]
fn reserve_decision(
    store: &Store,
    decision: Decision,
    charges: [ContextCharge; 2],
) -> StoreResult<Reservation> {
    let binding = decision.binding();
    let mut header = header(store, binding.store)?;

    let key = key(binding.operation);
    if codec::load(
        &store.db.snapshot(),
        &store.journal,
        &key,
        MAX_JOURNAL_VALUE_BYTES,
    )?
    .is_some()
    {
        return Err(invalid());
    }
    let reservation = Reservation {
        version: 2,
        decision,
        charges,
    };
    validate(&reservation, &header.store)?;
    let raw = codec::encode(&reservation, MAX_JOURNAL_VALUE_BYTES)?;
    header.bytes = add(header.bytes, charge(&reservation, key.len(), raw.len())?)?;
    let [authority, affected] = &reservation.charges;
    header.roots = add(header.roots, add(authority.roots, affected.roots)?)?;
    header.reservations = add(header.reservations, 1)?;
    require_capacity(header.bytes, header.roots, header.reservations)?;
    let encoded = codec::encode(&header, header_payload_limit()?)?;
    let mut batch = store.db.batch();
    batch.insert(&store.journal, key.as_bytes(), raw);
    batch.insert(&store.journal, HEADER.as_bytes(), encoded);
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })?;
    Ok(reservation)
}

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
