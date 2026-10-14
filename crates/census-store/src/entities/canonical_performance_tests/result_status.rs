use super::*;
use census_domain::model::{CentiMetres, CentiPoints, Evidence, SourceRef};

#[path = "result_status/authority.rs"]
mod authority;
#[path = "result_status/contexts.rs"]
mod contexts;
#[path = "result_status/owner_resolution.rs"]
mod owner_resolution;
#[path = "result_status/triplets.rs"]
mod triplets;

fn numeric_marks() -> TestResult<[Mark; 4]> {
    Ok([
        Mark::TimeSeconds(ExactSeconds::parse("10.941")?),
        Mark::DistanceMetres(CentiMetres::new(800)),
        Mark::FieldImperial {
            feet_mark: "25-00.00".into(),
            metres: CentiMetres::new(762),
        },
        Mark::Points(CentiPoints::new(500000)),
    ])
}

fn status_pair(status: &str, mark: Mark) -> TestResult<[CanonicalPerformance; 2]> {
    let mut measured = performance("terminal-numeric-conflict", identity("111"))?;
    measured.mark = mark;
    measured.evidence = vec![Evidence::parsed(
        SourceRef::id("numeric-observation"),
        "2026-05-01",
    )];
    let mut terminal = measured.clone();
    terminal.mark = Mark::Raw(status.into());
    terminal.timing = None;
    terminal.evidence = vec![Evidence::parsed(
        SourceRef::id("terminal-observation"),
        "2026-05-01",
    )];
    Ok([measured, terminal])
}

fn orders(pair: &[CanonicalPerformance; 2]) -> [[CanonicalPerformance; 2]; 2] {
    [
        [pair[0].clone(), pair[1].clone()],
        [pair[1].clone(), pair[0].clone()],
    ]
}

fn assert_terminal_conflict(
    kept: CanonicalPerformance,
    incoming: CanonicalPerformance,
) -> TestResult {
    let mut merged = kept.clone();
    merged.merge(incoming.clone());
    check!(eq; merged.mark, kept.mark);
    check!(eq; merged.timing, kept.timing);
    check!(eq; merged.retained_conflicts.len(), 1);
    let conflict = merged
        .retained_conflicts
        .first()
        .ok_or("retained terminal conflict")?;
    check!(eq; conflict.family, "Individual result scalar");
    check!(eq; conflict.subject_id, kept.id.as_str());
    check!(conflict.detail.contains(&format!("{:?}", &kept.mark)));
    check!(conflict.detail.contains(&format!("{:?}", &incoming.mark)));
    check!(eq; merged.evidence.len(), 2);
    kept.evidence
        .iter()
        .chain(&incoming.evidence)
        .try_for_each(|evidence| -> TestResult {
            check!(merged.evidence.contains(evidence));
            Ok(())
        })?;
    let expected = merged.clone();
    merged.merge(incoming.clone());
    merged.merge(kept.clone());
    check!(eq; merged, expected);
    let mut reversed = incoming;
    reversed.merge(kept);
    check!(eq; reversed.retained_conflicts, expected.retained_conflicts);
    Ok(())
}

#[test]
fn explicit_terminal_and_every_numeric_mark_retain_evidence_and_conflict_in_both_orders(
) -> TestResult {
    for status in ["DQ", "DNF", "DNS", "NH", "FOUL", "NM", "NT", "SCR"] {
        for token in [
            status.to_string(),
            format!(" \t{}\u{2003}\n", status.to_ascii_lowercase()),
        ] {
            for mark in numeric_marks()? {
                for [kept, incoming] in orders(&status_pair(&token, mark)?) {
                    assert_terminal_conflict(kept, incoming)?;
                }
            }
        }
    }
    Ok(())
}

fn assert_stored_pair(
    store: &Store,
    expected: &[CanonicalPerformance; 2],
    physical_count: usize,
) -> TestResult {
    let canonical: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
    check!(eq; canonical.len(), 1);
    let merged = canonical.first().ok_or("merged canonical result")?;
    check!(eq; merged.mark, expected[0].mark);
    check!(eq; merged.timing, expected[0].timing);
    check!(eq; merged.retained_conflicts.len(), 1);
    check!(eq; merged.retained_conflicts.first().ok_or("retained result conflict")?.family,
        "Individual result scalar");
    check!(eq; merged.evidence.len(), 2);
    expected
        .iter()
        .flat_map(|row| &row.evidence)
        .try_for_each(|evidence| -> TestResult {
            check!(merged.evidence.contains(evidence));
            Ok(())
        })?;
    let mut physical = Vec::new();
    store
        .snapshot()
        .for_each_observation(Table::Performances, |row: CanonicalPerformance| {
            physical.push(row);
            Ok(())
        })?;
    check!(eq; physical.len(), physical_count);
    expected.iter().try_for_each(|row| -> TestResult {
        check!(physical.contains(row));
        Ok(())
    })
}

#[test]
fn stored_terminal_numeric_conflict_survives_replay_and_reopen_with_original_observations(
) -> TestResult {
    for status in ["DQ", "DNF", "DNS", "NH", "FOUL", "NM", "NT", "SCR"] {
        let pair = status_pair(status, Mark::TimeSeconds(ExactSeconds::parse("10.941")?))?;
        for observations in orders(&pair) {
            let directory = tempfile::tempdir()?;
            let store = Store::open(directory.path())?;
            store.append_many(Table::Performances, &observations)?;
            assert_stored_pair(&store, &observations, 2)?;
            store.append_many(Table::Performances, &observations)?;
            assert_stored_pair(&store, &observations, 4)?;
            let before: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            store.flush()?;
            drop(store);
            let reopened = Store::open(directory.path())?;
            assert_stored_pair(&reopened, &observations, 4)?;
            check!(eq; reopened.scan::<CanonicalPerformance>(Table::Performances)?, before);
        }
    }
    Ok(())
}

#[test]
fn ambiguous_and_unknown_raw_marks_remain_refinable_in_both_orders() -> TestResult {
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
        let pair = status_pair(raw, Mark::TimeSeconds(ExactSeconds::parse("10.941")?))?;
        for [kept, incoming] in orders(&pair) {
            let mut merged = kept;
            merged.merge(incoming);
            check!(eq; merged.mark, pair[0].mark);
            check!(eq; merged.timing, pair[0].timing);
            check!(eq; merged.retained_conflicts, Vec::new());
            check!(eq; merged.evidence.len(), 2);
        }
    }
    Ok(())
}
