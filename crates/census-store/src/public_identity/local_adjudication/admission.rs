mod decision;
pub(in crate::public_identity) mod graph;
mod graph_wire;
mod persist;
mod retry;
use crate::journal::context::ContextCharge;
use crate::public_identity::{codec, PublicIdentityError};
use crate::{Store, StoreError, StoreResult, MAX_JOURNAL_VALUE_BYTES};
use census_domain::model::StoreIdentity;
pub(in crate::public_identity) use decision::Binding;
use decision::Decision;
use fjall::Readable;
pub(in crate::public_identity) use persist::{reserve, reserve_withholding};
use serde::{Deserialize, Serialize};

const PREFIX: &str = "manual-context-admission-v1\0";
const HEADER: &str = "manual-context-admission-v1\0header";
const RESERVATIONS: &str = "manual-context-admission-v1\0reservation\0";
const POLICY: &str = "manual-context-admission/v2";
const MAX_BYTES: u64 = 128 * 1024 * 1024;
const MAX_ROOTS: u64 = 4096;
const MAX_RESERVATIONS: u64 = 4096;
const HEADER_MARGIN: u64 = 4096;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    version: u8,
    policy: String,
    store: StoreIdentity,
    bytes: u64,
    roots: u64,
    reservations: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::public_identity) struct Reservation {
    version: u8,
    decision: Decision,
    charges: [ContextCharge; 2],
}

fn invalid() -> StoreError {
    PublicIdentityError::Corrupt("manual admission missing, corrupt, or exhausted").into()
}
fn add(left: u64, right: u64) -> StoreResult<u64> {
    left.checked_add(right).ok_or(StoreError::CounterOverflow)
}
fn key(operation: &str) -> String {
    format!(
        "{RESERVATIONS}{}",
        crate::journal::object_digest(operation.as_bytes())
    )
}

fn header(store: &Store, identity: &StoreIdentity) -> StoreResult<Header> {
    let snapshot = store.db.snapshot();
    let raw = codec::load(&snapshot, &store.journal, HEADER, header_payload_limit()?)?
        .ok_or_else(invalid)?;
    let header: Header = codec::decode(HEADER, &raw)?;
    if header.version != 2
        || header.policy != POLICY
        || &header.store != identity
        || header.bytes > MAX_BYTES
        || header.roots > MAX_ROOTS
        || header.reservations > MAX_RESERVATIONS
    {
        return Err(invalid());
    }
    verify_namespace(store, header.reservations)?;
    let mut bytes = HEADER_MARGIN;
    let mut roots = 0;
    let mut count = 0;
    for guard in snapshot
        .prefix(&store.journal, RESERVATIONS)
        .take(reservation_scan_limit(1)?)
    {
        count = add(count, 1)?;
        if count > MAX_RESERVATIONS {
            return Err(invalid());
        }
        let row_key = guard.key().map_err(|source| StoreError::Read { source })?;
        let (row_bytes, row_roots) = audit_row(store, &snapshot, &row_key, identity)?;
        bytes = add(bytes, row_bytes)?;
        roots = add(roots, row_roots)?;
        if bytes > MAX_BYTES || roots > MAX_ROOTS {
            return Err(invalid());
        }
    }
    if (header.bytes, header.roots, header.reservations) != (bytes, roots, count) {
        return Err(invalid());
    }
    Ok(header)
}

fn audit_row(
    store: &Store,
    snapshot: &fjall::Snapshot,
    row_key: &[u8],
    identity: &StoreIdentity,
) -> StoreResult<(u64, u64)> {
    let row_key = std::str::from_utf8(row_key).map_err(|_| invalid())?;
    let raw = codec::load(snapshot, &store.journal, row_key, MAX_JOURNAL_VALUE_BYTES)?
        .ok_or_else(invalid)?;
    #[derive(Deserialize)]
    struct Version {
        version: u8,
    }
    let version: Version = codec::decode(row_key, &raw)?;
    match version.version {
        3 => return graph::audit(store, row_key, &raw, identity),
        2 => (),
        _ => return Err(invalid()),
    }
    let reservation: Reservation = codec::decode(row_key, &raw)?;
    validate(&reservation, identity)?;
    for charge in &reservation.charges {
        crate::journal::context::verify_retained_charge(store, charge)?;
    }
    if row_key != key(reservation.decision.binding().operation) {
        return Err(invalid());
    }
    let [authority, affected] = &reservation.charges;
    Ok((
        charge(&reservation, row_key.len(), raw.len())?,
        add(authority.roots, affected.roots)?,
    ))
}

