use super::*;
use crate::report::{build_census, coverage_report, CoverageTotals};

fn assert_case(case: Case) -> TestResult {
    let fixture = Fixture::new(case)?;
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    fixture.append_to(&store)?;
    let dataset = ExportDataset::load(&store)?;
    [Scope::Core, Scope::AllSources]
        .into_iter()
        .try_for_each(|scope| -> TestResult {
            assert_projection(&dataset, &fixture, scope)?;
            if case.has_pr(scope) {
                assert_selection(&dataset, &fixture, scope)?;
            } else {
                check!(eq; build_from_dataset(&dataset, &options(scope))?, Vec::new());
            }
            Ok(())
        })?;
    check!(eq; dataset.performances.as_slice(), std::slice::from_ref(&fixture.performance));
    check!(eq; dataset.athletes.as_slice(), std::slice::from_ref(&fixture.athlete));
    check!(eq; dataset.canonical_aliases.len(), 0);
    Ok(())
}

#[test]
fn selected_illinois_athlete_keeps_alaska_school_and_hawaii_venue_with_exact_outdoor_pr(
) -> TestResult {
    assert_case(Case::AlaskaSchoolHawaiiVenue)
}

#[test]
fn selected_illinois_athlete_keeps_hawaii_school_and_alaska_venue_with_exact_outdoor_pr(
) -> TestResult {
    assert_case(Case::HawaiiSchoolAlaskaVenue)
}

#[test]
fn missing_historical_team_and_school_stay_unknown_without_current_school_substitution(
) -> TestResult {
    [Case::MissingTeam, Case::MissingSchool]
        .into_iter()
        .try_for_each(assert_case)
}

#[test]
fn missing_historical_meet_stays_unknown_and_cannot_supply_comparison_context() -> TestResult {
    assert_case(Case::MissingMeet)
}

#[test]
fn core_never_borrows_noncore_meet_context_but_all_sources_restores_exact_outdoor_pr() -> TestResult
{
    assert_case(Case::NonCoreMeet)
}

#[test]
fn historical_parent_retention_keeps_existing_core_event_evidence_rules() -> TestResult {
    assert_case(Case::NonCoreEvent)
}

#[test]
fn alaska_current_athlete_stays_excluded_despite_wisconsin_venue() -> TestResult {
    let fixture = Fixture::new(Case::OutsideCurrent)?;
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    fixture.append_to(&store)?;
    let dataset = ExportDataset::load(&store)?;
    [Scope::Core, Scope::AllSources]
        .into_iter()
        .try_for_each(|scope| -> TestResult {
            let derivation = Derivation::of(&dataset, scope, Some(2027));
            check!(eq; derivation.athletes().len(), 0);
            check!(eq; derivation.performances().len(), 0);
            check!(eq; derivation.schools().len(), 0);
            check!(eq; derivation.coaches().len(), 0);
            check!(eq; derivation.meets(), std::slice::from_ref(&fixture.meet));
            check!(eq; build_from_dataset(&dataset, &options(scope))?, Vec::new());
            let census = build_census(&derivation, directory.path());
            check!(eq; census.totals.athletes, 0);
            check!(eq; census.totals.schools, 0);
            check!(eq; census.totals.coaches, 0);
            check!(eq; census.meets.total, 1);
            Ok(())
        })?;
    check!(eq; dataset.performances.as_slice(), std::slice::from_ref(&fixture.performance));
    Ok(())
}

