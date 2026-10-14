use super::*;

mod triplets;

fn status_pair(
    dataset: &ExportDataset,
    case: &NumericCase,
    status: &str,
) -> [CanonicalPerformance; 2] {
    let measured = reported(
        dataset,
        "terminal_numeric_context",
        case.conflicting[0].clone(),
    );
    let mut terminal = measured.clone();
    terminal.mark = Mark::Raw(status.into());
    terminal.timing = None;
    [measured, terminal]
}

#[test]
fn stored_terminal_numeric_context_has_no_best_in_both_append_orders() -> TestResult {
    for status in ["DQ", "DNF", "DNS", "NH", "FOUL", "NM", "NT", "SCR"] {
        for case in numeric_cases()? {
            let seed = selection_dataset(case.kind.clone())?;
            let pair = status_pair(&seed, &case, status);
            for order in append_orders(&pair) {
                let dataset = reloaded_dataset(&seed, &order)?;
                check!(eq; dataset.performances.len(), 1);
                check!(eq; selected(&dataset)?, Vec::new());
                retained_conflict(&dataset, &pair[0].id)?;
            }
        }
    }
    Ok(())
}

#[test]
fn stored_terminal_numeric_context_uses_clean_fallback_without_duplicate_resurrection() -> TestResult
{
    for status in ["DQ", "DNF", "DNS", "NH", "FOUL", "NM", "NT", "SCR"] {
        for case in numeric_cases()? {
            let seed = selection_dataset(case.kind.clone())?;
            let pair = status_pair(&seed, &case, status);
            let mut duplicate = pair[0].clone();
            duplicate.id = Id::mint("perf", &["duplicate_numeric_in_terminal_context"]);
            duplicate.source_key = "duplicate_numeric_in_terminal_context".into();
            duplicate.source_athlete =
                Some(SourceIdentity::new(SourceNamespace::TfrrsAthlete, "2001"));
            let mut clean = reported(
                &seed,
                "clean_after_terminal_conflict",
                case.fallback.clone(),
            );
            clean.date = "2025-02-28".into();
            for order in append_orders(&pair) {
                let observations = [
                    order[0].clone(),
                    order[1].clone(),
                    duplicate.clone(),
                    clean.clone(),
                ];
                let mut dataset = reloaded_dataset(&seed, &observations)?;
                let rows = selected(&dataset)?;
                dataset.performances.reverse();
                check!(eq; selected(&dataset)?, rows);
                let selected = rows.first().ok_or("clean fallback remains selectable")?;
                check!(eq; rows.len(), 1);
                check!(eq; selected.source.performance_id, clean.id);
                check!(eq; selected.result.value, case.expected);
                check!(eq; selected.conflicts.len(), 1);
                retained_conflict(&dataset, &pair[0].id)?;
            }
        }
    }
    Ok(())
}

#[test]
fn ambiguous_and_unknown_raw_revisions_keep_numeric_best_in_both_append_orders() -> TestResult {
    for raw in [
        "X",
        "x",
        " -- ",
        "24.95a",
        "unknown",
        "DQ.",
        "D Q",
        "DNF 10.941",
    ] {
        let seed = selection_dataset(EventKind::Track400m)?;
        let measured = reported(
            &seed,
            "refinable_raw_context",
            Mark::TimeSeconds(ExactSeconds::parse("50.00")?),
        );
        let mut unresolved = measured.clone();
        unresolved.mark = Mark::Raw(raw.into());
        unresolved.timing = None;
        for order in append_orders(&[measured.clone(), unresolved]) {
            let mut dataset = reloaded_dataset(&seed, &order)?;
            let performance = dataset.performances.first().ok_or("refined result")?;
            check!(eq; performance.mark, measured.mark);
            check!(eq; performance.retained_conflicts, Vec::new());
            let rows = selected(&dataset)?;
            dataset.performances.reverse();
            check!(eq; selected(&dataset)?, rows);
            let selected = rows.first().ok_or("numeric best remains selectable")?;
            check!(eq; rows.len(), 1);
            check!(eq; selected.source.performance_id, measured.id);
            check!(eq; selected.result.value, 50_000_000_000);
            check!(eq; selected.conflicts, Vec::new());
        }
    }
    Ok(())
}
