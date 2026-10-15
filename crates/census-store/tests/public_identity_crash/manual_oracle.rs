use census_domain::public_identity_history::{prepare_next, CandidateHistorySeed, ReplayLimits};
use census_store::{LocalAdjudicationCommitOutcomeV1, Receipt};
use serde::de::DeserializeSeed;
use sha2::{Digest, Sha256};

use super::context;
use super::fixture::{Approval, Fixture, OPERATION};
use super::physical::{Rows, State};
use super::TestResult;

pub(super) fn committed(
    fixture: &Fixture,
    state: &State,
    approval: &Approval,
    outcome: &LocalAdjudicationCommitOutcomeV1,
    prepared_journal: &Rows,
) -> TestResult<String> {
    let record = &approval.prepared;
    check!(eq; record.version, 2);
    check!(eq; record.policy, "local-operator-retained-identity-review/v2");
    check!(eq; record.store, fixture.plan.identity);
    check!(eq; record.operation, OPERATION);
    check!(eq; record.expected_head, fixture.plan.head);
    check!(eq; record.request, fixture.plan.request);
    check!(eq; record.evidence_generation, fixture.evidence);
    check!(eq; record.derived_generation, fixture.derived);
    check!(eq; state.rows("entities")?, fixture.before.rows("entities")?);
    let claims = record
        .request
        .dispositions
        .iter()
        .map(|row| row.claim.clone())
        .collect::<Vec<_>>();
    let prepared = prepare_next(
        &record.store,
        &record.expected_head,
        record.history_request(&claims)?,
        ReplayLimits::SMALL,
    )?;
    check!(eq; outcome.head(), &prepared.next_head);
    check!(eq; outcome.introduced_public_ids(), prepared.established_ids);
    let mut parser = serde_json::Deserializer::from_slice(
        fixture
            .before
            .rows("meta")?
            .get(b"public_identity:v1:history".as_slice())
            .ok_or("prior history")?,
    );
    let (mut history, _) = CandidateHistorySeed::new(
        &fixture.plan.identity,
        &fixture.plan.head,
        ReplayLimits::SMALL,
    )
    .deserialize(&mut parser)?
    .into_parts();
    parser.end()?;
    check!(eq; history.len(), 4);
    history.push(prepared.uncommitted_record);
    let mut meta = fixture.before.rows("meta")?.clone();
    meta.insert(
        b"public_identity:v1:head".to_vec(),
        serde_json::to_vec(outcome.head())?,
    );
    meta.insert(
        b"public_identity:v1:history".to_vec(),
        serde_json::to_vec(&history)?,
    );
    meta.insert(
        b"generation:entities".to_vec(),
        fixture
            .evidence
            .checked_add(1)
            .ok_or("evidence generation overflow")?
            .to_string()
            .into_bytes(),
    );
    check!(eq; state.rows("meta")?, &meta);
    witness_and_receipt(fixture, state, approval, outcome, prepared_journal)
}

fn witness_and_receipt(
    fixture: &Fixture,
    state: &State,
    approval: &Approval,
    outcome: &LocalAdjudicationCommitOutcomeV1,
    prepared_journal: &Rows,
) -> TestResult<String> {
    let key = b"public_identity_v1:00000000000000000005";
    let journal = state.rows("journal")?;
    let raw = journal.get(key.as_slice()).ok_or("manual witness5")?;
    let (_, request_digest) = approval.encoded()?;
    let expected = serde_json::json!({
        "version": 5, "operation": OPERATION, "actor": "local_operator",
        "request_digest": request_digest, "adjudication": approval.prepared,
        "record_digest": outcome.head().digest,
    });
    check!(eq; serde_json::from_slice::<serde_json::Value>(raw)?, expected);
    let mut expected_journal =
        context::extend(fixture, state, &approval.prepared, prepared_journal)?;
    check!(expected_journal.insert(key.to_vec(), raw.clone()).is_none());
    check!(eq; journal, &expected_journal);
    let digest = format!("{:x}", Sha256::digest(raw));
    let operation = format!("public_identity_v1:{OPERATION}");
    let receipt = Receipt {
        operation: operation.clone(),
        digest: digest.clone(),
        at: approval.prepared.observed_at.clone(),
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

pub(super) fn prepared_journal(
    fixture: &Fixture,
    state: &State,
    approval: &Approval,
) -> TestResult<Rows> {
    context::extend(
        fixture,
        state,
        &approval.prepared,
        fixture.before.rows("journal")?,
    )
}

pub(super) fn staged(fixture: &Fixture, state: &State, journal: &Rows) -> TestResult {
    for name in ["entities", "meta", "receipts"] {
        check!(eq; state.rows(name)?, fixture.before.rows(name)?);
    }
    check!(eq; state.rows("journal")?, journal);
    Ok(())
}
