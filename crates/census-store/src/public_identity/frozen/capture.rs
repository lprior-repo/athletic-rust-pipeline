use super::{digest, wire, FrozenPublicIdentityBundleV1, MAX_FROZEN_PUBLIC_IDENTITY_BYTES};
use crate::public_identity::{
    codec,
    operation::HeldWitness,
    retained::{Live, Read, Rows},
    witness, PublicIdentityError, DECLARATION, HEAD, HISTORY, MAX_PUBLIC_IDENTITY_HISTORY_BYTES,
};
use crate::{AcceptedPublicIdentitySnapshot, StoreError, StoreResult};
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;
use fjall::{Keyspace, Snapshot};
use std::collections::BTreeSet;

pub(crate) fn capture(
    snapshot: &Snapshot,
    entities: &Keyspace,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    accepted: &AcceptedPublicIdentitySnapshot,
) -> StoreResult<FrozenPublicIdentityBundleV1> {
    if accepted.qualification_version()
        != PublicIdentityQualificationVersion::RetainedAuthorityContextV2
    {
        return Err(
            PublicIdentityError::Corrupt("frozen identity requires retained authority V2").into(),
        );
    }
    let reader = Live {
        snapshot,
        entities,
        meta,
        journal,
        receipts,
    };
    let mut bundle = wire::Bundle {
        version: 1,
        store_identity: accepted.store_identity().clone(),
        revision: reader.revision()?,
        declaration: metadata(snapshot, meta, DECLARATION, 65_536)?,
        head: metadata(snapshot, meta, HEAD, 1024)?,
        history: metadata(snapshot, meta, HISTORY, MAX_PUBLIC_IDENTITY_HISTORY_BYTES)?,
        operations: Vec::new(),
        contexts: Vec::new(),
    };
    let mut remaining = MAX_FROZEN_PUBLIC_IDENTITY_BYTES;
    charge(&mut remaining, bundle.declaration.len())?;
    charge(&mut remaining, bundle.head.len())?;
    charge(&mut remaining, bundle.history.len())?;
    let roots = capture_operations(&reader, accepted, &mut bundle, &mut remaining)?;
    capture_contexts(snapshot, journal, roots, &mut bundle, &mut remaining)?;
    finish(bundle, accepted)
}

fn capture_operations(
    reader: &impl Read,
    accepted: &AcceptedPublicIdentitySnapshot,
    bundle: &mut wire::Bundle,
    remaining: &mut usize,
) -> StoreResult<BTreeSet<String>> {
    let mut roots = BTreeSet::new();
    for record in accepted.history_records() {
        let raw = row(reader, Rows::Witness, &witness::key(record.sequence))?;
        let held = HeldWitness::decode("frozen witness", raw.as_bytes())?;
        let receipt = row(
            reader,
            Rows::Receipt,
            &witness::receipt_key(held.operation()),
        )?;
        let decision = held
            .decision_id()
            .map(|id| row(reader, Rows::Decision, id))
            .transpose()?;
        for bytes in [&raw, &receipt] {
            charge(remaining, bytes.len())?;
        }
        if let Some(bytes) = &decision {
            charge(remaining, bytes.len())?;
        }
        roots.insert(
            held.authority_context()
                .ok_or(PublicIdentityError::Corrupt(
                    "missing frozen authority context",
                ))?
                .to_owned(),
        );
        if let Some(root) = held.affected_context() {
            roots.insert(root.to_owned());
        }
        bundle.operations.try_reserve(1).map_err(allocation)?;
        bundle.operations.push(wire::Operation {
            sequence: record.sequence,
            witness: raw,
            receipt,
            decision,
        });
    }
    Ok(roots)
}

fn capture_contexts(
    snapshot: &Snapshot,
    journal: &Keyspace,
    roots: BTreeSet<String>,
    bundle: &mut wire::Bundle,
    remaining: &mut usize,
) -> StoreResult<()> {
    for root in roots {
        let object = crate::read::read_context_object(snapshot, journal, &root)?;
        let raw = object
            .retained_bytes((*remaining).min(crate::journal::context::CONTEXT_TOTAL_BYTES))?;
        charge(remaining, raw.len())?;
        bundle.contexts.try_reserve(1).map_err(allocation)?;
        bundle.contexts.push(wire::Context {
            root,
            bytes: text(raw)?,
        });
    }
    Ok(())
}

fn finish(
    bundle: wire::Bundle,
    accepted: &AcceptedPublicIdentitySnapshot,
) -> StoreResult<FrozenPublicIdentityBundleV1> {
    bundle.bounds()?;
    let bytes = codec::encode(&bundle, MAX_FROZEN_PUBLIC_IDENTITY_BYTES)?;
    let digest = digest(&bytes);
    let replayed = super::replay::replay(&bytes, &digest, MAX_FROZEN_PUBLIC_IDENTITY_BYTES)?;
    if replayed.store_identity() != accepted.store_identity()
        || replayed.head() != accepted.head()
        || replayed.snapshot_sequence() != accepted.snapshot_sequence()
    {
        return Err(PublicIdentityError::Corrupt("frozen capture snapshot binding").into());
    }
    Ok(FrozenPublicIdentityBundleV1 { bytes, digest })
}

fn metadata(
    snapshot: &Snapshot,
    meta: &Keyspace,
    key: &str,
    maximum: usize,
) -> StoreResult<String> {
    let raw = codec::load(snapshot, meta, key, maximum)?
        .ok_or(PublicIdentityError::Corrupt("missing frozen metadata"))?;
    text(raw.to_vec())
}
fn row(reader: &impl Read, rows: Rows, key: &str) -> StoreResult<String> {
    text(reader.row(rows, key)?.ok_or(PublicIdentityError::Corrupt(
        "missing frozen operation material",
    ))?)
}
fn text(bytes: Vec<u8>) -> StoreResult<String> {
    String::from_utf8(bytes)
        .map_err(|_| PublicIdentityError::Corrupt("frozen material UTF-8").into())
}
fn charge(remaining: &mut usize, bytes: usize) -> StoreResult<()> {
    *remaining = remaining
        .checked_sub(bytes)
        .ok_or(PublicIdentityError::Corrupt(
            "frozen identity aggregate byte budget",
        ))?;
    Ok(())
}
fn allocation(error: std::collections::TryReserveError) -> StoreError {
    StoreError::Refused {
        detail: format!("frozen identity allocation: {error}"),
    }
}
