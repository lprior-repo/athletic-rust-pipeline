use super::*;
use crate::public_identity::{authority, commit, context::Inputs, retained::Revision, witness};
use census_domain::model::{serialized_digest, AthleteCandidateId};
use census_domain::public_identity_history::{
    genesis, prepare_next, CandidateDecisionReference, HistoryRequest, ReplayLimits,
};
use census_domain::public_identity_qualification::PublicIdentityQualificationV2;

pub(super) struct HistoricalFixture {
    pub original: wire::Bundle,
    pub frozen: FrozenPublicIdentityBundleV1,
    pub first: AthleteCandidateId,
}

pub(super) fn two_births() -> TestResult<HistoricalFixture> {
    let (_directory, store, _writer) = fixture()?;
    let first = subject("historical-frozen-first", "501")?;
    let second = subject("historical-frozen-second", "502")?;
    let first_id = first.id.cast();
    let inputs = Inputs {
        athletes: vec![first, second],
        observations: Vec::new(),
        cases: Vec::new(),
        verdicts: Vec::new(),
    };
    let identity = store.store_identity()?;
    let staged = inputs.stage(&store, &identity)?;
    let context =
        crate::read::read_context_object(&store.db.snapshot(), &store.journal, staged.object_id())?;
    let context_bytes = context.retained_bytes(crate::journal::context::CONTEXT_TOTAL_BYTES)?;
    let mut head = genesis(&identity)?;
    let mut records = Vec::new();
    let mut operations = Vec::new();
    for athlete in &inputs.athletes {
        let candidate = athlete.id.cast();
        let operation = format!("historical-frozen-{}", candidate.as_str());
        let request_digest = witness::request_digest(&identity, &operation, &candidate)?;
        let (accepted, source) = authority::accept_inputs(&inputs, &candidate, "2026-10-14")?;
        let prepared = prepare_next(
            &identity,
            &head,
            HistoryRequest::Establish {
                binding: commit::source_binding(&candidate, source, &request_digest),
                evidence: CandidateDecisionReference::from_accepted(
                    &accepted,
                    ReplayLimits::SMALL,
                )?,
            },
            ReplayLimits::SMALL,
        )?;
        let [public_id] = prepared.established_ids.as_slice() else {
            return Err("historical fixture establishment cardinality".into());
        };
        let held = witness::Witness {
            version: 2,
            operation: operation.clone(),
            candidate: candidate.as_str().to_owned(),
            request_digest,
            evidence_generation: prepared.uncommitted_record.sequence,
            derived_generation: 0,
            snapshot_sequence: prepared.uncommitted_record.sequence,
            record_digest: prepared.uncommitted_record.digest.clone(),
            decision_id: accepted.record().id.clone(),
            decision_digest: serialized_digest(accepted.record())?,
            public_id: public_id.as_str().to_owned(),
            authority_context: Some(staged.object_id().to_owned()),
        };
        let receipt = crate::Receipt {
            operation: witness::receipt_key(&operation),
            digest: serialized_digest(&held)?,
            at: accepted.record().observed_at.clone(),
            appended: 1,
            credited: false,
        };
        operations.push(wire::Operation {
            sequence: prepared.uncommitted_record.sequence,
            witness: serde_json::to_string_pretty(&held)?,
            receipt: serde_json::to_string_pretty(&receipt)?,
            decision: Some(serde_json::to_string_pretty(accepted.record())?),
        });
        records.push(prepared.uncommitted_record);
        head = prepared.next_head;
    }
    let revision = head
        .sequence
        .checked_add(1)
        .ok_or("historical fixture revision")?;
    let original = wire::Bundle {
        version: 1,
        store_identity: identity.clone(),
        revision: Revision {
            evidence_generation: revision,
            derived_generation: 0,
            snapshot_sequence: revision,
        },
        declaration: serde_json::to_string_pretty(&PublicIdentityQualificationV2::new(identity))?,
        head: serde_json::to_string_pretty(&head)?,
        history: serde_json::to_string_pretty(&records)?,
        operations,
        contexts: vec![wire::Context {
            root: staged.object_id().to_owned(),
            bytes: String::from_utf8(context_bytes)?,
        }],
    };
    let bytes = serde_json::to_vec(&original)?;
    let frozen = FrozenPublicIdentityBundleV1 {
        digest: digest(&bytes),
        bytes,
    };
    let replayed = frozen.replay()?;
    check!(eq; replayed.qualification_version(), PublicIdentityQualificationVersion::RetainedAuthorityContextV2);
    check!(eq; replayed.head(), &head);
    check!(eq; replayed.snapshot_sequence(), revision);
    Ok(HistoricalFixture {
        original,
        frozen,
        first: first_id,
    })
}

