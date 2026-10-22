mod capacity;
use capacity::require_capacity;
mod decision;
pub(in crate::public_identity) mod graph;
mod graph_wire;
mod persist;
pub(in crate::public_identity) mod preparation;
use crate::journal::context::ContextCharge;
use crate::public_identity::{codec, PublicIdentityAdmissionResource, PublicIdentityError};
use crate::{Store, StoreError, StoreResult, MAX_JOURNAL_VALUE_BYTES};
use census_domain::model::StoreIdentity;
use decision::Decision;
use fjall::Readable;
use serde::{Deserialize, Serialize};

const PREFIX: &str = "manual-context-admission-v1\0";
const HEADER: &str = "manual-context-admission-v1\0header";
const RESERVATIONS: &str = "manual-context-admission-v1\0reservation\0";
const POLICY: &str = "manual-context-admission/v2";
const MAX_BYTES: u64 = 8 * 1024 * 1024 * 1024;
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
pub(in crate::public_identity) fn key(operation: &str) -> String {
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

pub(crate) fn owns_integrity_phase(phase: &str) -> bool {
    PREFIX.strip_suffix('\0') == Some(phase)
}

pub(crate) fn audit_integrity(store: &Store) -> StoreResult<()> {
    header(store, &store.store_identity()?).map(|_| ())
}

fn audit_row(
    store: &Store,
    snapshot: &fjall::Snapshot,
    row_key: &[u8],
    identity: &StoreIdentity,
) -> StoreResult<(u64, u64)> {
    let row_key = std::str::from_utf8(row_key).map_err(|_| invalid())?;
    let raw = codec::load(
        snapshot,
        &store.journal,
        row_key,
        graph_wire::MAX_METADATA_BYTES,
    )?
    .ok_or_else(invalid)?;
    #[derive(Deserialize)]
    struct Version {
        version: u8,
    }
    let version: Version = codec::decode(row_key, &raw)?;
    if version.version != 3 && raw.len() > MAX_JOURNAL_VALUE_BYTES {
        return Err(StoreError::RowTooLarge {
            bytes: raw.len(),
            max: MAX_JOURNAL_VALUE_BYTES,
        });
    }
    match version.version {
        3 => return graph::audit(store, row_key, &raw, identity),
        4 => return preparation::audit(store, row_key, &raw, identity),
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

#[cfg(test)]
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

#[cfg(test)]
mod capacity_tests;
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

#[cfg(test)]
pub(in crate::public_identity) fn enroll_header_for_test(store: &Store) -> StoreResult<()> {
    let identity = store.store_identity()?;
    let _guard = store.lock_appends();
    persist::enroll_empty(store, &identity)
}
