use census_domain::public_identity_history::local_adjudication::LocalIdentityAdjudicationRecordV3;
use census_domain::public_identity_history::local_membership_withholding::LocalMembershipWithholdingRecordV2;
use census_store::GraphAuthorityReferenceV1;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::controller::{Boundary, Reached};
use super::fixture::{Fixture, Mode};
use super::graph;
use super::physical::State;
use super::TestResult;

pub(super) fn interrupted(
    fixture: &Fixture,
    state: &State,
    boundary: Boundary,
    marker: &Reached,
) -> TestResult<Option<Vec<u8>>> {
    unchanged_authority(&fixture.before, state)?;
    let before = fixture.before.rows("journal")?;
    let journal = state.rows("journal")?;
    graph::verify_accounting(before)?;
    graph::verify_accounting(journal)?;
    let raw_key = graph::reservation_key(&graph::raw_operation(
        &fixture.identity,
        fixture.mode.operation(),
    )?);
    check!(!before.contains_key(&raw_key));
    let raw = graph::row(journal, &raw_key)?;
    check!(eq; raw["purpose"]["capture"]["evidence_generation"], json!(fixture.evidence));
    check!(eq; raw["purpose"]["capture"]["derived_generation"], json!(fixture.derived));
    let preparation_key = graph::reservation_key(fixture.mode.operation());
    check!(!before.contains_key(&preparation_key));
    let prior_header = graph::row(before, graph::HEADER)?;
    let header = graph::row(journal, graph::HEADER)?;
    let added = match boundary {
        Boundary::RawGraphChunk => 1u64,
        Boundary::Preparation => 2,
    };
    check!(eq; header["reservations"].as_u64(), prior_header["reservations"].as_u64().and_then(|value| value.checked_add(added)));
    match boundary {
        Boundary::RawGraphChunk => {
            check!(!journal.contains_key(&preparation_key));
            let root = &marker.operation;
            let descriptors = raw["descriptors"].as_array().ok_or("graph descriptors")?;
            let descriptor = descriptors
                .iter()
                .find(|entry| entry["object"]["root"].as_str() == Some(root.as_str()))
                .ok_or("interrupted graph object")?;
            check!(eq; descriptor["incremental"]["roots"], json!(1));
            let chunk = graph::key("chunk", root, Some(0))?;
            check!(!before.contains_key(&chunk));
            check!(eq; graph::digest(journal.get(&chunk).ok_or("durable graph chunk")?), marker.input_digest);
            check!(!journal.contains_key(&graph::key("index", root, Some(0))?));
            check!(!journal.contains_key(&graph::key("manifest", root, None)?));
            let expected = graph::extend_raw(
                &fixture.before,
                state,
                &fixture.identity,
                fixture.mode.operation(),
                false,
            )?;
            check!(eq; journal, &expected);
            Ok(None)
        }
        Boundary::Preparation => {
            let retained = graph::prepared(
                &fixture.before,
                state,
                &fixture.identity,
                fixture.mode.operation(),
            )?;
            check!(eq; graph::digest(journal.get(&preparation_key).ok_or("durable preparation")?), marker.input_digest);
            check!(eq; journal, &retained.journal);
            Ok(Some(approval(fixture.mode, &retained.reservation)?))
        }
    }
}

pub(super) fn recovered(
    fixture: &Fixture,
    interrupted: &State,
    recovered: &State,
    boundary: Boundary,
    approval_bytes: &[u8],
) -> TestResult {
    unchanged_authority(interrupted, recovered)?;
    let before = interrupted.rows("journal")?;
    let after = recovered.rows("journal")?;
    let retained = graph::prepared(
        &fixture.before,
        recovered,
        &fixture.identity,
        fixture.mode.operation(),
    )?;
    check!(eq; after, &retained.journal);
    check!(eq; serde_json::to_value(retained.graph.domain_reference()?)?, retained.reservation["witness"]["payload"]["adjudication"]["graph"]);
    check!(eq; retained.reservation["expected_head"], json!(fixture.head));
    check!(eq; retained.reservation["witness"]["payload"]["family"], json!(fixture.mode.kind()));
    check!(eq; approval(fixture.mode, &retained.reservation)?.as_slice(), approval_bytes);
    for (key, value) in before
        .iter()
        .filter(|(key, _)| key.as_slice() != graph::HEADER)
    {
        check!(eq; after.get(key), Some(value));
    }
    match boundary {
        Boundary::RawGraphChunk => {
            let prior = graph::row(before, graph::HEADER)?;
            let current = graph::row(after, graph::HEADER)?;
            let key = graph::reservation_key(fixture.mode.operation());
            let raw = after.get(&key).ok_or("new preparation")?;
            let charge = u64::try_from(
                key.len()
                    .checked_add(raw.len())
                    .ok_or("preparation charge")?,
            )?;
            check!(eq; current["bytes"].as_u64(), prior["bytes"].as_u64().and_then(|bytes| bytes.checked_add(charge)));
            check!(eq; current["roots"], prior["roots"]);
            check!(eq; current["reservations"].as_u64(), prior["reservations"].as_u64().and_then(|count| count.checked_add(1)));
        }
        Boundary::Preparation => check!(eq; recovered, interrupted),
    }
    Ok(())
}

#[derive(Serialize)]
struct Approval<'a, T> {
    schema: &'static str,
    actor: &'static str,
    prepared: &'a T,
    graph: &'a GraphAuthorityReferenceV1,
}

fn approval(mode: Mode, reservation: &Value) -> TestResult<Vec<u8>> {
    let graph: GraphAuthorityReferenceV1 = serde_json::from_value(reservation["graph"].clone())?;
    let record = reservation["witness"]["payload"]["adjudication"].clone();
    check!(eq; reservation["witness"]["payload"]["family"], json!(mode.kind()));
    let bytes = match mode {
        Mode::Split => {
            let record: LocalIdentityAdjudicationRecordV3 = serde_json::from_value(record)?;
            check!(eq; record.graph, graph.domain_reference()?);
            serde_json::to_vec(&Approval {
                schema: "local-identity-adjudication-approval/v2",
                actor: "local_operator",
                prepared: &record,
                graph: &graph,
            })?
        }
        Mode::Withhold => {
            let record: LocalMembershipWithholdingRecordV2 = serde_json::from_value(record)?;
            check!(eq; record.graph, graph.domain_reference()?);
            serde_json::to_vec(&Approval {
                schema: "local-source-membership-withholding-approval/v2",
                actor: "local_operator",
                prepared: &record,
                graph: &graph,
            })?
        }
    };
    let schema = match mode {
        Mode::Split => "local-identity-adjudication-approval/v2",
        Mode::Withhold => "local-source-membership-withholding-approval/v2",
    };
    let mut digest = Sha256::new();
    digest.update(schema.as_bytes());
    digest.update([0]);
    digest.update(&bytes);
    check!(eq; reservation["witness"]["request_digest"], json!(format!("{:x}", digest.finalize())));
    Ok(bytes)
}

fn unchanged_authority(before: &State, after: &State) -> TestResult {
    for name in ["entities", "meta", "receipts"] {
        check!(eq; before.rows(name)?, after.rows(name)?);
    }
    Ok(())
}
