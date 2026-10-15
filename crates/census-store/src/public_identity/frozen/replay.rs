use super::{digest, wire};
use crate::public_identity::{
    codec, context,
    operation::HeldWitness,
    qualification,
    retained::{Read, Revision, Rows},
    witness, PublicIdentityError,
};
use crate::{AcceptedPublicIdentitySnapshot, StoreResult};
use census_domain::model::StoreIdentity;
use census_domain::public_identity_qualification::{
    PublicIdentityQualification, PublicIdentityQualificationV2,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn replay(
    bytes: &[u8],
    expected_digest: &str,
    maximum: usize,
) -> StoreResult<AcceptedPublicIdentitySnapshot> {
    wire::bound(
        bytes.len(),
        maximum.min(super::MAX_FROZEN_PUBLIC_IDENTITY_BYTES),
    )?;
    if !crate::journal::is_object_id(expected_digest) || digest(bytes) != expected_digest {
        return Err(PublicIdentityError::Corrupt("frozen identity bundle digest").into());
    }
    let bundle: wire::Bundle = codec::decode("frozen identity bundle", bytes)?;
    bundle.bounds()?;
    let declaration =
        PublicIdentityQualification::V2(codec::decode::<PublicIdentityQualificationV2>(
            "frozen identity declaration",
            bundle.declaration.as_bytes(),
        )?);
    if declaration.store_identity() != &bundle.store_identity
        || bundle.revision.snapshot_sequence == 0
    {
        return Err(PublicIdentityError::Corrupt("frozen identity lineage binding").into());
    }
    let head = codec::head(bundle.head.as_bytes())?;
    let history = codec::history(bundle.history.as_bytes(), &bundle.store_identity, &head)?;
    if bundle.operations.len() != history.records().len()
        || bundle.revision.evidence_generation < head.sequence
    {
        return Err(PublicIdentityError::Corrupt("frozen identity operation inventory").into());
    }
    let reader = Retained::new(&bundle)?;
    qualification::witnesses::validate_retained(&reader, &history, &declaration)?;
    Ok(AcceptedPublicIdentitySnapshot::from_qualified_ledger(
        qualification::Ledger {
            declaration,
            history,
        },
        bundle.revision.snapshot_sequence,
    ))
}

struct Retained<'a> {
    revision: Revision,
    witnesses: BTreeMap<String, &'a str>,
    receipts: BTreeMap<String, &'a str>,
    decisions: BTreeMap<String, &'a str>,
    contexts: BTreeMap<&'a str, &'a str>,
}

impl<'a> Retained<'a> {
    fn new(bundle: &'a wire::Bundle) -> StoreResult<Self> {
        let mut reader = Self {
            revision: bundle.revision,
            witnesses: BTreeMap::new(),
            receipts: BTreeMap::new(),
            decisions: BTreeMap::new(),
            contexts: BTreeMap::new(),
        };
        let required = reader.operations(&bundle.operations)?;
        for context in &bundle.contexts {
            if reader
                .contexts
                .insert(context.root.as_str(), context.bytes.as_str())
                .is_some()
            {
                return Err(
                    PublicIdentityError::Corrupt("duplicate frozen authority context").into(),
                );
            }
        }
        if required.iter().map(String::as_str).collect::<BTreeSet<_>>()
            != reader.contexts.keys().copied().collect()
        {
            return Err(PublicIdentityError::Corrupt("frozen authority context inventory").into());
        }
        Ok(reader)
    }

    fn operations(&mut self, operations: &'a [wire::Operation]) -> StoreResult<BTreeSet<String>> {
        let mut required = BTreeSet::new();
        for (index, operation) in operations.iter().enumerate() {
            let expected = u64::try_from(index)
                .map_err(|_| crate::StoreError::CounterOverflow)?
                .checked_add(1)
                .ok_or(crate::StoreError::CounterOverflow)?;
            if operation.sequence != expected {
                return Err(PublicIdentityError::Corrupt("frozen operation sequence").into());
            }
            let held = HeldWitness::decode("frozen witness", operation.witness.as_bytes())?;
            insert(
                &mut self.witnesses,
                witness::key(operation.sequence),
                &operation.witness,
            )?;
            insert(
                &mut self.receipts,
                witness::receipt_key(held.operation()),
                &operation.receipt,
            )?;
            match (held.decision_id(), operation.decision.as_deref()) {
                (Some(id), Some(decision)) => insert(&mut self.decisions, id.to_owned(), decision)?,
                (None, None) => {}
                _ => {
                    return Err(
                        PublicIdentityError::Corrupt("frozen decision material kind").into(),
                    )
                }
            }
            required.insert(
                held.authority_context()
                    .ok_or(PublicIdentityError::Corrupt(
                        "missing frozen authority context",
                    ))?
                    .to_owned(),
            );
        }
        Ok(required)
    }
}

impl Read for Retained<'_> {
    fn row(&self, rows: Rows, key: &str) -> StoreResult<Option<Vec<u8>>> {
        let values = match rows {
            Rows::Witness => &self.witnesses,
            Rows::Receipt => &self.receipts,
            Rows::Decision => &self.decisions,
        };
        Ok(values.get(key).map(|bytes| bytes.as_bytes().to_vec()))
    }
    fn context(&self, root: &str, identity: &StoreIdentity) -> StoreResult<context::Inputs> {
        let bytes = self.contexts.get(root).ok_or(PublicIdentityError::Corrupt(
            "missing frozen authority context",
        ))?;
        context::from_bytes(bytes.as_bytes(), root, identity)
    }
    fn revision(&self) -> StoreResult<Revision> {
        Ok(self.revision)
    }
}

fn insert<'a>(map: &mut BTreeMap<String, &'a str>, key: String, bytes: &'a str) -> StoreResult<()> {
    if map.insert(key, bytes).is_some() {
        return Err(PublicIdentityError::Corrupt("duplicate frozen operation material").into());
    }
    Ok(())
}
