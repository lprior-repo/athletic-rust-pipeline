use census_domain::model::{ReviewCase, ReviewVerdictRecord};
use census_store::{Store, StoreResult, Table};
use futures::stream::{self, TryStreamExt};

use crate::consensus::{ask_lanes, Asked};
use crate::packets::SubjectIndex;
use crate::review_budget::Budget;
use crate::review_checkpoint::{charge_asked, commit, matches_case, needs_advice, process_answers};
use crate::{ModelClient, ReviewFamily, ReviewOptions, ReviewReport};

pub(super) async fn process(
    store: &Store,
    chunk: &[(ReviewCase, ReviewFamily)],
    clients: [&ModelClient; 2],
    options: &ReviewOptions,
    observed_at: &str,
    checkpoint: usize,
    report: &mut ReviewReport,
) -> StoreResult<()> {
    let mut budget = Budget::default();
    let asked = ask_checkpoint(
        store,
        chunk,
        clients,
        options.limit.saturating_sub(report.requested),
        &mut budget,
    )
    .await?;
    if asked.is_empty() {
        return Ok(());
    }
    report.requested = report.requested.saturating_add(asked.len());
    let snapshot = store.snapshot();
    let current = SubjectIndex::read(&snapshot, chunk, &mut budget)?;
    let current_cases = crate::review_subjects::current_cases(&snapshot, chunk, &mut budget)?;
    let (verdicts, states) = process_answers(
        chunk,
        asked,
        &current,
        &current_cases,
        observed_at,
        report,
        &mut budget,
    )?;
    if !options.dry_run && !verdicts.is_empty() {
        budget.charge(&verdicts)?;
        budget.charge(&states)?;
        commit(
            store,
            &verdicts,
            &states,
            observed_at,
            checkpoint,
            snapshot.sequence(),
        )?;
    }
    Ok(())
}

async fn ask_checkpoint(
    store: &Store,
    chunk: &[(ReviewCase, ReviewFamily)],
    clients: [&ModelClient; 2],
    remaining: usize,
    budget: &mut Budget,
) -> StoreResult<Vec<(usize, Asked)>> {
    for (case, _) in chunk {
        budget.charge(case)?;
    }
    let snapshot = store.snapshot();
    let ids = chunk.iter().map(|(case, _)| case.id.as_str()).collect();
    let standing = crate::review_subjects::selected::<ReviewVerdictRecord>(
        &snapshot,
        Table::IdentityVerdicts,
        &ids,
        budget,
    )?;
    let subjects = SubjectIndex::read(&snapshot, chunk, budget)?;
    drop(snapshot);
    let intake = chunk
        .iter()
        .enumerate()
        .filter_map(|(index, (case, family))| {
            match needs_advice(case, *family, &subjects, clients, &standing) {
                Ok(false) => None,
                Ok(true) => Some(Ok((index, case, *family))),
                Err(error) => Some(Err(error)),
            }
        })
        .take(remaining);
    stream::iter(intake)
        .and_then(|(index, case, family)| {
            let subjects = &subjects;
            let standing = &standing;
            async move {
                let previous = standing.get(&case.id).filter(|row| matches_case(row, case));
                ask_lanes(case, family, subjects, clients, previous)
                    .await
                    .map(|asked| (index, asked))
            }
        })
        .try_fold(Vec::new(), |mut rows, (index, asked)| {
            let result = charge_asked(budget, &asked).map(|()| {
                rows.push((index, asked));
                rows
            });
            futures::future::ready(result)
        })
        .await
}
