use std::collections::HashMap;

use census_domain::model::{ReviewCase, ReviewState, ReviewVerdictRecord};
use census_store::{Application, Store, StoreResult, Table};

use crate::consensus::{decide, lane_binding, replay_matches, Asked};
use crate::packets::SubjectIndex;
use crate::review_budget::Budget;
use crate::{ModelClient, ReviewFamily, ReviewReport};

pub(super) fn needs_advice(
    case: &ReviewCase,
    family: ReviewFamily,
    subjects: &SubjectIndex,
    clients: [&ModelClient; 2],
    standing: &HashMap<String, ReviewVerdictRecord>,
) -> StoreResult<bool> {
    let Some(row) = standing.get(&case.id).filter(|row| matches_case(row, case)) else {
        return Ok(true);
    };
    if case.state == ReviewState::Resolved && row.accepted && row.reviewer == crate::RULE_REVIEWER {
        return Ok(false);
    }
    let packet = subjects.packet(case, family)?;
    let [first, second] = clients;
    let lanes = [
        lane_binding(first, packet.as_ref())?,
        lane_binding(second, packet.as_ref())?,
    ];
    let expected = if row.accepted {
        ReviewState::Resolved
    } else {
        ReviewState::Retained
    };
    Ok(case.state != expected || !replay_matches(row, packet.as_ref(), &lanes))
}

pub(super) fn matches_case(row: &ReviewVerdictRecord, case: &ReviewCase) -> bool {
    row.id == case.id
        && row.case_id == case.id
        && row.subject_id == case.subject_id
        && row.family == case.family
        && row.member_ids == case.member_ids
}

pub(super) fn charge_asked(budget: &mut Budget, asked: &Asked) -> StoreResult<()> {
    budget.charge(&asked.packet)?;
    budget.charge(&asked.lanes)?;
    if let Some(answers) = &asked.answers {
        for answer in answers {
            match answer {
                crate::ask::Answer::Answered {
                    batch, verdicts, ..
                } => {
                    budget.charge(batch)?;
                    for (verdict, adjudication) in verdicts {
                        budget.charge(verdict)?;
                        budget.add(format!("{adjudication:?}").len())?;
                    }
                }
                crate::ask::Answer::Failed(error) => budget.add(error.to_string().len())?,
            }
        }
    }
    Ok(())
}

pub(super) fn process_answers(
    pending: &[(ReviewCase, ReviewFamily)],
    asked: Vec<(usize, Asked)>,
    current: &SubjectIndex,
    current_cases: &HashMap<String, ReviewCase>,
    observed_at: &str,
    report: &mut ReviewReport,
    budget: &mut Budget,
) -> StoreResult<(Vec<ReviewVerdictRecord>, Vec<ReviewCase>)> {
    asked.into_iter().try_fold(
        (Vec::new(), Vec::new()),
        |(mut rows, mut states), (index, asked)| {
            let (case, family) = pending.get(index).ok_or_else(|| {
                crate::consensus::invariant("review answer has no checkpoint case")
            })?;
            if current_cases
                .get(&case.id)
                .is_some_and(|current| current != case)
            {
                return Err(crate::consensus::invariant(format!(
                    "review case {} changed before checkpoint",
                    case.id
                )));
            }
            let packet = current.packet(case, *family)?;
            budget.charge(&packet)?;
            let consensus = decide(&asked, packet.as_ref());
            let (row, state) =
                crate::records::record_case(case, asked, consensus, observed_at, report)?;
            budget.charge(&row)?;
            budget.charge(&state)?;
            rows.push(row);
            states.push(state);
            Ok((rows, states))
        },
    )
}

pub(super) fn commit(
    store: &Store,
    verdicts: &[ReviewVerdictRecord],
    cases: &[ReviewCase],
    observed_at: &str,
    checkpoint: usize,
    generation: u64,
) -> StoreResult<Application> {
    let mut batch = store.write_batch();
    let digest = crate::compute_digest(verdicts, cases)?;
    let operation = format!("review:{observed_at}:{checkpoint}:{generation}:{digest}");
    verdicts.iter().try_for_each(|row| {
        let key = census_domain::model::serialized_digest(row)
            .map_err(|error| crate::consensus::invariant(error.to_string()))?;
        batch.journal_done("review_advice_v1", &key, row)
    })?;
    batch.replace_many(Table::IdentityVerdicts, verdicts)?;
    batch.replace_many(Table::ReviewCases, cases)?;
    let application = batch.commit_once_at_evidence_generation(&operation, &digest, generation)?;
    if matches!(application, Application::Repeated(_)) {
        verify_repeated(store, verdicts, cases)?;
    }
    Ok(application)
}

pub(super) fn verify_repeated(
    store: &Store,
    verdicts: &[ReviewVerdictRecord],
    cases: &[ReviewCase],
) -> StoreResult<()> {
    let snapshot = store.snapshot();
    let mut budget = Budget::default();
    let ids = cases.iter().map(|case| case.id.as_str()).collect();
    let states: HashMap<String, ReviewCase> =
        crate::review_subjects::selected(&snapshot, Table::ReviewCases, &ids, &mut budget)?;
    let rows: HashMap<String, ReviewVerdictRecord> =
        crate::review_subjects::selected(&snapshot, Table::IdentityVerdicts, &ids, &mut budget)?;
    if cases.iter().any(|case| states.get(&case.id) != Some(case))
        || verdicts.iter().any(|row| rows.get(&row.id) != Some(row))
    {
        return Err(crate::consensus::invariant("repeated review checkpoint differs from durable verdict or case state; explicit projection repair required"));
    }
    Ok(())
}
