use super::*;

fn triplet(status: &str, unresolved: &str, mark: Mark) -> TestResult<[CanonicalPerformance; 3]> {
    let [measured, terminal] = status_pair(status, mark)?;
    let mut unknown = measured.clone();
    unknown.mark = Mark::Raw(unresolved.into());
    unknown.timing = None;
    unknown.evidence = vec![Evidence::parsed(
        SourceRef::id("unresolved-observation"),
        "2026-05-01",
    )];
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

fn check_triplet_conflict(
    merged: &CanonicalPerformance,
    original: &[CanonicalPerformance; 3],
) -> TestResult {
    check!(eq; merged.retained_conflicts.len(), 1);
    let conflict = merged
        .retained_conflicts
        .first()
        .ok_or("terminal/numeric conflict survives an unresolved predecessor")?;
    check!(eq; conflict.family, "Individual result scalar");
    check!(eq; conflict.subject_id, merged.id.as_str());
    check!(conflict.detail.contains(&format!("{:?}", original[1].mark)));
    check!(conflict.detail.contains(&format!("{:?}", original[2].mark)));
    check!(merged.mark == original[1].mark || merged.mark == original[2].mark);
    check!(eq; merged.evidence.len(), 3);
    original
        .iter()
        .flat_map(|row| &row.evidence)
        .try_for_each(|evidence| -> TestResult {
            check!(merged.evidence.contains(evidence));
            Ok(())
        })
}

#[test]
fn every_result_status_triplet_permutation_retains_conflict_during_canonical_replay() -> TestResult
{
    for status in ["DQ", "DNF", "DNS", "NH", "FOUL", "NM", "NT", "SCR"] {
        for unresolved in ["unknown published mark", "X", "--"] {
            for mark in numeric_marks()? {
                let original = triplet(status, unresolved, mark)?;
                let mut reference = original[1].clone();
                reference.merge(original[2].clone());
                for [first, second, third] in permutations(&original) {
                    let mut merged = first.clone();
                    merged.merge(second.clone());
                    merged.merge(third.clone());
                    check_triplet_conflict(&merged, &original)?;
                    check!(eq; merged.retained_conflicts, reference.retained_conflicts);
                    let expected = merged.clone();
                    merged.merge(first);
                    merged.merge(second);
                    merged.merge(third);
                    check!(eq; merged, expected);
                }
            }
        }
    }
    Ok(())
}

fn check_physical_triplet(
    store: &Store,
    original: &[CanonicalPerformance; 3],
    physical_count: usize,
) -> TestResult {
    let canonical: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
    check!(eq; canonical.len(), 1);
    check_triplet_conflict(canonical.first().ok_or("merged triplet")?, original)?;
    let mut physical = Vec::new();
    store
        .snapshot()
        .for_each_observation(Table::Performances, |row: CanonicalPerformance| {
            physical.push(row);
            Ok(())
        })?;
    check!(eq; physical.len(), physical_count);
    original.iter().try_for_each(|row| -> TestResult {
        check!(physical.contains(row));
        Ok(())
    })
}

#[test]
fn stored_result_status_triplets_keep_original_observations_after_replay_and_reopen() -> TestResult
{
    for status in ["DQ", "DNF", "DNS", "NH", "FOUL", "NM", "NT", "SCR"] {
        for unresolved in ["unknown published mark", "X", "--"] {
            let original = triplet(
                status,
                unresolved,
                Mark::TimeSeconds(ExactSeconds::parse("10.941")?),
            )?;
            for order in permutations(&original) {
                let directory = tempfile::tempdir()?;
                let store = Store::open(directory.path())?;
                store.append_many(Table::Performances, &order)?;
                check_physical_triplet(&store, &original, 3)?;
                let before: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
                store.append_many(Table::Performances, &order)?;
                check_physical_triplet(&store, &original, 6)?;
                check!(eq; store.scan::<CanonicalPerformance>(Table::Performances)?, before);
                store.flush()?;
                drop(store);
                let reopened = Store::open(directory.path())?;
                check_physical_triplet(&reopened, &original, 6)?;
                check!(eq; reopened.scan::<CanonicalPerformance>(Table::Performances)?, before);
            }
        }
    }
    Ok(())
}

#[test]
fn terminal_status_refines_unresolved_raw_and_never_downgrades_in_either_order() -> TestResult {
    for status in ["DQ", "DNF", "DNS", "NH", "FOUL", "NM", "NT", "SCR"] {
        for unresolved in ["unknown published mark", "X", "--"] {
            let [unknown, terminal, _] = triplet(
                status,
                unresolved,
                Mark::TimeSeconds(ExactSeconds::parse("10.941")?),
            )?;
            for [first, second] in orders(&[unknown, terminal.clone()]) {
                let mut merged = first.clone();
                merged.merge(second.clone());
                check!(eq; merged.mark, terminal.mark);
                check!(eq; merged.retained_conflicts, Vec::new());
                check!(eq; merged.evidence.len(), 2);
                let expected = merged.clone();
                merged.merge(first);
                merged.merge(second);
                check!(eq; merged, expected);
            }
        }
    }
    Ok(())
}
