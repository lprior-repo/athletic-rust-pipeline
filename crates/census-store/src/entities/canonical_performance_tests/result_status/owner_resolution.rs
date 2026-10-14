use super::*;

fn owner_resolution_triplet(
    status: &str,
    unresolved: &str,
) -> TestResult<[CanonicalPerformance; 3]> {
    let [measured, terminal] =
        status_pair(status, Mark::TimeSeconds(ExactSeconds::parse("10.941")?))?;
    let mut unknown = measured.clone();
    unknown.mark = Mark::Raw(unresolved.into());
    unknown.timing = None;
    unknown.source_athlete = None;
    unknown.evidence = vec![Evidence::parsed(
        SourceRef::id("unresolved-owner-observation"),
        "2026-05-01",
    )];
    Ok([unknown, terminal, measured])
}

fn check_retained_owner_resolution(row: &CanonicalPerformance) -> TestResult {
    let conflict = row
        .retained_conflicts
        .iter()
        .find(|conflict| conflict.family == "Individual result context")
        .ok_or("missing ownership remains an explicit context conflict")?;
    check!(eq; conflict.subject_id, row.id.as_str());
    check!(eq; conflict.subject, "event, team, source_athlete");
    check!(eq; row.evidence.len(), 3);
    Ok(())
}

#[test]
fn missing_to_present_owner_result_status_triplet_is_a_canonical_replay_fixed_point() -> TestResult
{
    for status in ["DQ", "DNF", "DNS", "NH", "FOUL", "NM", "NT", "SCR"] {
        for unresolved in ["unknown published mark", "X", "--"] {
            let observations = owner_resolution_triplet(status, unresolved)?;
            let mut merged = observations[0].clone();
            merged.merge(observations[1].clone());
            merged.merge(observations[2].clone());
            check_retained_owner_resolution(&merged)?;
            let first_pass = merged.clone();
            for observation in &observations {
                merged.merge(observation.clone());
            }
            check!(eq; merged, first_pass, "{status} after {unresolved}");
        }
    }
    Ok(())
}

#[test]
fn stored_missing_to_present_owner_result_status_triplet_is_stable_after_replay_and_reopen(
) -> TestResult {
    for status in ["DQ", "DNF", "DNS", "NH", "FOUL", "NM", "NT", "SCR"] {
        for unresolved in ["unknown published mark", "X", "--"] {
            let observations = owner_resolution_triplet(status, unresolved)?;
            let directory = tempfile::tempdir()?;
            let store = Store::open(directory.path())?;
            store.append_many(Table::Performances, &observations)?;
            let before: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            check!(eq; before.len(), 1);
            check_retained_owner_resolution(before.first().ok_or("first-pass canonical row")?)?;
            store.append_many(Table::Performances, &observations)?;
            check!(eq; store.scan::<CanonicalPerformance>(Table::Performances)?, before);
            store.flush()?;
            drop(store);
            let reopened = Store::open(directory.path())?;
            check!(eq; reopened.scan::<CanonicalPerformance>(Table::Performances)?, before);
            let mut physical = Vec::new();
            reopened.snapshot().for_each_observation(
                Table::Performances,
                |row: CanonicalPerformance| {
                    physical.push(row);
                    Ok(())
                },
            )?;
            check!(eq; physical.len(), 6);
            observations.iter().try_for_each(|row| -> TestResult {
                check!(physical.contains(row));
                Ok(())
            })?;
        }
    }
    Ok(())
}
