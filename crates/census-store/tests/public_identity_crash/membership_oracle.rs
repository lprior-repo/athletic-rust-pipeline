use super::fixture::{Fixture, Mode};
use super::physical::{sole_added, State};
use super::TestResult;
use census_domain::model::{
    serialized_digest, AppliedAthleteIdentity, AthleteIdentityIndex, CanonicalAthlete,
    IdentityApplication, IdentityProjectionBuilder, ReviewCase, ReviewVerdictRecord, StoreIdentity,
};
use census_domain::public_identity_history::{
    prepare_next, CandidateDecisionReference, CandidateHistorySeed, CandidateMembershipClaimV1,
    HistoryRequest, RegistryHead, ReplayLimits,
};
use census_store::{PublicIdentityWriterV1, Receipt, Store, Table};
use serde::de::DeserializeSeed;
use sha2::{Digest, Sha256};

use super::super::graph;

pub(super) struct Expected {
    identity: StoreIdentity,
    athletes: Vec<CanonicalAthlete>,
    cases: Vec<ReviewCase>,
    verdicts: Vec<ReviewVerdictRecord>,
    evidence: u64,
    derived: u64,
}

impl Expected {
    pub fn capture(store: &Store, writer: &PublicIdentityWriterV1) -> TestResult<Self> {
        let snapshot = store.snapshot();
        let expected = Self {
            identity: writer.store_identity().clone(),
            athletes: snapshot.scan(Table::Athletes)?,
            cases: snapshot.scan(Table::ReviewCases)?,
            verdicts: snapshot.scan(Table::IdentityVerdicts)?,
            evidence: snapshot.evidence_generation(),
            derived: snapshot.derived_generation(),
        };
        check!(eq; expected.athletes.len(), 2);
        check!(eq; expected.cases.len(), 1);
        check!(eq; expected.verdicts.len(), 1);
        Ok(expected)
    }
}

pub(super) fn staged(fixture: &Fixture, state: &State) -> TestResult {
    for name in ["entities", "meta", "receipts"] {
        check!(eq; state.rows(name)?, fixture.before.rows(name)?);
    }
    let journal = graph::prepared(
        &fixture.before,
        state,
        &fixture.expected.identity,
        fixture.plan.operation(),
    )?
    .journal;
    check!(eq; state.rows("journal")?, &journal);
    Ok(())
}

pub(super) fn committed(
    fixture: &Fixture,
    state: &State,
    mode: Mode,
    head: &RegistryHead,
    snapshot_sequence: u64,
) -> TestResult<String> {
    let expected = &fixture.expected;
    let prior = fixture.plan.prior_head();
    let mut parser = serde_json::Deserializer::from_slice(
        fixture
            .before
            .rows("meta")?
            .get(b"public_identity:v1:history".as_slice())
            .ok_or("prior history")?,
    );
    let (mut history, _) =
        CandidateHistorySeed::new(&expected.identity, &prior, ReplayLimits::SMALL)
            .deserialize(&mut parser)?
            .into_parts();
    parser.end()?;
    let retained = graph::prepared(
        &fixture.before,
        state,
        &expected.identity,
        fixture.plan.operation(),
    )?;
    check!(eq; retained.reservation["graph"]["capture"]["evidence_generation"], serde_json::json!(expected.evidence));
    check!(eq; retained.reservation["graph"]["capture"]["derived_generation"], serde_json::json!(expected.derived));
    check!(eq; retained.reservation["expected_head"], serde_json::json!(prior));
    let mut journal = retained.journal;
    let (request, at, witness) = match mode {
        Mode::Attach => attachment(
            fixture,
            state,
            &history,
            head,
            &retained.graph,
            snapshot_sequence,
        )?,
        Mode::Withhold => {
            check!(eq; state.rows("entities")?, fixture.before.rows("entities")?);
            let record = fixture.plan.withholding().ok_or("withholding plan")?;
            let claims = record
                .dispositions
                .iter()
                .map(|row| row.claim.clone())
                .collect::<Vec<_>>();
            let (_, digest) = super::fixture::approval(record, fixture.plan.withholding_graph()?)?;
            (
                record.history_request(&claims)?,
                record.observed_at.clone(),
                serde_json::json!({
                    "version": 12, "operation": fixture.plan.operation(),
                    "request_digest": digest, "graph": fixture.plan.withholding_graph()?,
                    "observed_at": record.observed_at,
                    "payload": { "family": "withholding", "actor": "local_operator", "adjudication": record },
                    "record_digest": head.digest,
                }),
            )
        }
    };
    check!(eq; retained.reservation["witness"], witness);
    let prepared = prepare_next(&expected.identity, &prior, request, ReplayLimits::SMALL)?;
    check!(prepared.established_ids.is_empty());
    check!(eq; head, &prepared.next_head);
    history.push(prepared.uncommitted_record);
    let mut meta = fixture.before.rows("meta")?.clone();
    meta.insert(
        b"public_identity:v1:head".to_vec(),
        serde_json::to_vec(head)?,
    );
    meta.insert(
        b"public_identity:v1:history".to_vec(),
        serde_json::to_vec(&history)?,
    );
    meta.insert(
        b"generation:entities".to_vec(),
        expected
            .evidence
            .checked_add(1)
            .ok_or("generation overflow")?
            .to_string()
            .into_bytes(),
    );
    if matches!(mode, Mode::Attach) {
        meta.insert(b"rows:athlete_identity_decisions".to_vec(), b"2".to_vec());
    }
    check!(eq; state.rows("meta")?, &meta);
    let key = format!("public_identity_v1:{:020}", head.sequence).into_bytes();
    let raw = state
        .rows("journal")?
        .get(&key)
        .ok_or("operation witness")?;
    check!(eq; serde_json::from_slice::<serde_json::Value>(raw)?, witness);
    check!(journal.insert(key, raw.clone()).is_none());
    check!(eq; state.rows("journal")?, &journal);
    let digest = format!("{:x}", Sha256::digest(raw));
    let operation = format!("public_identity_v1:{}", fixture.plan.operation());
    let receipt = Receipt {
        operation: operation.clone(),
        digest: digest.clone(),
        at,
        appended: 1,
        credited: false,
    };
    let mut receipts = fixture.before.rows("receipts")?.clone();
    check!(receipts
        .insert(operation.into_bytes(), serde_json::to_vec(&receipt)?)
        .is_none());
    check!(eq; state.rows("receipts")?, &receipts);
    Ok(digest)
}

