use super::*;
use crate::report::Derivation;

fn assert_case(case: AffiliationCase) -> TestResult {
    let fixture = Fixture::new(case)?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store(&directory.path().join("store"))?;
    let dataset = ExportDataset::load(&store)?;
    [Scope::Core, Scope::AllSources]
        .into_iter()
        .try_for_each(|scope| -> TestResult {
            let derivation = Derivation::of(&dataset, scope, Some(2027));
            check!(eq; derivation.athletes(), std::slice::from_ref(&fixture.athlete));
            check!(eq; derivation.performances(), &[&fixture.performance]);
            assert_selection(&dataset, &fixture, scope)?;
            Ok(())
        })
}

#[test]
fn bests_keep_unknown_affiliation_when_the_performance_team_is_missing() -> TestResult {
    assert_case(AffiliationCase::MissingTeam)
}

#[test]
fn bests_keep_unknown_affiliation_when_the_historical_school_is_missing() -> TestResult {
    assert_case(AffiliationCase::MissingSchool)
}

#[test]
fn bests_keep_the_historical_name_when_its_state_is_unknown() -> TestResult {
    assert_case(AffiliationCase::UnknownHistoricalState)
}

#[test]
fn bests_keep_wisconsin_affiliation_after_a_transfer_to_illinois() -> TestResult {
    assert_case(AffiliationCase::CrossStateTransfer)
}

#[test]
fn bests_affiliation_is_independent_of_current_school_name_and_state() -> TestResult {
    AffiliationCase::ALL.into_iter().try_for_each(|case| -> TestResult {
        let fixture = Fixture::new(case)?;
        let directory = tempfile::tempdir()?;
        let store = fixture.store(&directory.path().join("store"))?;
        let mut dataset = ExportDataset::load(&store)?;
        [Scope::Core, Scope::AllSources]
            .into_iter()
            .try_for_each(|scope| -> TestResult {
                let expected = build_from_dataset(&dataset, &options(scope))?;
                check!(eq; expected.len(), 1, "{case:?}");
                [UsJurisdiction::Illinois, UsJurisdiction::Michigan, UsJurisdiction::Ohio]
                    .into_iter()
                    .try_for_each(|state| -> TestResult {
                        let current = dataset
                            .schools
                            .get_mut(&fixture.current.id)
                            .ok_or("missing current school mutation target")?;
                        current.name = format!("Current {} school", state.code());
                        current.state = Some(state);
                        let actual = build_from_dataset(&dataset, &options(scope))?;
                        check!(eq; actual, expected, "{case:?}, {scope:?}, current={state:?}");
                        check!(eq; dataset.performances.as_slice(), std::slice::from_ref(&fixture.performance));
                        check!(eq; dataset.athletes.as_slice(), std::slice::from_ref(&fixture.athlete));
                        Ok(())
                    })
            })
    })
}

fn assert_mixed_order(rows: &[SharedSelection], expected: &[&Fixture; 3]) -> TestResult {
    check!(eq; rows.len(), 3);
    let ids = rows
        .iter()
        .map(SharedSelection::athlete_id)
        .collect::<Vec<_>>();
    check!(eq; ids, expected.iter().map(|fixture| &fixture.athlete.id).collect::<Vec<_>>());
    let affiliations = rows
        .iter()
        .map(|row| {
            (
                row.athlete.athlete_state.code(),
                row.athlete.school.as_deref(),
            )
        })
        .collect::<Vec<_>>();
    check!(eq; affiliations, vec![
        ("WI", Some("Zulu historical school")),
        ("UNKNOWN", None),
        ("UNKNOWN", Some("Alpha historical school")),
    ]);
    rows.iter().zip(expected).try_for_each(|(row, fixture)| -> TestResult {
        check!(eq; row.athlete.name, fixture.athlete.canonical_name);
        check!(eq; row.athlete.athlete_school.as_str(), row.athlete.school.as_deref().map_or("", core::convert::identity));
        check!(eq; row.athlete.profile_url.as_deref(), Some(PROFILE_URL));
        check!(eq; row.source.performance_id, fixture.performance.id);
        check!(eq; row.source.source_key, fixture.performance.source_key);
        check!(eq; row.source.source_athlete.as_str(), fixture.performance.source_athlete.as_ref().ok_or("missing mixed-row owner")?.id.as_str());
        check!(eq; row.source.result_url.as_str(), RESULT_URL);
        check!(eq; row.result.mark, fixture.performance.mark);
        check!(eq; row.result.value, MARK_VALUE);
        check!(eq; row.meet.meet_state, MeetState::Placed(UsJurisdiction::Michigan));
        Ok(())
    })
}

#[test]
fn mixed_historical_affiliation_order_and_limits_keep_known_and_unknown_rows_in_exact_prefixes(
) -> TestResult {
    let missing = Fixture::mixed_row(
        AffiliationCase::MissingTeam,
        "A missing-team runner",
        "Unused historical school",
        "mixed-missing-team",
    )?;
    let unknown = Fixture::mixed_row(
        AffiliationCase::UnknownHistoricalState,
        "B unknown-state runner",
        "Alpha historical school",
        "mixed-unknown-state",
    )?;
    let known = Fixture::mixed_row(
        AffiliationCase::CrossStateTransfer,
        "Z Wisconsin runner",
        "Zulu historical school",
        "mixed-known-state",
    )?;
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    [&unknown, &missing, &known]
        .into_iter()
        .try_for_each(|fixture| fixture.append_to(&store))?;
    let dataset = ExportDataset::load(&store)?;
    check!(eq; dataset.canonical_aliases.len(), 0);
    check!(eq; dataset.schools.get(&missing.current.id).and_then(|school| school.state), Some(UsJurisdiction::Illinois));
    check!(eq; dataset.athletes.iter().filter(|athlete| athlete.school == missing.current.id).count(), 3);
    let expected = [&known, &missing, &unknown];
    [Scope::Core, Scope::AllSources]
        .into_iter()
        .try_for_each(|scope| -> TestResult {
            check!(eq; Derivation::of(&dataset, scope, Some(2027)).athletes().len(), 3);
            let complete = build_from_dataset(&dataset, &options(scope))?;
            assert_mixed_order(&complete, &expected)?;
            [0_usize, 1, 2, 3, 4]
                .into_iter()
                .try_for_each(|limit| -> TestResult {
                    let mut limited_options = options(scope);
                    limited_options.limit = Some(limit);
                    let limited = build_from_dataset(&dataset, &limited_options)?;
                    let prefix = complete.iter().take(limit).cloned().collect::<Vec<_>>();
                    check!(eq; limited, prefix, "{scope:?}, limit={limit}");
                    Ok(())
                })
        })
}