#[test]
fn retained_historical_parents_never_expand_directory_population_or_coverage_denominator(
) -> TestResult {
    let fixture = Fixture::new(Case::AlaskaSchoolHawaiiVenue)?;
    let outside = Fixture::new(Case::OutsideCurrent)?;
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    fixture.append_to(&store)?;
    outside.append_to(&store)?;
    let dataset = ExportDataset::load(&store)?;
    [Scope::Core, Scope::AllSources].into_iter().try_for_each(|scope| -> TestResult {
        let derivation = Derivation::of(&dataset, scope, Some(2027));
        check!(eq; derivation.schools(), std::slice::from_ref(&fixture.current));
        check!(eq; derivation.athletes(), std::slice::from_ref(&fixture.athlete));
        check!(eq; derivation.coaches().len(), 0);
        check!(eq; derivation.meets(), std::slice::from_ref(&outside.meet));
        check!(eq; derivation.performances(), &[&fixture.performance]);
        assert_selection(&dataset, &fixture, scope)?;
        let census = build_census(&derivation, directory.path());
        check!(eq; census.totals.schools, 1);
        check!(eq; census.totals.athletes, 1);
        check!(eq; census.totals.class_of_2027, 1);
        check!(eq; census.totals.coaches, 0);
        check!(eq; census.meets.total, 1);
        check!(eq; census.meets.by_state.get(&MeetState::Placed(UsJurisdiction::Wisconsin)), Some(&1));
        check!(eq; census.by_state.len(), 50);
        check!(!census.by_state.contains_key(&UsJurisdiction::Alaska.into()));
        check!(!census.by_state.contains_key(&UsJurisdiction::Hawaii.into()));
        let notes = census.notes.iter().filter(|note| note.contains("is outside the census run scope:")).collect::<Vec<_>>();
        check!(eq; notes.len(), 2);
        notes.iter().try_for_each(|note| -> TestResult {
            check!(note.contains("excluded from directory/population counts; retained historical school/meet metadata may appear in selected athletes' results"));
            check!(!note.contains("publish in no row"));
            Ok(())
        })?;
        check!(notes.iter().any(|note| note.starts_with("Alaska (AK)") && note.ends_with("schools=2 athletes=1 class_of_2027=1 coaches=1 meets=0")));
        check!(notes.iter().any(|note| note.starts_with("Hawaii (HI)") && note.ends_with("schools=1 athletes=0 class_of_2027=0 coaches=1 meets=1")));
        Ok(())
    })?;
    let coverage = coverage_report(&dataset, Some(2027))?;
    check!(eq; coverage.jurisdictions.len(), 50);
    check!(eq; coverage.read, CoverageTotals { schools: 1, athletes: 1, coaches: 0, meets: 1, performances: 1 });
    check!(eq; coverage.published_totals(), coverage.read);
    check!(!coverage.jurisdictions.iter().any(|row| matches!(
        row.jurisdiction.jurisdiction(),
        Some(UsJurisdiction::Alaska | UsJurisdiction::Hawaii)
    )));
    let illinois = coverage
        .jurisdictions
        .iter()
        .find(|row| row.jurisdiction == UsJurisdiction::Illinois.into())
        .ok_or("missing Illinois coverage")?;
    check!(eq; illinois.athletes, 1);
    check!(eq; illinois.performances, 1);
    check!(eq; illinois.with_pr_support, 1);
    let note = coverage
        .notes
        .iter()
        .find(|note| note.contains("stored rows outside the census run scope"))
        .ok_or("missing population coverage exclusion note")?;
    check!(note.contains("excluded from `coverage read` and population coverage rows; retained historical school/meet metadata may appear in selected athletes' results"));
    check!(note.ends_with("schools=3 athletes=1 coaches=2 meets=1 performances=1"));
    coverage.reconcile()?;
    Ok(())
}

#[test]
fn mixed_histories_keep_exact_selected_identities_unknowns_and_limit_prefixes() -> TestResult {
    let fixtures = Case::ALL
        .into_iter()
        .map(Fixture::new)
        .collect::<TestResult<Vec<_>>>()?;
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    fixtures
        .iter()
        .rev()
        .try_for_each(|fixture| fixture.append_to(&store))?;
    let dataset = ExportDataset::load(&store)?;
    check!(eq; dataset.athletes.len(), 8);
    check!(eq; dataset.performances.len(), 8);
    check!(eq; dataset.canonical_aliases.len(), 0);
    [Scope::Core, Scope::AllSources].into_iter().try_for_each(|scope| -> TestResult {
        let derivation = Derivation::of(&dataset, scope, Some(2027));
        check!(eq; derivation.athletes().len(), 7);
        check!(eq; derivation.performances().len(), 7);
        let rows = build_from_dataset(&dataset, &options(scope))?;
        check!(eq; rows.len(), if scope == Scope::Core { 4 } else { 6 });
        fixtures.iter().try_for_each(|fixture| -> TestResult {
            let matches: Vec<_> = rows.iter().filter(|row| row.athlete_id() == &fixture.athlete.id).collect();
            check!(eq; matches.len(), usize::from(fixture.case.has_pr(scope)), "{:?}, {scope:?}", fixture.case);
            if let Some(row) = matches.first() { assert_result(row, fixture)?; }
            Ok(())
        })?;
        [0_usize, 1, 2, 4, 6, 8].into_iter().try_for_each(|limit| -> TestResult {
            let mut limited = options(scope);
            limited.limit = Some(limit);
            check!(eq; build_from_dataset(&dataset, &limited)?, rows.iter().take(limit).cloned().collect::<Vec<_>>());
            Ok(())
        })
    })
}
