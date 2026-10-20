use super::*;
use census_domain::model::Mark;

#[test]
fn ordinary_terminal_replay_with_numeric_same_id_uses_only_the_clean_distinct_id() -> TestResult {
    let fixture = TieFixture::new()?;
    for case in numeric_cases()?.into_iter().take(2) {
        let [first, clean] = reports(&fixture, &case)?;
        let first = first.with_mark(case.better)?;
        let directory = tempfile::tempdir()?;
        let store = fixture.store_with_replay(directory.path(), &[first.clone(), clean.clone()])?;
        let mut dataset = ExportDataset::load(&store)?;
        for status in ["DNF", "DNS", "DQ", "SCR", "NH", "FOUL", "NM", "NT"] {
            let mut terminal = first.performance.clone();
            terminal.mark = Mark::Raw(status.into());
            dataset.performances = vec![
                first.performance.clone(),
                terminal.clone(),
                terminal,
                clean.performance.clone(),
            ];
            for _ in 0..2 {
                let selected = single(&mut dataset, 21)?;
                check!(eq; selected.source.performance_id, clean.performance.id);
                check!(eq; selected.inventory.eligible_group_count(), 1);
                check!(eq; selected.inventory.tied_group_count(), 1);
                check!(eq; selected.supporting_tied_candidate_ids(), std::slice::from_ref(&clean.performance.id));
                dataset.performances.reverse();
            }
        }
    }
    Ok(())
}

#[test]
fn ordinary_terminal_policy_change_does_not_reinterpret_historical_public_input() -> TestResult {
    let fixture = TieFixture::new()?;
    let cases = numeric_cases()?;
    let [first, _] = reports(&fixture, cases.first().ok_or("ordinary case")?)?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), std::slice::from_ref(&first))?;
    let mut dataset = ExportDataset::load(&store)?;
    let mut terminal = first.performance.clone();
    terminal.mark = Mark::Raw("DNF".into());
    dataset.performances.push(terminal);
    check!(rows(&mut dataset, 21)?.is_empty());
    for policy in [19, 20] {
        check!(eq; single(&mut dataset, policy)?.source.performance_id, first.performance.id);
    }
    Ok(())
}

#[test]
fn ordinary_terminal_under_a_distinct_id_does_not_poison_same_context_numeric_result() -> TestResult
{
    let fixture = TieFixture::new()?;
    let cases = numeric_cases()?;
    let [first, mut terminal] = reports(&fixture, cases.first().ok_or("ordinary case")?)?;
    terminal.performance.mark = Mark::Raw("DNF".into());
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), &[first.clone(), terminal])?;
    let mut dataset = ExportDataset::load(&store)?;
    for _ in 0..2 {
        let selected = single(&mut dataset, 21)?;
        check!(eq; selected.source.performance_id, first.performance.id);
        check!(eq; selected.inventory.eligible_group_count(), 1);
        dataset.performances.reverse();
    }
    Ok(())
}
