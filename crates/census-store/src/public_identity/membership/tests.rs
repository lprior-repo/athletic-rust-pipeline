use super::*;
use crate::public_identity::context::reviewed_fixture::review;
use crate::public_identity::graph_witness::{HeldGraphWitness, Payload};
use crate::public_identity::membership_withholding::MembershipWithholdingInventoryV1;
use crate::public_identity_tests::support::{fixture, retained_state, subject, TestResult};
use crate::{PublicIdentityError, PublicIdentityWriterV1, Store, StoreError, Table};
use census_domain::model::{AppliedAthleteIdentity, CanonicalAthlete, SchoolId};
use census_domain::public_identity_history::PublicResolution;

struct Fixture {
    _directory: tempfile::TempDir,
    store: Store,
    writer: PublicIdentityWriterV1,
    request: SourceCandidateMembershipRequestV1,
}

fn ready() -> TestResult<Fixture> {
    let (directory, store, writer) = fixture()?;
    let first = subject("membership", "7401")?;
    store.append(Table::Athletes, &first)?;
    let birth = writer.establish_source_bound(
        &store,
        "membership-birth",
        &first.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let mut second = CanonicalAthlete::new(
        &SchoolId::mint("sch", &["membership-second-school"]),
        &first.canonical_name,
        first.grad_year,
        first.gender,
        first.source.clone().ok_or("primary")?,
    )?;
    second.evidence = first.evidence.clone();
    store.append(Table::Athletes, &second)?;
    let (case, verdict) = review(&store, &[first.id.cast(), second.id.cast()], "same_person")?;
    store.replace_many(Table::ReviewCases, std::slice::from_ref(&case))?;
    store.replace_many(Table::IdentityVerdicts, &[verdict])?;
    let key = crate::public_identity::witness::key(1);
    let raw = store.journal.get(&key)?.ok_or("birth witness")?;
    let witness = HeldGraphWitness::decode(&key, &raw)?;
    check!(eq; witness.version, 8);
    check!(
        matches!(&witness.payload, Payload::Source { candidate, .. } if candidate == &first.id.cast())
    );
    let request = SourceCandidateMembershipRequestV1 {
        case_id: case.id,
        owner: birth.public_id().clone(),
        anchor_binding_id: format!("source_binding_v1_{}", witness.request_digest),
        candidate: second.id.cast(),
    };
    Ok(Fixture {
        _directory: directory,
        store,
        writer,
        request,
    })
}

#[test]
fn genuine_membership_receipt_and_frozen_replay_preserve_anchor() -> TestResult {
    let Fixture {
        _directory,
        store,
        writer,
        request,
    } = ready()?;
    let before = store.snapshot().freeze_public_identity_v2()?;
    let snapshot = store.snapshot();
    let head = writer.head(&store)?;
    let attached = writer.extend_source_candidate(
        &store,
        "membership-attach",
        &request,
        snapshot.revision(),
        &head,
    )?;
    check!(eq; attached.public_id(), &request.owner);
    check!(attached.introduced_public_ids().is_empty());
    check!(!attached.replayed());
    check!(eq; attached.head().sequence, 2);
    let after = store.snapshot().freeze_public_identity_v2()?;
    check!(eq; before.replay()?.resolve_candidate(&request.candidate)?, PublicResolution::Unknown);
    let resolution = after.replay()?.resolve_candidate(&request.candidate)?;
    check!(
        matches!(&resolution, PublicResolution::Canonical(owner) if owner.public_id == request.owner)
    );
    check!(eq; store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?.len(), 2);
    let state = retained_state(&store)?;
    let repeated = writer.extend_source_candidate(
        &store,
        "membership-attach",
        &request,
        snapshot.revision(),
        &head,
    )?;
    check!(repeated.replayed());
    check!(eq; repeated.head(), attached.head());
    check!(eq; retained_state(&store)?, state);
    let mut changed = request.clone();
    changed.case_id.push_str("-changed");
    check!(writer
        .extend_source_candidate(
            &store,
            "membership-attach",
            &changed,
            snapshot.revision(),
            &head
        )
        .is_err());
    check!(eq; retained_state(&store)?, state);
    drop(snapshot);
    drop(writer);
    drop(store);
    std::fs::remove_dir_all(_directory.path().join("identity"))?;
    check!(eq; after.replay()?.resolve_candidate(&request.candidate)?, resolution);
    check!(eq; before.replay()?.resolve_candidate(&request.candidate)?, PublicResolution::Unknown);
    Ok(())
}

#[test]
fn membership_refuses_stale_foreign_and_wrong_anchor_atomically() -> TestResult {
    let fixture = ready()?;
    let Fixture {
        store,
        writer,
        request,
        ..
    } = &fixture;
    let snapshot = store.snapshot();
    let head = writer.head(store)?;
    let mut wrong = request.clone();
    wrong.anchor_binding_id.push_str("-foreign");
    stage_inventory(store, writer, "wrong-anchor", &request.owner)?;
    let before = retained_state(store)?;
    check!(matches!(
        writer.extend_source_candidate(store, "wrong-anchor", &wrong, snapshot.revision(), &head),
        Err(StoreError::PublicIdentity(
            PublicIdentityError::NotSourceBound
        ))
    ));
    check!(eq; retained_state(store)?, before);
    let other = ready()?;
    check!(matches!(
        writer.extend_source_candidate(
            store,
            "foreign-revision",
            request,
            other.store.snapshot().revision(),
            &head
        ),
        Err(StoreError::RevisionOriginMismatch { operation }) if operation == "foreign-revision"
    ));
    let mut wrong_head = head.clone();
    wrong_head.digest.push('0');
    check!(matches!(
        writer.extend_source_candidate(
            store,
            "wrong-head",
            request,
            snapshot.revision(),
            &wrong_head
        ),
        Err(StoreError::PublicIdentity(PublicIdentityError::HeadMoved))
    ));
    check!(eq; retained_state(store)?, before);
    store.append(Table::Athletes, &subject("membership-unrelated", "7402")?)?;
    let before = retained_state(store)?;
    check!(matches!(
        writer.extend_source_candidate(store, "stale", request, snapshot.revision(), &head),
        Err(StoreError::RevisionMoved { operation, .. }) if operation == "stale"
    ));
    check!(eq; retained_state(store)?, before);
    Ok(())
}

#[test]
fn membership_witness_tamper_and_legacy_manual_version_refuse() -> TestResult {
    let fixture = ready()?;
    let Fixture {
        store,
        writer,
        request,
        ..
    } = &fixture;
    writer.extend_source_candidate(
        store,
        "attach-tamper",
        request,
        store.snapshot().revision(),
        &writer.head(store)?,
    )?;
    let key = crate::public_identity::witness::key(2);
    let raw = store.journal.get(&key)?.ok_or("membership witness")?;
    let original: serde_json::Value = serde_json::from_slice(&raw)?;
    check!(eq; original.get("version").and_then(serde_json::Value::as_u64), Some(11));
    for pointer in [
        "/request_digest",
        "/payload/decision/digest",
        "/record_digest",
        "/graph/seal/root",
    ] {
        let mut value = original.clone();
        *value
            .pointer_mut(pointer)
            .ok_or("membership tamper target")? = serde_json::Value::String("0".repeat(64));
        let mut batch = store.db.batch();
        batch.insert(&store.journal, &key, serde_json::to_vec(&value)?);
        batch.commit()?;
        check!(writer.head(store).is_err());
    }
    let mut batch = store.db.batch();
    batch.insert(&store.journal, &key, raw.as_ref());
    batch.commit()?;
    check!(eq; writer.head(store)?.sequence, 2);
    let mut legacy = original;
    *legacy.get_mut("version").ok_or("witness version")? = serde_json::Value::from(4);
    check!(matches!(
        crate::public_identity::operation::HeldWitness::decode(
            "manual4",
            &serde_json::to_vec(&legacy)?
        ),
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "unsupported identity witness version"
        )))
    ));
    let mut oversized = raw.to_vec();
    oversized.resize(65_537, b' ');
    check!(matches!(
        HeldGraphWitness::decode("oversized", &oversized),
        Err(StoreError::RowTooLarge {
            bytes: 65_537,
            max: 65_536
        })
    ));
    Ok(())
}