#[test]
fn historical_frozen_v1_replays_original_v2_witnesses_after_fixture_close() -> TestResult {
    let historical = two_births()?;
    let replayed = historical.frozen.replay()?;
    check!(eq; replayed.head().sequence, 2);
    check!(matches!(
        replayed.resolve_candidate(&historical.first)?,
        PublicResolution::Canonical(_)
    ));
    check!(eq; historical.original.contexts.len(), 1);
    for operation in &historical.original.operations {
        let held: witness::Witness = serde_json::from_str(&operation.witness)?;
        check!(eq; held.version, 2);
        check!(eq; held.authority_context.as_deref(), historical.original.contexts.first().map(|context| context.root.as_str()));
    }
    check!(
        replay_frozen_public_identity_v2(historical.frozen.bytes(), historical.frozen.digest(),)
            .is_err()
    );
    Ok(())
}

#[test]
fn historical_frozen_v1_preserves_original_embedded_json_and_context_bytes() -> TestResult {
    let historical = two_births()?;
    let decoded: wire::Bundle = serde_json::from_slice(historical.frozen.bytes())?;
    check!(eq; decoded.declaration.as_bytes(), historical.original.declaration.as_bytes());
    check!(eq; decoded.head.as_bytes(), historical.original.head.as_bytes());
    check!(eq; decoded.history.as_bytes(), historical.original.history.as_bytes());
    check!(eq; decoded.operations.len(), historical.original.operations.len());
    for (decoded, original) in decoded
        .operations
        .iter()
        .zip(&historical.original.operations)
    {
        check!(original.witness.contains('\n'));
        check!(eq; decoded.witness.as_bytes(), original.witness.as_bytes());
        check!(eq; decoded.receipt.as_bytes(), original.receipt.as_bytes());
        check!(eq; decoded.decision.as_deref(), original.decision.as_deref());
    }
    check!(eq; decoded.contexts.len(), historical.original.contexts.len());
    for (decoded, original) in decoded.contexts.iter().zip(&historical.original.contexts) {
        check!(eq; decoded.root, original.root);
        check!(eq; decoded.bytes.as_bytes(), original.bytes.as_bytes());
    }
    check!(eq; historical.frozen.replay()?.head().sequence, 2);
    Ok(())
}

#[test]
fn historical_frozen_v1_empty_ledger_keeps_empty_context_inventory() -> TestResult {
    let mut bundle = two_births()?.original;
    let head = genesis(&bundle.store_identity)?;
    bundle.head = serde_json::to_string(&head)?;
    bundle.history = "[]".to_owned();
    bundle.operations.clear();
    bundle.contexts.clear();
    let bytes = serde_json::to_vec(&bundle)?;
    let replayed = replay_frozen_public_identity_v1(&bytes, &digest(&bytes))?;
    check!(eq; replayed.head(), &head);
    check!(eq; replayed.snapshot_sequence(), bundle.revision.snapshot_sequence);
    check!(eq; replayed.qualification_version(), PublicIdentityQualificationVersion::RetainedAuthorityContextV2);
    bundle.contexts.push(wire::Context {
        root: "0".repeat(64),
        bytes: String::new(),
    });
    check!(matches!(
        tamper::corrupted(&bundle)?,
        StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "frozen authority context inventory"
        ))
    ));
    Ok(())
}
