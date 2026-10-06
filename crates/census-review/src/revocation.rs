use std::collections::HashMap;

use census_domain::model::{ReviewCase, ReviewState, ReviewVerdictRecord};
use census_store::{Application, Store, StoreResult, Table};
use futures::stream::{self, TryStreamExt};
use serde::Serialize;

use crate::consensus::{invariant, lane_binding, replay_matches, POLICY};
use crate::packets::SubjectIndex;
use crate::review_budget::Budget;
use crate::{ModelClient, ReviewFamily, ReviewOptions, RULE_REVIEWER};

const JOURNAL: &str = "review_invalidated_advice_v1";

#[derive(Serialize)]
struct RevocationAudit<'a> {
    policy: &'static str,
    outcome: &'static str,
    prior_record_digest: &'a str,
}

fn active_model_advice(row: &ReviewVerdictRecord) -> bool {
    row.accepted && row.reviewer != RULE_REVIEWER
}

fn revoked_row(row: &ReviewVerdictRecord, digest: &str) -> StoreResult<ReviewVerdictRecord> {
    let rationale = serde_json::to_string(&RevocationAudit {
        policy: POLICY,
        outcome: "standing_advice_invalidated",
        prior_record_digest: digest,
    })
    .map_err(|error| invariant(format!("cannot encode advice revocation: {error}")))?;
    Ok(ReviewVerdictRecord {
        id: row.id.clone(),
        case_id: row.case_id.clone(),
        subject_id: row.subject_id.clone(),
        family: row.family.clone(),
        kind: "insufficient_evidence".to_string(),
        field: String::new(),
        value: String::new(),
        accepted: false,
        confidence: 0,
        rationale,
        reviewer: row.reviewer.clone(),
        observed_at: row.observed_at.clone(),
        member_ids: row.member_ids.clone(),
    })
}

fn current_advice(
    row: &ReviewVerdictRecord,
    case: &ReviewCase,
    family: ReviewFamily,
    subjects: &SubjectIndex,
    clients: [&ModelClient; 2],
) -> StoreResult<bool> {
    if !crate::review_checkpoint::matches_case(row, case) {
        return Ok(false);
    }
    let packet = subjects.packet(case, family)?;
    let lanes = clients.map(|client| lane_binding(client, packet.as_ref()));
    let [first, second] = lanes;
    Ok(replay_matches(row, packet.as_ref(), &[first?, second?]))
}

fn read_active_advice(
    snapshot: &census_store::StoreSnapshot<'_>,
    chunk: &[(ReviewCase, ReviewFamily)],
    budget: &mut Budget,
) -> StoreResult<HashMap<String, ReviewVerdictRecord>> {
    let ids: std::collections::HashSet<_> =
        chunk.iter().map(|(case, _)| case.id.as_str()).collect();
    let mut standing = HashMap::new();
    snapshot.for_each_merged(Table::IdentityVerdicts, |row: ReviewVerdictRecord| {
        if ids.contains(row.id.as_str()) && active_model_advice(&row) {
            budget.charge(&row)?;
            standing.insert(row.id.clone(), row);
        }
        Ok(())
    })?;
    Ok(standing)
}

fn invalidate_chunk(
    store: &Store,
    chunk: &[(ReviewCase, ReviewFamily)],
    clients: [&ModelClient; 2],
    observed_at: &str,
    checkpoint: usize,
) -> StoreResult<()> {
    let snapshot = store.snapshot();
    let mut budget = Budget::default();
    let standing = read_active_advice(&snapshot, chunk, &mut budget)?;
    let selected: Vec<_> = chunk
        .iter()
        .filter(|(case, _)| standing.get(&case.id).is_some_and(active_model_advice))
        .cloned()
        .collect();
    if selected.is_empty() {
        return Ok(());
    }
    let subjects = SubjectIndex::read(&snapshot, &selected, &mut budget)?;
    let current_cases = crate::review_subjects::current_cases(&snapshot, &selected, &mut budget)?;
    let mut batch = store.write_batch();
    let mut verdicts = Vec::new();
    let mut states = Vec::new();
    selected.iter().try_for_each(|(selected, family)| {
        let row = standing
            .get(&selected.id)
            .ok_or_else(|| invariant("standing advice disappeared before invalidation"))?;
        let case = current_cases
            .get(&selected.id)
            .ok_or_else(|| invariant("review case disappeared before advice invalidation"))?;
        if case.state == ReviewState::Superseded
            || current_advice(row, case, *family, &subjects, clients)?
        {
            return Ok(());
        }
        let digest = census_domain::model::serialized_digest(row)
            .map_err(|error| invariant(error.to_string()))?;
        batch.journal_done(JOURNAL, &digest, row)?;
        let revoked = revoked_row(row, &digest)?;
        budget.charge(&revoked)?;
        verdicts.push(revoked);
        let mut retained = case.clone();
        retained.state = ReviewState::Retained;
        budget.charge(&retained)?;
        states.push(retained);
        Ok::<_, census_store::StoreError>(())
    })?;
    commit_revocations(
        batch,
        &verdicts,
        &states,
        observed_at,
        checkpoint,
        snapshot.evidence_generation(),
    )
}

fn commit_revocations(
    mut batch: census_store::StoreBatch<'_>,
    verdicts: &[ReviewVerdictRecord],
    states: &[ReviewCase],
    observed_at: &str,
    checkpoint: usize,
    generation: u64,
) -> StoreResult<()> {
    if verdicts.is_empty() {
        return Ok(());
    }
    let digest = crate::compute_digest(verdicts, states)?;
    let operation = format!("review-revoke:{observed_at}:{checkpoint}:{generation}:{digest}");
    batch.replace_many(Table::IdentityVerdicts, verdicts)?;
    batch.replace_many(Table::ReviewCases, states)?;
    match batch.commit_once_at_evidence_generation(&operation, &digest, generation)? {
        Application::Written(_) => Ok(()),
        Application::Repeated(_) => Err(invariant(
            "fresh advice invalidation replayed a committed transition",
        )),
    }
}

pub(super) async fn invalidate(
    store: &Store,
    options: &ReviewOptions,
    clients: [&ModelClient; 2],
    observed_at: &str,
) -> StoreResult<()> {
    stream::try_unfold(
        (None::<String>, 0_usize),
        |(after, checkpoint)| async move {
            let intake =
                crate::review_intake::next_for_revocation(store, options, after.as_deref())?;
            let Some(cursor) = intake.cursor else {
                return Ok::<_, census_store::StoreError>(None);
            };
            invalidate_chunk(store, &intake.cases, clients, observed_at, checkpoint)?;
            let next = checkpoint
                .checked_add(1)
                .ok_or(census_store::StoreError::CounterOverflow)?;
            Ok(Some(((), (Some(cursor), next))))
        },
    )
    .try_fold((), |(), ()| futures::future::ready(Ok(())))
    .await
}