#[test]
fn membership_rejects_rehashed_source_graph_substitution() -> TestResult {
    let fixture = ready()?;
    let Fixture {
        store,
        writer,
        request,
        ..
    } = &fixture;
    writer.extend_source_candidate(
        store,
        "graph-substitution",
        request,
        store.snapshot().revision(),
        &writer.head(store)?,
    )?;
    let original_state = retained_state(store)?;
    let original_head = writer.head(store)?;
    let source = store
        .journal
        .get(crate::public_identity::witness::key(1))?
        .ok_or("source graph witness")?;
    let source = HeldGraphWitness::decode("source graph witness", &source)?;
    let key = crate::public_identity::witness::key(2);
    let raw = store.journal.get(&key)?.ok_or("membership graph witness")?;
    let mut witness = HeldGraphWitness::decode(&key, &raw)?;
    check!(witness.graph != source.graph);
    assert_substituted_graph_is_not_reviewed(
        store,
        &source.graph,
        &request.case_id,
        &witness.observed_at,
    )?;
    let original_digest = census_domain::model::serialized_digest(&witness)?;
    witness.graph = source.graph;
    let receipt_key = crate::public_identity::witness::receipt_key("graph-substitution");
    let raw_receipt = store
        .receipts
        .get(&receipt_key)?
        .ok_or("membership receipt")?;
    let mut receipt: crate::Receipt = serde_json::from_slice(&raw_receipt)?;
    check!(eq; receipt.digest, original_digest);
    receipt.digest = census_domain::model::serialized_digest(&witness)?;
    check!(ne; receipt.digest, original_digest);
    let mut batch = store.db.batch();
    batch.insert(&store.journal, &key, serde_json::to_vec(&witness)?);
    batch.insert(&store.receipts, &receipt_key, serde_json::to_vec(&receipt)?);
    batch.commit()?;
    let tampered = retained_state(store)?;
    assert_substitution_ledger_refusal(writer.head(store))?;
    check!(eq; retained_state(store)?, tampered);
    let mut batch = store.db.batch();
    batch.insert(&store.journal, &key, raw.as_ref());
    batch.insert(&store.receipts, &receipt_key, raw_receipt.as_ref());
    batch.commit()?;
    check!(eq; writer.head(store)?.sequence, 2);
    check!(eq; writer.head(store)?, original_head);
    check!(eq; retained_state(store)?, original_state);
    Ok(())
}

