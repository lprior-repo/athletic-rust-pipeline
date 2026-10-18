use census_domain::model::{
    serialized_digest, AppliedAthleteIdentity, AthleteIdentityIndex, IdentityApplication,
    IdentityProjectionBuilder,
};
use census_domain::public_identity_history::{
    prepare_next, BindingClaim, CandidateDecisionReference, EffectiveScope, HistoryRequest,
    ReplayLimits, SourceObjectKey,
};
use census_store::{IdentityCommitOutcomeV1, Receipt};
use sha2::{Digest, Sha256};

use super::fixture::{Fixture, OPERATION};
use super::physical::{sole_added, State};
use super::TestResult;

use super::graph;

pub(super) fn staged_delta(fixture: &Fixture, state: &State) -> TestResult {
    for name in ["entities", "meta", "receipts"] {
        check!(eq; state.rows(name)?, fixture.before.rows(name)?);
    }
    let retained = graph::prepared(&fixture.before, state, &fixture.identity, OPERATION)?;
    let expected = retained.journal;
    check!(eq; state.rows("journal")?, &expected);
    Ok(())
}

pub(super) fn retained_graph_unchanged(
    fixture: &Fixture,
    interrupted: &State,
    recovered: &State,
) -> TestResult {
    let interrupted = graph::prepared(&fixture.before, interrupted, &fixture.identity, OPERATION)?;
    let recovered = graph::prepared(&fixture.before, recovered, &fixture.identity, OPERATION)?;
    check!(eq; recovered.graph, interrupted.graph);
    check!(eq; recovered.reservation, interrupted.reservation);
    check!(eq; recovered.journal, interrupted.journal);
    Ok(())
}

pub(super) fn committed_delta(
    fixture: &Fixture,
    state: &State,
    outcome: &IdentityCommitOutcomeV1,
    snapshot_sequence: u64,
) -> TestResult<String> {
    let (decision_key, decision_raw) =
        sole_added(fixture.before.rows("entities")?, state.rows("entities")?)?;
    let decision: AppliedAthleteIdentity = serde_json::from_slice(decision_raw)?;
    verify_decision(fixture, decision_key, decision_raw, &decision)?;
    let request_digest = serialized_digest(&(
        "public-identity-establish-request-v1",
        &fixture.identity,
        OPERATION,
        &fixture.candidate,
    ))?;
    let source = fixture
        .athlete
        .source
        .as_ref()
        .ok_or("fixture source missing")?;
    let evidence = accepted_reference(fixture, &decision)?;
    let prepared = prepare_next(
        &fixture.identity,
        &fixture.genesis,
        HistoryRequest::Establish {
            binding: BindingClaim {
                id: format!("source_binding_v1_{request_digest}"),
                source: SourceObjectKey {
                    namespace: source.namespace.clone(),
                    identifier: source.id.clone(),
                },
                candidate: fixture.candidate.clone(),
                effective: EffectiveScope::Unknown,
                supersedes: None,
            },
            evidence,
        },
        ReplayLimits::SMALL,
    )?;
    check!(eq; outcome.head(), &prepared.next_head);
    check!(eq; prepared.established_ids.as_slice(), std::slice::from_ref(outcome.public_id()));
    let mut expected_meta = fixture.before.rows("meta")?.clone();
    expected_meta.insert(
        b"public_identity:v1:head".to_vec(),
        serde_json::to_vec(&prepared.next_head)?,
    );
    expected_meta.insert(
        b"public_identity:v1:history".to_vec(),
        serde_json::to_vec(std::slice::from_ref(&prepared.uncommitted_record))?,
    );
    expected_meta.insert(b"rows:athlete_identity_decisions".to_vec(), b"1".to_vec());
    expected_meta.insert(
        b"generation:entities".to_vec(),
        fixture
            .evidence
            .checked_add(1)
            .ok_or("fixture generation overflow")?
            .to_string()
            .into_bytes(),
    );
    check!(eq; state.rows("meta")?, &expected_meta);
    verify_witness_and_receipt(
        fixture,
        state,
        outcome,
        &decision,
        &request_digest,
        snapshot_sequence,
    )
}