fn validate(reservation: &Reservation, identity: &StoreIdentity) -> StoreResult<()> {
    reservation.decision.require_schema()?;
    let binding = reservation.decision.binding();
    let [authority, affected] = &reservation.charges;
    if reservation.version != 2
        || !crate::journal::is_object_id(&authority.root)
        || !crate::journal::is_object_id(&affected.root)
        || binding.store != identity
        || authority.root != binding.authority_context
        || affected.root != binding.affected_context
        || reservation
            .charges
            .iter()
            .any(|charge| charge.roots > 1 || (charge.roots == 0) != (charge.bytes == 0))
    {
        return Err(invalid());
    }
    Ok(())
}

fn charge(reservation: &Reservation, key_bytes: usize, value_bytes: usize) -> StoreResult<u64> {
    let [authority, affected] = &reservation.charges;
    let overhead = key_bytes
        .checked_add(value_bytes)
        .ok_or(StoreError::CounterOverflow)?;
    add(
        u64::try_from(overhead).map_err(|_| StoreError::CounterOverflow)?,
        add(authority.bytes, affected.bytes)?,
    )
}

pub(in crate::public_identity) fn load(
    store: &Store,
    identity: &StoreIdentity,
    operation: &str,
) -> StoreResult<Option<Reservation>> {
    header(store, identity)?;
    let key = key(operation);
    codec::load(
        &store.db.snapshot(),
        &store.journal,
        &key,
        MAX_JOURNAL_VALUE_BYTES,
    )?
    .map(|raw| codec::decode(&key, &raw))
    .transpose()
}

impl Reservation {
    pub(in crate::public_identity) fn require_allowance(
        &self,
        measured: &[ContextCharge; 2],
    ) -> StoreResult<()> {
        for (original, current) in self.charges.iter().zip(measured) {
            if original.root != current.root
                || current.bytes > original.bytes
                || current.roots > original.roots
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
}

fn require_capacity(bytes: u64, roots: u64, reservations: u64) -> StoreResult<()> {
    if bytes > MAX_BYTES || roots > MAX_ROOTS || reservations > MAX_RESERVATIONS {
        return Err(invalid());
    }
    Ok(())
}

#[cfg(test)]
mod tests;

fn verify_namespace(store: &Store, reservations: u64) -> StoreResult<()> {
    let mut count = 0u64;
    for guard in store
        .db
        .snapshot()
        .prefix(&store.journal, PREFIX)
        .take(reservation_scan_limit(2)?)
    {
        count = add(count, 1)?;
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        if key.as_ref() != HEADER.as_bytes() && !key.starts_with(RESERVATIONS.as_bytes()) {
            return Err(invalid());
        }
    }
    if count != add(reservations, 1)? {
        return Err(invalid());
    }
    Ok(())
}

pub(crate) struct Permit<'s> {
    store: &'s Store,
    root: String,
    ready_required: bool,
}

impl Permit<'_> {
    pub(crate) fn root_for(&self, store: &Store) -> StoreResult<&str> {
        if !std::ptr::eq(self.store, store) {
            return Err(invalid());
        }
        if self.ready_required {
            let manifest = crate::journal::object_manifest_key(&self.root)?;
            if !store
                .db
                .snapshot()
                .contains_key(&store.journal, &manifest)
                .map_err(|source| StoreError::Read { source })?
            {
                return Err(invalid());
            }
        }
        Ok(&self.root)
    }
}

impl Reservation {
    pub(in crate::public_identity) fn permits<'s>(&self, store: &'s Store) -> [Permit<'s>; 2] {
        self.charges.each_ref().map(|charge| Permit {
            store,
            root: charge.root.clone(),
            ready_required: charge.roots == 0,
        })
    }
}

fn header_payload_limit() -> StoreResult<usize> {
    usize::try_from(HEADER_MARGIN)
        .map_err(|_| StoreError::CounterOverflow)?
        .checked_sub(HEADER.len())
        .ok_or(StoreError::CounterOverflow)
}

fn reservation_scan_limit(extra: usize) -> StoreResult<usize> {
    usize::try_from(MAX_RESERVATIONS)
        .map_err(|_| StoreError::CounterOverflow)?
        .checked_add(extra)
        .ok_or(StoreError::CounterOverflow)
}
