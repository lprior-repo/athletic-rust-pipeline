use super::super::*;
use super::fixture::{numeric_cases, orders, NumericCase, Observation, TieFixture};
use census_domain::model::{Evidence, SourceRef};

pub(super) fn assert_winner(
    dataset: &ExportDataset,
    expected: &Observation,
) -> TestResult<SharedSelection> {
    let rows = selected(dataset)?;
    check!(eq; rows.len(), 1);
    let row = rows.into_iter().next().ok_or("missing tie winner")?;
    let performance = &expected.performance;
    let evidence = performance.evidence.first().ok_or("missing tie evidence")?;
    check!(eq; row.source.performance_id, performance.id);
    check!(eq; row.source.source_key, performance.source_key);
    check!(eq; row.source.result_url.as_str(), evidence.source.url.as_deref().ok_or("missing tie URL")?);
    check!(eq; row.source.source_athlete.as_str(), performance.source_athlete.as_ref().ok_or("missing source owner")?.id.as_str());
    check!(eq; row.result.mark, performance.mark);
    check!(eq; row.result.timing, performance.timing);
    check!(eq; row.result.wind_mps, performance.wind_mps);
    check!(eq; row.meet.date, performance.date);
    check!(eq; row.meet.meet_id, performance.meet);
    check!(eq; row.meet.name, expected.meet.name);
    check!(eq; row.meet.round, performance.round);
    check!(eq; row.meet.heat, performance.heat);
    check!(eq; row.athlete.school.as_deref(), Some("Tie fixture school"));
    check!(eq; row.athlete.grad_year, 2027);
    Ok(row)
}

fn assert_all_orders(
    fixture: &TieFixture,
    observations: &[Observation],
    expected: &Observation,
) -> TestResult {
    orders(observations)?
        .into_iter()
        .try_for_each(|order| -> TestResult {
            let directory = tempfile::tempdir()?;
            let store = fixture.store_with_replay(directory.path(), &order)?;
            let mut dataset = ExportDataset::load(&store)?;
            let row = assert_winner(&dataset, expected)?;
            dataset.performances.reverse();
            check!(eq; assert_winner(&dataset, expected)?, row);
            Ok(())
        })
}

fn chronological_reports(fixture: &TieFixture, case: &NumericCase) -> TestResult<[Observation; 3]> {
    Ok([
        fixture.report(case, "later-exact", "2026-04-02")?,
        fixture.report(case, "earlier-exact", "2026-04-01")?,
        fixture.report(case, "partial-date", "2025")?,
    ])
}

#[test]
fn shared_selection_ties_choose_earliest_exact_date_for_every_measure() -> TestResult {
    let fixture = TieFixture::new()?;
    numeric_cases()?.iter().try_for_each(|case| -> TestResult {
        let observations = chronological_reports(&fixture, case)?;
        let earliest = observations.get(1).ok_or("earliest fixture")?;
        assert_all_orders(&fixture, &observations, earliest)?;
        let directory = tempfile::tempdir()?;
        let store = fixture.store_with_replay(directory.path(), &observations)?;
        let row = assert_winner(&ExportDataset::load(&store)?, earliest)?;
        check!(eq; row.result.value, case.value);
        check!(eq; row.population.marks, 3);
        check!(eq; row.conflicts.len(), 0);
        Ok(())
    })
}

#[test]
fn shared_selection_better_numeric_marks_win_even_with_unknown_dates() -> TestResult {
    let fixture = TieFixture::new()?;
    numeric_cases()?.iter().try_for_each(|case| -> TestResult {
        let earlier = fixture.report(case, "earlier-slower", "2024-02-29")?;
        let later = fixture.report(case, "later-slower", "2026-04-01")?;
        let mut better = fixture.report(case, "better-unknown-date", "2025-02-29")?;
        better.performance.mark = case.better.clone();
        assert_all_orders(&fixture, &[earlier, better.clone(), later], &better)
    })
}