fn accepted_reference(
    fixture: &Fixture,
    decision: &AppliedAthleteIdentity,
) -> TestResult<CandidateDecisionReference> {
    let mut index = AthleteIdentityIndex::default();
    index.observe(&fixture.athlete)?;
    let builder = IdentityProjectionBuilder::new(index, &[], &[])?;
    let application = builder
        .source_applications(&decision.observed_at)
        .next()
        .ok_or("fixture lacks source application")??;
    let IdentityApplication::Accepted(accepted) = application else {
        return Err("fixture authority refused the retained decision".into());
    };
    check!(eq; accepted.record(), decision);
    Ok(CandidateDecisionReference::from_accepted(
        &accepted,
        ReplayLimits::SMALL,
    )?)
}

fn verify_decision(
    fixture: &Fixture,
    key: &[u8],
    raw: &[u8],
    decision: &AppliedAthleteIdentity,
) -> TestResult {
    let mut expected_key = b"athlete_identity_decisions\0".to_vec();
    expected_key.extend_from_slice(decision.id.as_bytes());
    expected_key.push(0);
    expected_key.extend_from_slice(&0_u64.to_be_bytes());
    check!(eq; key, expected_key.as_slice());
    check!(eq; raw, serde_json::to_vec(decision)?.as_slice());
    check!(eq; decision.members.len(), 1);
    check!(eq; decision.members.first().ok_or("missing accepted member")?.subject, fixture.candidate);
    Ok(())
}

fn verify_witness_and_receipt(
    fixture: &Fixture,
    state: &State,
    outcome: &IdentityCommitOutcomeV1,
    decision: &AppliedAthleteIdentity,
    request_digest: &str,
    snapshot_sequence: u64,
) -> TestResult<String> {
    let retained = graph::prepared(&fixture.before, state, &fixture.identity, OPERATION)?;
    let key = b"public_identity_v1:00000000000000000001";
    let journal = state.rows("journal")?;
    let raw = journal
        .get(key.as_slice())
        .ok_or("missing committed witness")?;
    let mut expected_journal = retained.journal;
    check!(expected_journal.insert(key.to_vec(), raw.clone()).is_none());
    check!(eq; journal, &expected_journal);
    let witness: serde_json::Value = serde_json::from_slice(raw)?;
    check!(eq; retained.reservation["graph"]["capture"]["evidence_generation"], serde_json::json!(fixture.evidence));
    check!(eq; retained.reservation["graph"]["capture"]["derived_generation"], serde_json::json!(fixture.derived));
    check!(eq; retained.reservation["graph"]["capture"]["snapshot_sequence"], serde_json::json!(snapshot_sequence));
    check!(eq; retained.reservation["expected_head"], serde_json::json!(fixture.genesis));
    check!(eq; retained.reservation["request_digest"], serde_json::json!(request_digest));
    let expected = serde_json::json!({
        "version": 8, "operation": OPERATION, "request_digest": request_digest,
        "record_digest": outcome.head().digest, "graph": retained.graph,
        "observed_at": decision.observed_at,
        "payload": { "family": "source", "candidate": fixture.candidate,
            "decision": { "id": decision.id, "digest": serialized_digest(decision)? },
            "public_id": outcome.public_id().as_str() },
    });
    check!(eq; witness, expected);
    check!(eq; retained.reservation["witness"], expected);
    let digest = format!("{:x}", Sha256::digest(raw));
    let (receipt_key, receipt_raw) =
        sole_added(fixture.before.rows("receipts")?, state.rows("receipts")?)?;
    let operation = format!("public_identity_v1:{OPERATION}");
    check!(eq; receipt_key, operation.as_bytes());
    let expected = Receipt {
        operation,
        digest: digest.clone(),
        at: decision.observed_at.clone(),
        appended: 1,
        credited: false,
    };
    check!(eq; receipt_raw, serde_json::to_vec(&expected)?.as_slice());
    Ok(digest)
}
