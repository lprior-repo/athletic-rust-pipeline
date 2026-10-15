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
use super::physical::{Rows, State};
use super::TestResult;

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
    let (key, raw) = sole_added(fixture.before.rows("journal")?, state.rows("journal")?)?;
    check!(eq; key, b"public_identity_v1:00000000000000000001");
    let witness: serde_json::Value = serde_json::from_slice(raw)?;
    let object = witness.as_object().ok_or("witness is not an object")?;
    check!(eq; object.len(), 11);
    for (name, expected) in [
        ("version", serde_json::json!(1)),
        ("operation", serde_json::json!(OPERATION)),
        ("candidate", serde_json::json!(fixture.candidate)),
        ("request_digest", serde_json::json!(request_digest)),
        ("evidence_generation", serde_json::json!(fixture.evidence)),
        ("derived_generation", serde_json::json!(fixture.derived)),
        ("snapshot_sequence", serde_json::json!(snapshot_sequence)),
        ("record_digest", serde_json::json!(outcome.head().digest)),
        ("decision_id", serde_json::json!(decision.id)),
        (
            "decision_digest",
            serde_json::json!(serialized_digest(decision)?),
        ),
        ("public_id", serde_json::json!(outcome.public_id().as_str())),
    ] {
        check!(eq; object.get(name), Some(&expected));
    }
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

fn sole_added<'a>(before: &Rows, after: &'a Rows) -> TestResult<(&'a [u8], &'a [u8])> {
    check!(eq; after.len(), before.len().checked_add(1).ok_or("fixture row count overflow")?);
    for (key, value) in before {
        check!(eq; after.get(key), Some(value));
    }
    let (key, value) = after
        .iter()
        .find(|(key, _)| !before.contains_key(*key))
        .ok_or("one new physical row was not found")?;
    Ok((key, value))
}