fn assert_substituted_graph_is_not_reviewed(
    store: &Store,
    graph: &crate::public_identity::graph_reference::GraphAuthorityReferenceV1,
    case_id: &str,
    observed_at: &str,
) -> TestResult {
    let snapshot = store.snapshot();
    let view = crate::public_identity::paged_generation::VerifiedRawGraph::verify(
        &snapshot,
        graph.capture(),
        graph.seal(),
    )?;
    match crate::public_identity::authority::graph::accept_reviewed(&view, case_id, observed_at) {
        Err(StoreError::PublicIdentity(PublicIdentityError::NotReviewed)) => Ok(()),
        Err(error) => {
            Err(format!("unexpected substituted graph authority refusal: {error:?}").into())
        }
        Ok(_) => Err("substituted source graph unexpectedly authorized membership review".into()),
    }
}

fn assert_substitution_ledger_refusal(
    result: crate::StoreResult<census_domain::public_identity_history::RegistryHead>,
) -> TestResult {
    match result {
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "manual review case missing",
        ))) => Ok(()),
        Err(error) => Err(format!("unexpected substituted graph ledger refusal: {error:?}").into()),
        Ok(head) => Err(format!("substituted source graph unexpectedly verified: {head:?}").into()),
    }
}

fn stage_inventory(
    store: &Store,
    writer: &PublicIdentityWriterV1,
    operation: &str,
    canonical: &census_domain::public_identity_history::PublicAthleteId,
) -> TestResult<MembershipWithholdingInventoryV1> {
    let before = retained_state(store)?;
    let inventory = writer.inspect_membership_withholding(store, operation, canonical)?;
    let after = retained_state(store)?;
    let raw_operation =
        crate::public_identity::graph_operation::raw_operation(writer.store_identity(), operation)?;
    let reservation_key = format!(
        "manual-context-admission-v1\0reservation\0{}",
        crate::journal::object_digest(raw_operation.as_str().as_bytes())
    );
    let reservation = store
        .journal
        .get(&reservation_key)?
        .ok_or("graph reservation")?;
    let reservation: serde_json::Value = serde_json::from_slice(&reservation)?;
    check!(eq; reservation.get("version").and_then(serde_json::Value::as_u64), Some(3));
    let allowed = staged_graph_keys(&reservation, &reservation_key)?;
    check!(eq; before.len(), after.len());
    for ((before_name, before_rows), (after_name, after_rows)) in before.iter().zip(&after) {
        check!(eq; before_name, after_name);
        if before_name != crate::JOURNAL {
            check!(eq; before_rows, after_rows);
            continue;
        }
        let header_key = b"manual-context-admission-v1\0header";
        for (key, value) in before_rows {
            let (_, current) = after_rows
                .iter()
                .find(|(held, _)| held == key)
                .ok_or("inspection removed a retained row")?;
            if key.as_slice() != header_key {
                check!(eq; current, value);
            }
        }
        let added: Vec<_> = after_rows
            .iter()
            .filter(|(key, _)| !before_rows.iter().any(|(prior, _)| prior == key))
            .collect();
        check!(added.iter().all(|(key, _)| allowed.contains(key)));
        check!(added
            .iter()
            .any(|(key, _)| key == reservation_key.as_bytes()));
        let bytes = added
            .iter()
            .try_fold(0_u64, |total, (key, value)| -> TestResult<u64> {
                let row = key
                    .len()
                    .checked_add(value.len())
                    .ok_or("row charge overflow")?;
                Ok(total
                    .checked_add(u64::try_from(row)?)
                    .ok_or("charge overflow")?)
            })?;
        let roots = u64::try_from(
            added
                .iter()
                .filter(|(key, _)| key.starts_with(b"j2\0manifest\0"))
                .count(),
        )?;
        let header = |rows: &[(Vec<u8>, Vec<u8>)]| -> TestResult<serde_json::Value> {
            let (_, raw) = rows
                .iter()
                .find(|(key, _)| key.as_slice() == header_key)
                .ok_or("admission header")?;
            Ok(serde_json::from_slice(raw)?)
        };
        let mut expected_header = header(before_rows)?;
        for (field, increment) in [("bytes", bytes), ("roots", roots), ("reservations", 1)] {
            let count = expected_header
                .get(field)
                .and_then(serde_json::Value::as_u64)
                .ok_or("admission counter")?;
            *expected_header.get_mut(field).ok_or("admission counter")? =
                serde_json::Value::from(count.checked_add(increment).ok_or("counter overflow")?);
        }
        check!(eq; header(after_rows)?, expected_header);
    }
    let sequence = store.snapshot().sequence();
    let repeated = writer.inspect_membership_withholding(store, operation, canonical)?;
    check!(eq; repeated.graph(), inventory.graph());
    check!(eq; repeated.claims(), inventory.claims());
    check!(eq; repeated.affected_record_keys(), inventory.affected_record_keys());
    check!(eq; retained_state(store)?, after);
    check!(eq; store.snapshot().sequence(), sequence);
    Ok(inventory)
}

fn staged_graph_keys(
    reservation: &serde_json::Value,
    reservation_key: &str,
) -> TestResult<std::collections::BTreeSet<Vec<u8>>> {
    let descriptors = reservation
        .get("descriptors")
        .and_then(serde_json::Value::as_array)
        .ok_or("graph descriptors")?;
    let mut keys = std::collections::BTreeSet::from([reservation_key.as_bytes().to_vec()]);
    for descriptor in descriptors {
        let physical = descriptor.get("physical").ok_or("physical descriptor")?;
        let root = physical
            .get("root")
            .and_then(serde_json::Value::as_str)
            .ok_or("physical root")?;
        keys.insert(crate::journal::object_manifest_key(root)?);
        for (field, chunk) in [("chunks", true), ("pages", false)] {
            let rows = physical
                .get(field)
                .and_then(serde_json::Value::as_array)
                .ok_or("physical rows")?;
            for ordinal in 0..rows.len() {
                keys.insert(if chunk {
                    crate::journal::object_chunk_key(root, ordinal)?
                } else {
                    crate::journal::object_index_key(root, ordinal)?
                });
            }
        }
    }
    Ok(keys)
}

mod withholding;

mod admission;
