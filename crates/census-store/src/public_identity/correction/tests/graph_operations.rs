use super::graph_fixture::{self, Fixture};
use super::*;
use crate::{
    AcceptedPublicIdentitySnapshot, FrozenPublicIdentityBundleV2, ReviewedIdentityRequestV1,
};
use census_domain::public_identity_history::{prepare_next, HistoryRequest, PublicResolution};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[test]
fn reviewed_split_uses_complete_later_page_graph_and_reopens_with_stable_successors() -> TestResult
{
    let fixture = graph_fixture::ready()?;
    let request = graph_fixture::split(&fixture)?;
    let (accepted, frozen) = apply(&fixture, "graph-reviewed-split", &request)?;
    check!(eq; accepted.head().sequence, 7);
    let PublicResolution::Ambiguous {
        successors,
        revoked_merges,
        ..
    } = accepted.resolve_public(fixture.births[0].public_id())?
    else {
        return Err("reviewed split must retain ambiguous canonical predecessor".into());
    };
    check!(eq; successors.len(), 3);
    check!(eq; revoked_merges, vec![4, 5, 6]);
    for (birth, member) in fixture.births.iter().zip(&fixture.members).skip(1) {
        check!(
            matches!(accepted.resolve_public(birth.public_id())?, PublicResolution::RevokedRedirect { successor, revoked_merges, .. }
            if successor.representative_candidate == member.id.cast() && revoked_merges == vec![4, 5, 6] && successors.contains(&successor))
        );
    }
    check!(eq; accepted.history_records().last().ok_or("split")?.sequence, 7);
    finish_offline(fixture, accepted, frozen)
}

#[test]
fn reviewed_revoke_preserves_a_redundantly_connected_component_and_replays_offline() -> TestResult {
    let fixture = graph_fixture::ready()?;
    let request = graph_fixture::revoke(&fixture);
    let (accepted, frozen) = apply(&fixture, "graph-reviewed-revoke", &request)?;
    check!(eq; accepted.head().sequence, 7);
    check!(
        matches!(accepted.resolve_public(fixture.births[0].public_id())?, PublicResolution::Canonical(owner) if owner.public_id == *fixture.births[0].public_id())
    );
    for birth in fixture.births.iter().skip(1) {
        check!(
            matches!(accepted.resolve_public(birth.public_id())?, PublicResolution::RevokedRedirect { successor, revoked_merges, .. }
            if successor.public_id == *fixture.births[0].public_id() && revoked_merges == vec![4])
        );
    }
    finish_offline(fixture, accepted, frozen)
}

#[test]
fn reviewed_corrections_refuse_omitted_or_fabricated_later_page_keys_without_commit() -> TestResult
{
    let fixture = graph_fixture::ready()?;
    for (family, original) in [
        ("split", graph_fixture::split(&fixture)?),
        ("revoke", graph_fixture::revoke(&fixture)),
    ] {
        for (variant, key) in [
            ("omit-performance", "performances:"),
            ("omit-relay", "relay_results:"),
            ("fabricate", "fabricated:"),
        ] {
            let mut changed = original.clone();
            let keys = keys_mut(&mut changed)?;
            if variant == "fabricate" {
                keys.push("performances:invented-row".into());
                keys.sort();
            } else {
                let held = keys
                    .iter()
                    .find(|held| held.starts_with(key))
                    .cloned()
                    .ok_or("later-page key")?;
                keys.retain(|value| value != &held);
            }
            let operation = format!("graph-reviewed-{family}-{variant}");
            stage_graph(&fixture.store, &fixture.writer, &operation)?;
            let before = retained_state(&fixture.store)?;
            let head = fixture.writer.head(&fixture.store)?;
            for _ in 0..2 {
                let refused = fixture.writer.apply_reviewed_identity(
                    &fixture.store,
                    &operation,
                    &changed,
                    fixture.store.snapshot().revision(),
                    &head,
                );
                check!(matches!(
                    refused,
                    Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
                        "reviewed affected record inventory"
                    )))
                ));
                check!(eq; retained_state(&fixture.store)?, before);
                check!(eq; fixture.writer.head(&fixture.store)?, head);
            }
        }
    }
    Ok(())
}

