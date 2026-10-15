use super::*;
use census_domain::model::{Adjustment, Evidence, ExactSeconds, Mark, SourceRef};

fn evidence(url: &str, date: &str) -> Evidence {
    Evidence::parsed(SourceRef::new("timer", Some(url.into())), date)
}

#[test]
fn repeated_id_provenance_ties_are_stable_in_both_orders_without_recency_priority() -> TestResult {
    let fixture = TieFixture::new()?;
    for case in numeric_cases()? {
        let [first, _] = reports(&fixture, &case)?;
        let directory = tempfile::tempdir()?;
        let store = fixture.store_with_replay(directory.path(), std::slice::from_ref(&first))?;
        let mut dataset = ExportDataset::load(&store)?;
        for [lexical_date, other_date] in
            [["2026-10-01", "2026-10-09"], ["2026-10-09", "2026-10-01"]]
        {
            let mut lexical = first.performance.clone();
            lexical.evidence = vec![evidence("https://example.test/a", lexical_date)];
            let mut other = first.performance.clone();
            other.evidence = vec![evidence("https://example.test/z", other_date)];
            dataset.performances = vec![other, lexical.clone()];
            let retained = dataset.performances.clone();
            let expected = single(&mut dataset, 21)?;
            check!(eq; expected.source.result_url, "https://example.test/a");
            check!(eq; expected.inventory.eligible_group_count(), 1);
            check!(eq; expected.inventory.tied_group_count(), 1);
            check!(eq; expected.supporting_tied_candidate_ids(), &[lexical.id.clone()]);
            check!(eq; dataset.performances, retained);
            dataset.performances.reverse();
            check!(eq; single(&mut dataset, 21)?, expected);
            check!(eq; dataset.performances, retained.into_iter().rev().collect::<Vec<_>>());
        }
    }
    Ok(())
}

#[test]
fn historical_provenance_ties_keep_the_retained_first_report() -> TestResult {
    let fixture = TieFixture::new()?;
    let cases = numeric_cases()?;
    let [first, _] = reports(&fixture, cases.first().ok_or("ordinary case")?)?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), std::slice::from_ref(&first))?;
    let mut dataset = ExportDataset::load(&store)?;
    let mut lexical = first.performance.clone();
    lexical.evidence = vec![evidence("https://example.test/a", "2026-10-09")];
    let mut other = first.performance;
    other.evidence = vec![evidence("https://example.test/z", "2026-10-01")];
    for policy in [18, 19, 20] {
        dataset.performances = vec![other.clone(), lexical.clone()];
        check!(eq; single(&mut dataset, policy)?.source.result_url, "https://example.test/z");
        dataset.performances.reverse();
        check!(eq; single(&mut dataset, policy)?.source.result_url, "https://example.test/a");
    }
    Ok(())
}

#[test]
fn repeated_id_equal_conversion_provenance_uses_stable_exact_metadata() -> TestResult {
    let fixture = TieFixture::new()?;
    let cases = numeric_cases()?;
    let [first, _] = reports(&fixture, cases.first().ok_or("ordinary case")?)?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), std::slice::from_ref(&first))?;
    let mut dataset = ExportDataset::load(&store)?;
    let mut lexical = first.performance;
    lexical.adjustment = Some(Adjustment::track_size_conversion(
        Some(lexical.mark.clone()),
        lexical.mark.clone(),
        Some("a retained derivation".into()),
        Some(SourceRef::id("timer")),
    ));
    let mut other = lexical.clone();
    other.adjustment.as_mut().ok_or("adjustment")?.note = Some("z retained derivation".into());
    dataset.performances = vec![other, lexical];
    let expected = single(&mut dataset, 21)?;
    check!(eq; expected.result.adjustment_note.as_deref(), Some("a retained derivation"));
    dataset.performances.reverse();
    check!(eq; single(&mut dataset, 21)?, expected);
    Ok(())
}

#[test]
fn repeated_id_numeric_ties_stabilize_exact_time_precision_without_changing_value() -> TestResult {
    let fixture = TieFixture::new()?;
    let cases = numeric_cases()?;
    let [first, _] = reports(&fixture, cases.first().ok_or("ordinary case")?)?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), std::slice::from_ref(&first))?;
    let mut dataset = ExportDataset::load(&store)?;
    let mut more_digits = first.performance.clone();
    more_digits.mark = Mark::TimeSeconds(ExactSeconds::parse("50.000")?);
    dataset.performances = vec![more_digits, first.performance];
    let expected = single(&mut dataset, 21)?;
    let Mark::TimeSeconds(time) = &expected.result.mark else {
        return Err("selected mark is not a time".into());
    };
    check!(eq; time.precision(), 2);
    let encoded = serde_json::to_string(&expected)?;
    dataset.performances.reverse();
    check!(eq; serde_json::to_string(&single(&mut dataset, 21)?)?, encoded);
    Ok(())
}