#[test]
fn shared_selection_exact_calendar_dates_precede_partial_and_malformed_dates() -> TestResult {
    let fixture = TieFixture::new()?;
    let cases = numeric_cases()?;
    let case = cases.first().ok_or("time fixture")?;
    [
        "",
        "2023",
        "2025-02",
        "2025-02-29",
        "2026-04-31",
        "2026-01-01T00:00:00Z",
        " 2026-01-01",
        "２０２６-01-01",
    ]
    .into_iter()
    .try_for_each(|unknown| -> TestResult {
        let exact = fixture.report(case, "known-leap-day", "2024-02-29")?;
        let invalid = fixture.report(case, "unknown-calendar-date", unknown)?;
        assert_all_orders(&fixture, &[invalid, exact.clone()], &exact)
    })
}

pub(super) fn stable_id_reports(
    fixture: &TieFixture,
    case: &NumericCase,
) -> TestResult<[Observation; 2]> {
    let first = fixture.report(case, "same-date-first", "2026-04-01")?;
    let second = fixture.report(case, "same-date-second", "2026-04-01")?;
    let (mut preferred, mut other) = if first.performance.id < second.performance.id {
        (first, second)
    } else {
        (second, first)
    };
    let (low_meet, high_meet) = if preferred.meet.id < other.meet.id {
        (preferred.meet.id.clone(), other.meet.id.clone())
    } else {
        (other.meet.id.clone(), preferred.meet.id.clone())
    };
    preferred.meet.id = high_meet;
    other.meet.id = low_meet;
    rebind_event(&mut preferred, case)?;
    rebind_event(&mut other, case)?;
    check!(preferred.performance.id < other.performance.id);
    check!(preferred.performance.meet > other.performance.meet);
    Ok([preferred, other])
}

pub(super) fn reverse_meet_order(
    observations: &[Observation; 2],
    case: &NumericCase,
) -> TestResult<[Observation; 2]> {
    let [preferred, other] = observations;
    let mut preferred = preferred.clone();
    let mut other = other.clone();
    std::mem::swap(&mut preferred.meet.id, &mut other.meet.id);
    rebind_event(&mut preferred, case)?;
    rebind_event(&mut other, case)?;
    check!(preferred.performance.id < other.performance.id);
    check!(preferred.performance.meet < other.performance.meet);
    Ok([preferred, other])
}