#[test]
fn frozen_reviewed_keys_remain_bound_after_history_request_witness_receipt_and_bundle_rehash(
) -> TestResult {
    for split in [false, true] {
        let fixture = graph_fixture::ready()?;
        let request = if split {
            graph_fixture::split(&fixture)?
        } else {
            graph_fixture::revoke(&fixture)
        };
        let (accepted, frozen) = apply(&fixture, "graph-reviewed-tamper", &request)?;
        for prefix in ["performances:", "relay_results:"] {
            let mut changed = request.clone();
            keys_mut(&mut changed)?.retain(|key| !key.starts_with(prefix));
            let tampered = rehash_request(&accepted, &frozen, &changed)?;
            let digest = format!("{:x}", Sha256::digest(&tampered));
            check!(matches!(
                crate::replay_frozen_public_identity_v2(&tampered, &digest),
                Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
                    "reviewed affected record inventory"
                )))
            ));
        }
    }
    Ok(())
}

fn apply(
    fixture: &Fixture,
    operation: &str,
    request: &ReviewedIdentityRequestV1,
) -> TestResult<(AcceptedPublicIdentitySnapshot, FrozenPublicIdentityBundleV2)> {
    let facts = graph_fixture::facts(&fixture.store)?;
    let before = fixture
        .store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("snapshot")?;
    let outcome = fixture.writer.apply_reviewed_identity(
        &fixture.store,
        operation,
        request,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    check!(eq; outcome.head().sequence, before.head().sequence.checked_add(1).ok_or("head")?);
    let expected_births = if matches!(request, ReviewedIdentityRequestV1::Split { .. }) {
        3
    } else {
        0
    };
    check!(eq; outcome.introduced_public_ids().len(), expected_births);
    check!(!outcome.replayed());
    check!(eq; graph_fixture::facts(&fixture.store)?, facts);
    let state = retained_state(&fixture.store)?;
    let retry = fixture.writer.apply_reviewed_identity(
        &fixture.store,
        operation,
        request,
        fixture.store.snapshot().revision(),
        outcome.head(),
    )?;
    check!(retry.replayed());
    check!(eq; retry.head(), outcome.head());
    check!(eq; retry.introduced_public_ids(), outcome.introduced_public_ids());
    check!(eq; retained_state(&fixture.store)?, state);
    let accepted = fixture
        .store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("snapshot")?;
    let held = accepted.history_records().last().ok_or("correction")?;
    let keys = match &held.request {
        HistoryRequest::Split(row) => &row.affected_record_keys,
        HistoryRequest::Revoke(row) => &row.affected_record_keys,
        _ => return Err("correction history kind".into()),
    };
    check!(eq; keys, &fixture.affected);
    let raw = fixture
        .store
        .journal
        .get(crate::public_identity::witness::key(held.sequence))?
        .ok_or("witness")?;
    let witness = HeldGraphWitness::decode("correction", &raw)?;
    check!(eq; witness.version, 9);
    assert_later_pages(fixture, &witness.graph)?;
    let frozen = fixture.store.snapshot().freeze_public_identity_v2()?;
    accepted.require_same_projection(&frozen.replay()?)?;
    Ok((accepted, frozen))
}

fn assert_later_pages(
    fixture: &Fixture,
    reference: &crate::public_identity::graph_reference::GraphAuthorityReferenceV1,
) -> TestResult {
    use crate::public_identity::paged_generation::{AuthorityTableV2, VerifiedRawGraph};
    let snapshot = fixture.store.snapshot();
    let view = VerifiedRawGraph::verify(&snapshot, reference.capture(), reference.seal())?;
    let mut performances = Vec::new();
    let mut relays = Vec::new();
    view.for_each_physical::<census_domain::model::CanonicalPerformance>(
        AuthorityTableV2::Performances,
        context::BUDGET,
        |row, key, raw, ordinal, _| {
            performances.push((row, key.to_vec(), raw.to_vec(), ordinal));
            Ok(())
        },
    )?;
    view.for_each_physical::<census_domain::model::RelayResult>(
        AuthorityTableV2::RelayResults,
        context::BUDGET,
        |row, key, raw, ordinal, _| {
            relays.push((row, key.to_vec(), raw.to_vec(), ordinal));
            Ok(())
        },
    )?;
    check!(eq; performances.len(), 521);
    check!(eq; relays.len(), 521);
    for (expected, (_, key, raw, ordinal)) in performances.iter().enumerate() {
        check!(eq; *ordinal, u64::try_from(expected)?);
        check!(!key.is_empty());
        check!(!raw.is_empty());
    }
    for (expected, (_, key, raw, ordinal)) in relays.iter().enumerate() {
        check!(eq; *ordinal, u64::try_from(expected)?);
        check!(!key.is_empty());
        check!(!raw.is_empty());
    }
    check!(eq; performances.last().ok_or("last performance")?.0.athlete, fixture.members[0].id);
    check!(eq; relays.last().ok_or("last relay")?.0.members.first().ok_or("relay member")?.athlete.as_ref(), Some(&fixture.members[2].id));
    Ok(())
}

fn finish_offline(
    fixture: Fixture,
    accepted: AcceptedPublicIdentitySnapshot,
    frozen: FrozenPublicIdentityBundleV2,
) -> TestResult {
    let path = fixture.directory.path().join("identity");
    drop(fixture.writer);
    drop(fixture.store);
    let (store, writer) = crate::Store::open_public_identity_qualification_v3(&path)?.into_parts();
    accepted.require_same_projection(
        &store
            .snapshot()
            .accepted_public_identity_snapshot()?
            .ok_or("snapshot")?,
    )?;
    check!(eq; writer.head(&store)?, *accepted.head());
    drop(writer);
    drop(store);
    drop(fixture.directory);
    accepted.require_same_projection(&frozen.replay()?)?;
    Ok(())
}

fn keys_mut(request: &mut ReviewedIdentityRequestV1) -> TestResult<&mut Vec<String>> {
    match request {
        ReviewedIdentityRequestV1::Split {
            affected_record_keys,
            ..
        }
        | ReviewedIdentityRequestV1::Revoke {
            affected_record_keys,
            ..
        } => Ok(affected_record_keys),
        _ => Err("correction request".into()),
    }
}

fn rehash_request(
    accepted: &AcceptedPublicIdentitySnapshot,
    frozen: &FrozenPublicIdentityBundleV2,
    changed: &ReviewedIdentityRequestV1,
) -> TestResult<Vec<u8>> {
    let mut records = accepted.history_records().to_vec();
    let old = records.pop().ok_or("correction record")?;
    let mut history_request = old.request.clone();
    let keys = match changed {
        ReviewedIdentityRequestV1::Split {
            affected_record_keys,
            ..
        }
        | ReviewedIdentityRequestV1::Revoke {
            affected_record_keys,
            ..
        } => affected_record_keys,
        _ => return Err("correction".into()),
    };
    match &mut history_request {
        HistoryRequest::Split(row) => row.affected_record_keys = keys.clone(),
        HistoryRequest::Revoke(row) => row.affected_record_keys = keys.clone(),
        _ => return Err("history correction".into()),
    }
    let prepared = prepare_next(
        &old.installation,
        &old.previous,
        history_request,
        ReplayLimits::SMALL,
    )?;
    records.push(prepared.uncommitted_record.clone());
    let mut bundle: Value = serde_json::from_slice(frozen.bytes())?;
    bundle["history"] = serde_json::to_string(&records)?.into();
    bundle["head"] = serde_json::to_string(&prepared.next_head)?.into();
    let operation = bundle
        .get_mut("operations")
        .and_then(Value::as_array_mut)
        .ok_or("operations")?
        .last_mut()
        .ok_or("operation")?;
    let mut witness = HeldGraphWitness::decode(
        "changed correction",
        operation
            .get("witness")
            .and_then(Value::as_str)
            .ok_or("witness")?
            .as_bytes(),
    )?;
    witness.record_digest = prepared.next_head.digest;
    witness.request_digest = crate::public_identity::correction::request::digest(
        &old.installation,
        &witness.operation,
        changed,
    )?;
    let mut receipt: crate::Receipt = serde_json::from_str(
        operation
            .get("receipt")
            .and_then(Value::as_str)
            .ok_or("receipt")?,
    )?;
    receipt.digest = census_domain::model::serialized_digest(&witness)?;
    operation["witness"] = serde_json::to_string(&witness)?.into();
    operation["receipt"] = serde_json::to_string(&receipt)?.into();
    Ok(serde_json::to_vec(&bundle)?)
}
