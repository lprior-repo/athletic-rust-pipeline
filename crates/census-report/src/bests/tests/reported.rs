use super::*;
use crate::bests::{reported_best_groups, ReportedBestGroup};
use crate::report::EventAdmission;

mod refusals;

fn groups(dataset: &ExportDataset) -> TestResult<Vec<ReportedBestGroup>> {
    let admission = EventAdmission::of(dataset)?;
    let performances = dataset.performances.iter().collect::<Vec<_>>();
    Ok(reported_best_groups(dataset, &admission, &performances)?)
}

fn parity(dataset: &ExportDataset) -> TestResult<Vec<ReportedBestGroup>> {
    let groups = groups(dataset)?;
    let mut expected = selected(dataset)?
        .into_iter()
        .map(|selection| {
            (
                selection.key,
                Some(selection.source.performance_id),
                selection.conflicts,
            )
        })
        .collect::<Vec<_>>();
    expected.sort_by(|left, right| left.0.cmp(&right.0));
    let actual = groups
        .iter()
        .filter(|group| group.candidate_id.is_some())
        .map(|group| {
            (
                group.key.clone(),
                group.candidate_id.clone(),
                group.conflicts.clone(),
            )
        })
        .collect::<Vec<_>>();
    check!(eq; actual, expected);
    Ok(groups)
}

#[test]
fn reported_numeric_and_tie_winners_match_verified_path_for_every_measure() -> TestResult {
    let fixture = fixture::TieFixture::new()?;
    for case in fixture::numeric_cases()? {
        let earlier = fixture.report(&case, "reported-earlier", "2025-03-01")?;
        let tied = fixture.report(&case, "reported-tied", "2025-03-02")?;
        let better = fixture
            .report(&case, "reported-better", "2025-03-03")?
            .with_mark(case.better.clone())?;
        let directory = tempfile::tempdir()?;
        let store = fixture
            .store_with_replay(directory.path(), &[earlier.clone(), tied, better.clone()])?;
        let mut dataset = ExportDataset::load(&store)?;
        dataset.lineage.policy_revision = 21;
        for _ in 0..2 {
            let groups = parity(&dataset)?;
            check!(eq; groups.len(), 1);
            check!(eq; groups.first().ok_or("reported group")?.candidate_id.as_ref(), Some(&better.performance.id));
            dataset.performances.reverse();
        }
        dataset
            .performances
            .retain(|row| row.id != better.performance.id);
        for _ in 0..2 {
            let groups = parity(&dataset)?;
            check!(eq; groups.first().ok_or("tie group")?.candidate_id.as_ref(), Some(&earlier.performance.id));
            dataset.performances.reverse();
        }
    }
    Ok(())
}

#[test]
fn reported_wind_and_timing_classes_match_verified_keys() -> TestResult {
    let mut dataset = selection_dataset(EventKind::Track100m)?;
    dataset.lineage.policy_revision = 21;
    let legal = reported(
        &dataset,
        "reported-legal",
        Mark::TimeSeconds(ExactSeconds::parse("10.50")?),
    );
    let mut assisted = legal.clone();
    assisted.id = Id::mint("perf", &["reported-assisted"]);
    assisted.wind_mps = Some(3.0);
    let mut hand = legal.clone();
    hand.id = Id::mint("perf", &["reported-hand"]);
    hand.timing = Some(TimingMethod::Hand);
    dataset.performances = vec![legal, assisted, hand];
    for _ in 0..2 {
        check!(eq; parity(&dataset)?.len(), 3);
        dataset.performances.reverse();
    }
    Ok(())
}

#[test]
fn reported_conflict_only_group_is_visible_and_clean_fallback_matches_verified() -> TestResult {
    let mut dataset = selection_dataset(EventKind::Track100m)?;
    dataset.lineage.policy_revision = 21;
    let numeric = reported(
        &dataset,
        "reported-contested",
        Mark::TimeSeconds(ExactSeconds::parse("10.50")?),
    );
    let mut terminal = numeric.clone();
    terminal.mark = Mark::Raw("DNF".into());
    let mut clean = numeric.clone();
    clean.id = Id::mint("perf", &["reported-clean"]);
    clean.mark = Mark::TimeSeconds(ExactSeconds::parse("10.80")?);
    dataset.performances = vec![numeric, terminal];
    for _ in 0..2 {
        let groups = parity(&dataset)?;
        check!(eq; groups.len(), 1);
        let group = groups.first().ok_or("conflict-only group")?;
        check!(group.candidate_id.is_none());
        check!(eq; group.conflicts.len(), 1);
        dataset.performances.reverse();
    }
    let clean_id = clean.id.clone();
    dataset.performances.push(clean);
    for _ in 0..2 {
        let groups = parity(&dataset)?;
        let group = groups.first().ok_or("fallback group")?;
        check!(eq; group.candidate_id.as_ref(), Some(&clean_id));
        check!(eq; group.conflicts.len(), 1);
        dataset.performances.reverse();
    }
    Ok(())
}