fn rebind_event(observation: &mut Observation, case: &NumericCase) -> TestResult {
    let event = crate::test_events::owned(CanonicalEvent::new(
        EventIdentity {
            meet: &observation.meet.id,
            kind: case.kind.clone(),
            gender: Gender::Boys,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?);
    observation.event.id = event.id;
    observation.event.meet = observation.meet.id.clone();
    observation.performance.meet = observation.meet.id.clone();
    observation.performance.event = observation.event.id.clone();
    Ok(())
}

#[test]
fn shared_selection_same_date_uses_stable_id_independent_of_meet_id() -> TestResult {
    let fixture = TieFixture::new()?;
    numeric_cases()?.iter().try_for_each(|case| -> TestResult {
        let observations = stable_id_reports(&fixture, case)?;
        let reversed = reverse_meet_order(&observations, case)?;
        [observations, reversed]
            .iter()
            .try_for_each(|observations| {
                assert_all_orders(
                    &fixture,
                    observations,
                    observations.first().ok_or("ID winner")?,
                )
            })
    })
}

#[test]
fn shared_selection_same_context_equal_reports_choose_stable_id() -> TestResult {
    let fixture = TieFixture::new()?;
    numeric_cases()?.iter().try_for_each(|case| -> TestResult {
        let [preferred, mut other] = stable_id_reports(&fixture, case)?;
        other.meet = preferred.meet.clone();
        other.event = preferred.event.clone();
        other.performance.meet = preferred.performance.meet.clone();
        other.performance.event = preferred.performance.event.clone();
        assert_all_orders(&fixture, &[preferred.clone(), other], &preferred)
    })
}

#[test]
fn shared_selection_unknown_dates_use_stable_id_without_inventing_a_year() -> TestResult {
    let fixture = TieFixture::new()?;
    let cases = numeric_cases()?;
    let case = cases.first().ok_or("time fixture")?;
    let observations = [
        fixture.report(case, "unknown-year", "1900")?,
        fixture.report(case, "unknown-month", "2026-01")?,
        fixture.report(case, "invalid-day", "2026-02-30")?,
        fixture.report(case, "missing-date", "")?,
    ];
    let preferred = observations
        .iter()
        .min_by_key(|observation| &observation.performance.id)
        .ok_or("unknown-date winner")?;
    assert_all_orders(&fixture, &observations, preferred)
}

fn provider_evidence(observation: &mut Observation, provider: &str) -> TestResult {
    let url = observation
        .performance
        .evidence
        .first()
        .and_then(|evidence| evidence.source.url.clone())
        .ok_or("provider fixture URL")?;
    let source = SourceRef::new(provider, Some(url));
    let evidence = Evidence::parsed(source.clone(), "2026-10-07");
    observation.meet.evidence = vec![evidence.clone()];
    observation.event.evidence = vec![evidence.clone()];
    observation
        .event
        .source_labels
        .iter_mut()
        .for_each(|label| label.source = source.clone());
    observation.performance.evidence = vec![evidence];
    Ok(())
}

#[test]
fn shared_selection_provider_names_have_uniform_tie_preference() -> TestResult {
    let fixture = TieFixture::new()?;
    let cases = numeric_cases()?;
    let case = cases.first().ok_or("time fixture")?;
    let providers = ["wiaa_results", "milesplit", "athletic_net", "tfrrs"];
    providers
        .into_iter()
        .try_for_each(|earliest_provider| -> TestResult {
            let observations = providers
                .iter()
                .enumerate()
                .map(|(index, provider)| {
                    let mut observation =
                        fixture.report(case, &format!("provider-{index}"), "2026-04-02")?;
                    provider_evidence(&mut observation, provider)?;
                    if *provider == earliest_provider {
                        observation.performance.date = "2026-04-01".into();
                    }
                    Ok(observation)
                })
                .collect::<TestResult<Vec<_>>>()?;
            let expected = observations
                .iter()
                .find(|observation| observation.performance.date == "2026-04-01")
                .ok_or("earliest provider")?
                .clone();
            assert_all_orders(&fixture, &observations, &expected)
        })
}

#[test]
fn shared_selection_tie_preference_cannot_revive_a_retained_conflict_context() -> TestResult {
    let fixture = TieFixture::new()?;
    numeric_cases()?.iter().try_for_each(|case| -> TestResult {
        let contested = fixture.report(case, "contested-earliest", "2024-02-29")?;
        let mut revision = contested.clone();
        revision.performance.mark = case.better.clone();
        let mut duplicate = contested.clone();
        duplicate.performance.id = Id::mint("perf", &["contested-duplicate"]);
        duplicate.performance.source_key = "contested-duplicate".into();
        let clean = fixture.report(case, "clean-later", "2026-04-01")?;
        let observations = [contested.clone(), revision, duplicate, clean.clone()];
        orders(&observations)?
            .into_iter()
            .try_for_each(|order| -> TestResult {
                let directory = tempfile::tempdir()?;
                let store = fixture.store_with_replay(directory.path(), &order)?;
                let mut dataset = ExportDataset::load(&store)?;
                let retained = dataset
                    .performances
                    .iter()
                    .find(|performance| performance.id == contested.performance.id)
                    .ok_or("retained contested performance")?;
                check!(eq; retained.retained_conflicts.len(), 1);
                let row = assert_winner(&dataset, &clean)?;
                check!(eq; row.result.value, case.value);
                check!(eq; row.conflicts.len(), 1);
                dataset.performances.reverse();
                check!(eq; assert_winner(&dataset, &clean)?, row);
                dataset
                    .performances
                    .retain(|performance| performance.id != clean.performance.id);
                check!(eq; selected(&dataset)?, Vec::new());
                Ok(())
            })
    })
}