fn attachment(
    fixture: &Fixture,
    state: &State,
    history: &[census_domain::public_identity_history::HistoryRecord],
    head: &RegistryHead,
    graph: &census_store::GraphAuthorityReferenceV1,
    snapshot_sequence: u64,
) -> TestResult<(HistoryRequest, String, serde_json::Value)> {
    let expected = &fixture.expected;
    check!(eq; serde_json::to_value(graph.capture())?["snapshot_sequence"], serde_json::json!(snapshot_sequence));
    let (key, raw) = sole_added(fixture.before.rows("entities")?, state.rows("entities")?)?;
    let decision: AppliedAthleteIdentity = serde_json::from_slice(raw)?;
    let mut decision_key = format!("athlete_identity_decisions\0{}\0", decision.id).into_bytes();
    decision_key.extend_from_slice(&0_u64.to_be_bytes());
    check!(eq; key, decision_key.as_slice());
    check!(eq; raw, serde_json::to_vec(&decision)?.as_slice());
    let mut index = AthleteIdentityIndex::default();
    expected
        .athletes
        .iter()
        .try_for_each(|row| index.observe(row))?;
    let builder = IdentityProjectionBuilder::new(index, &expected.cases, &expected.verdicts)?;
    let (_, application) = builder
        .reviewed_applications(&decision.observed_at)
        .find(|(case, _)| case.id == fixture.plan.attachment.case_id)
        .ok_or("reviewed authority")?;
    let IdentityApplication::Accepted(accepted) = application? else {
        return Err("fixture review did not establish accepted authority".into());
    };
    check!(eq; accepted.record(), &decision);
    let digest = serialized_digest(&(
        "public-identity-source-membership-request-v1",
        &expected.identity,
        fixture.plan.operation(),
        serde_json::to_vec(&fixture.plan.attachment)?,
    ))?;
    let HistoryRequest::Establish { binding, .. } = &history.first().ok_or("source birth")?.request
    else {
        return Err("source birth kind".into());
    };
    let request = HistoryRequest::BindCandidateMembership {
        owner: fixture.plan.attachment.owner.clone(),
        membership: CandidateMembershipClaimV1 {
            id: format!("source_membership_v1_{digest}"),
            candidate: fixture.plan.attachment.candidate.clone(),
            anchor_binding_id: fixture.plan.attachment.anchor_binding_id.clone(),
            source: binding.source.clone(),
        },
        evidence: CandidateDecisionReference::from_accepted(&accepted, ReplayLimits::SMALL)?,
    };
    Ok((
        request,
        decision.observed_at.clone(),
        serde_json::json!({
            "version": 11, "operation": fixture.plan.operation(),
            "request_digest": digest, "graph": graph,
            "record_digest": head.digest, "observed_at": decision.observed_at,
            "payload": { "family": "membership", "request": fixture.plan.attachment,
                "decision": { "id": decision.id, "digest": serialized_digest(&decision)? } },
        }),
    ))
}
