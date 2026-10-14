use super::*;

fn timed_case() -> TestResult<NumericCase> {
    Ok(NumericCase {
        kind: EventKind::Track400m,
        conflicting: [
            Mark::TimeSeconds(ExactSeconds::parse("50.00")?),
            Mark::TimeSeconds(ExactSeconds::parse("51.00")?),
        ],
        fallback: Mark::TimeSeconds(ExactSeconds::parse("52.00")?),
        expected: 52_000_000_000,
    })
}

fn triplet(
    seed: &ExportDataset,
    case: &NumericCase,
    status: &str,
    unresolved: &str,
) -> TestResult<[CanonicalPerformance; 3]> {
    let [measured, terminal] = status_pair(seed, case, status)?;
    let mut unknown = with_mark(seed, measured.clone(), Mark::Raw(unresolved.into()))?;
    unknown.timing = None;
    Ok([unknown, terminal, measured])
}

fn permutations(rows: &[CanonicalPerformance; 3]) -> [[CanonicalPerformance; 3]; 6] {
    [
        [rows[0].clone(), rows[1].clone(), rows[2].clone()],
        [rows[0].clone(), rows[2].clone(), rows[1].clone()],
        [rows[1].clone(), rows[0].clone(), rows[2].clone()],
        [rows[1].clone(), rows[2].clone(), rows[0].clone()],
        [rows[2].clone(), rows[0].clone(), rows[1].clone()],
        [rows[2].clone(), rows[1].clone(), rows[0].clone()],
    ]
}

fn replayed(observations: &[CanonicalPerformance]) -> Vec<CanonicalPerformance> {
    observations.iter().chain(observations).cloned().collect()
}

#[test]
fn every_stored_result_status_triplet_order_has_no_best_before_and_after_replay_and_reopen(
) -> TestResult {
    let case = timed_case()?;
    let seed = numeric_dataset(case.kind.clone())?;
    for status in ["DQ", "DNF", "DNS", "NH", "FOUL", "NM", "NT", "SCR"] {
        for unresolved in ["unknown published mark", "X", "--"] {
            let original = triplet(&seed, &case, status, unresolved)?;
            for order in permutations(&original) {
                let mut first = reloaded_dataset(&seed, &order)?;
                check!(eq; selected(&first)?, Vec::new());
                retained_conflict(&first, &original[1].id)?;
                let canonical = first.performances.clone();
                first.performances.reverse();
                check!(eq; selected(&first)?, Vec::new());

                let mut repeated = reloaded_dataset(&seed, &replayed(&order))?;
                check!(eq; repeated.performances, canonical);
                check!(eq; selected(&repeated)?, Vec::new());
                retained_conflict(&repeated, &original[1].id)?;
                repeated.performances.reverse();
                check!(eq; selected(&repeated)?, Vec::new());
            }
        }
    }
    Ok(())
}

#[test]
fn every_stored_result_status_triplet_order_uses_clean_fallback_before_and_after_replay(
) -> TestResult {
    let case = timed_case()?;
    let seed = numeric_dataset(case.kind.clone())?;
    let mut clean = numeric_report(
        &seed,
        "clean_after_result_status_triplet",
        case.fallback.clone(),
    )?;
    clean.date = "2025-02-28".into();
    for status in ["DQ", "DNF", "DNS", "NH", "FOUL", "NM", "NT", "SCR"] {
        for unresolved in ["unknown published mark", "X", "--"] {
            let original = triplet(&seed, &case, status, unresolved)?;
            let mut duplicate = original[2].clone();
            duplicate.id = Id::mint("perf", &["numeric_duplicate_after_result_status_triplet"]);
            duplicate.source_key = "numeric_duplicate_after_result_status_triplet".into();
            duplicate.source_athlete =
                Some(SourceIdentity::new(SourceNamespace::TfrrsAthlete, "2001"));
            for order in permutations(&original) {
                let mut observations = order.to_vec();
                observations.extend([duplicate.clone(), clean.clone()]);
                let mut first = reloaded_dataset(&seed, &observations)?;
                let rows = selected(&first)?;
                let winner = rows
                    .first()
                    .ok_or("first-pass clean fallback remains selectable")?;
                check!(eq; rows.len(), 1);
                check!(eq; winner.source.performance_id, clean.id);
                check!(eq; winner.result.value, case.expected);
                check!(eq; winner.conflicts.len(), 1);
                retained_conflict(&first, &original[1].id)?;
                let canonical = first.performances.clone();
                first.performances.reverse();
                check!(eq; selected(&first)?, rows);

                let mut repeated = reloaded_dataset(&seed, &replayed(&observations))?;
                check!(eq; repeated.performances, canonical);
                check!(eq; selected(&repeated)?, rows);
                retained_conflict(&repeated, &original[1].id)?;
                repeated.performances.reverse();
                check!(eq; selected(&repeated)?, rows);
            }
        }
    }
    Ok(())
}
