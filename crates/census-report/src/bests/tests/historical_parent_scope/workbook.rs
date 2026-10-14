use super::*;
use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use census_domain::model::SchoolYear;
use serde_json::Value;
use std::path::{Path, PathBuf};

fn publish(directory: &Path, fixture: &Fixture) -> TestResult<(Store, PathBuf)> {
    let store = Store::open(directory.join("store"))?;
    fixture.append_to(&store)?;
    Fixture::new(Case::OutsideCurrent)?.append_to(&store)?;
    let path = crate::workbook::build(&store, &publication_options(directory, Scope::AllSources)?)?;
    Ok((store, path))
}

fn publication_options(directory: &Path, scope: Scope) -> TestResult<crate::workbook::Options> {
    Ok(crate::workbook::Options {
        grad_year: Some(2027),
        out: Some(directory.join("publication")),
        limit: None,
        scope,
        jurisdictions: Vec::new(),
        school_year: SchoolYear::new(2026).ok_or("invalid synthetic publication year")?,
    })
}

fn assert_frozen(path: &Path, fixture: &Fixture, scope: Scope) -> TestResult {
    let generation = path
        .parent()
        .ok_or("missing historical-parent generation")?;
    let dataset = ExportDataset::reopen_frozen(&generation.join("frozen-input.json"))?;
    check!(eq; dataset.lineage.schema_revision, 3);
    check!(eq; dataset.lineage.policy_revision, 18);
    check!(eq; dataset.athletes.len(), 2);
    check!(eq; dataset.performances.len(), 2);
    check!(eq; dataset.canonical_aliases.len(), 0);
    check!(dataset.athletes.contains(&fixture.athlete));
    check!(dataset.performances.contains(&fixture.performance));
    assert_projection(&dataset, fixture, scope)?;
    if fixture.case.has_pr(scope) {
        assert_selection(&dataset, fixture, scope)?;
    } else {
        check!(eq; build_from_dataset(&dataset, &options(scope))?, Vec::new());
    }
    let verified = crate::workbook::verify::verify_frozen(
        path,
        &dataset,
        &publication_options(generation, scope)?,
    )?;
    check!(eq; verified.mapped_athletes, 1);
    drop(dataset);
    check!(eq; crate::workbook::publication::verify_published(path)?.workbook.mapped_athletes, 1);
    artifacts::bound_generation(generation)
}

fn assert_prs(range: &Range<Data>, fixture: &Fixture, scope: Scope) -> TestResult {
    if !fixture.case.has_pr(scope) {
        check!(eq; range.height(), 1);
        return Ok(());
    }
    artifacts::xlsx_single(
        range,
        &[
            ("Athlete ID", fixture.athlete.id.as_str()),
            ("Athlete", fixture.case.athlete_name()),
            ("School", fixture.case.school_text()),
            ("State", fixture.case.school_bucket().code()),
            ("Meet", fixture.case.meet_name()),
            ("Result URL", RESULT_URL),
            ("Performance ID", fixture.performance.id.as_str()),
            ("Meet ID", fixture.meet.id.as_str()),
            ("Source Key", fixture.performance.source_key.as_str()),
        ],
    )?;
    artifacts::xlsx_golden(range, "xlsx_prs")?;
    check!(eq; artifacts::cell(range, "Mark Value")?, &Data::Float(48.55));
    Ok(())
}

fn assert_performances(range: &Range<Data>, fixture: &Fixture, scope: Scope) -> TestResult {
    let (meet, state) = fixture.case.meet_metadata(scope);
    artifacts::xlsx_single(
        range,
        &[
            ("Canonical Result ID", fixture.performance.id.as_str()),
            ("Athlete ID", fixture.athlete.id.as_str()),
            ("Athlete", fixture.case.athlete_name()),
            ("School", fixture.case.school_text()),
            ("Meet ID", fixture.meet.id.as_str()),
            ("Meet", meet),
            ("State", state.map_or("", core::convert::identity)),
            (
                "Sport",
                if fixture.case.event_label(scope).is_empty() {
                    ""
                } else {
                    "Track"
                },
            ),
            ("Event", fixture.case.event_label(scope)),
            ("Source ResultID", fixture.performance.source_key.as_str()),
            ("Source URL", RESULT_URL),
        ],
    )?;
    artifacts::xlsx_golden(range, "xlsx_performances")
}

fn assert_directory(
    book: &mut Xlsx<std::io::BufReader<std::fs::File>>,
    fixture: &Fixture,
    scope: Scope,
) -> TestResult {
    artifacts::xlsx_single(
        &book.worksheet_range("Athletes")?,
        &[
            ("Athlete ID", fixture.athlete.id.as_str()),
            ("Name", fixture.case.athlete_name()),
            ("School", "Current Illinois High"),
            ("School ID", fixture.current.id.as_str()),
            ("State", "IL"),
            ("MileSplit URL", PROFILE_URL),
        ],
    )?;
    artifacts::xlsx_single(
        &book.worksheet_range("Schools")?,
        &[
            ("School ID", fixture.current.id.as_str()),
            ("School", "Current Illinois High"),
            ("State", "IL"),
        ],
    )?;
    check!(eq; book.worksheet_range("Coaches")?.height(), 1);
    assert_metrics_notes(&book.worksheet_range("Run Metrics")?)?;
    let athletes = book.worksheet_range("Athletes")?;
    if fixture.case.has_pr(scope) {
        check!(eq; artifacts::cell(&athletes, "Headline PR summary")?.to_string(), "400m 48.55 [outdoor, na, fat]");
        check!(eq; artifacts::cell(&athletes, "Coverage State")?.to_string(), "pr");
    }
    Ok(())
}

fn assert_metrics_notes(range: &Range<Data>) -> TestResult {
    let notes = range
        .rows()
        .filter(|row| {
            row.first()
                .is_some_and(|cell| cell == "Population geography and historical results")
        })
        .collect::<Vec<_>>();
    check!(eq; notes.len(), 2);
    notes.iter().try_for_each(|row| -> TestResult {
        let note = row.get(1).ok_or("missing historical-parent Run Metrics note")?.to_string();
        check!(note.contains("excluded from directory/population counts; retained historical school/meet metadata may appear in selected athletes' results"));
        check!(!note.contains("publish in no row"));
        Ok(())
    })
}

fn assert_sidecars(generation: &Path, fixture: &Fixture, scope: Scope) -> TestResult {
    artifacts::assert_bests(generation, fixture, fixture.case.has_pr(scope))?;
    artifacts::csv_single(
        &generation.join("recruiting.csv"),
        &[
            ("athlete_id", fixture.athlete.id.as_str()),
            ("name", fixture.case.athlete_name()),
            ("school", "Current Illinois High"),
            ("state", "IL"),
            ("grad_year", "2027"),
            ("milesplit_url", PROFILE_URL),
        ],
    )?;
    let audit: Value = serde_json::from_slice(&std::fs::read(generation.join("audit.json"))?)?;
    check!(eq; audit.get("athletes"), Some(&serde_json::json!([fixture.athlete.id.as_str()])));
    check!(eq; audit.get("performances"), Some(&serde_json::json!([fixture.performance.id.as_str()])));
    check!(eq; audit.get("coaches"), Some(&serde_json::json!([])));
    ["census-core.json", "census-all-sources.json"].into_iter().try_for_each(|name| -> TestResult {
        let census: Value = serde_json::from_slice(&std::fs::read(generation.join(name))?)?;
        check!(eq; census.pointer("/totals/schools").and_then(Value::as_u64), Some(1));
        check!(eq; census.pointer("/totals/athletes").and_then(Value::as_u64), Some(1));
        check!(eq; census.pointer("/totals/class_of_2027").and_then(Value::as_u64), Some(1));
        check!(eq; census.pointer("/totals/coaches").and_then(Value::as_u64), Some(0));
        check!(eq; census.pointer("/meets/total").and_then(Value::as_u64), Some(1));
        check!(eq; census.pointer("/meets/by_state/WI").and_then(Value::as_u64), Some(1));
        check!(eq; census.get("by_state").and_then(Value::as_object).map(|rows| rows.len()), Some(50));
        check!(eq; census.pointer("/by_state/AK"), None);
        check!(eq; census.pointer("/by_state/HI"), None);
        let notes = census.get("notes").and_then(Value::as_array).ok_or("missing published Census notes")?;
        let outside = notes.iter().filter_map(Value::as_str).filter(|note| note.contains("is outside the census run scope:")).collect::<Vec<_>>();
        check!(eq; outside.len(), 2);
        outside.into_iter().try_for_each(|note| -> TestResult {
            check!(note.contains("excluded from directory/population counts; retained historical school/meet metadata may appear in selected athletes' results"));
            check!(!note.contains("publish in no row"));
            Ok(())
        })?;
        Ok(())
    })
}

fn assert_case(case: Case, scope: Scope) -> TestResult {
    let fixture = Fixture::new(case)?;
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    fixture.append_to(&store)?;
    Fixture::new(Case::OutsideCurrent)?.append_to(&store)?;
    let path = crate::workbook::build(&store, &publication_options(directory.path(), scope)?)?;
    let generation = path
        .parent()
        .ok_or("missing published historical-parent generation")?;
    assert_frozen(&path, &fixture, scope)?;
    let mut book: Xlsx<_> = open_workbook(&path)?;
    assert_prs(&book.worksheet_range("PRs")?, &fixture, scope)?;
    assert_performances(&book.worksheet_range("Performances_001")?, &fixture, scope)?;
    assert_directory(&mut book, &fixture, scope)?;
    assert_sidecars(generation, &fixture, scope)
}

#[test]
fn published_alaska_history_and_hawaii_venue_survive_csv_jsonl_xlsx_and_frozen_readback(
) -> TestResult {
    [Scope::Core, Scope::AllSources]
        .into_iter()
        .try_for_each(|scope| assert_case(Case::AlaskaSchoolHawaiiVenue, scope))
}

#[test]
fn published_hawaii_history_and_alaska_venue_survive_csv_jsonl_xlsx_and_frozen_readback(
) -> TestResult {
    [Scope::Core, Scope::AllSources]
        .into_iter()
        .try_for_each(|scope| assert_case(Case::HawaiiSchoolAlaskaVenue, scope))
}

#[test]
fn published_unknown_parents_and_noncore_context_keep_exact_scope_outcomes() -> TestResult {
    [
        Case::MissingTeam,
        Case::MissingSchool,
        Case::MissingMeet,
        Case::NonCoreMeet,
        Case::NonCoreEvent,
    ]
    .into_iter()
    .try_for_each(|case| -> TestResult {
        [Scope::Core, Scope::AllSources]
            .into_iter()
            .try_for_each(|scope| assert_case(case, scope))
    })
}

#[test]
fn historical_publication_verification_uses_frozen_parents_after_live_store_metadata_changes(
) -> TestResult {
    let fixture = Fixture::new(Case::AlaskaSchoolHawaiiVenue)?;
    let directory = tempfile::tempdir()?;
    let (store, path) = publish(directory.path(), &fixture)?;
    let generation = path
        .parent()
        .ok_or("missing original historical-parent generation")?;
    let frozen = std::fs::read(generation.join("frozen-input.json"))?;
    let workbook = std::fs::read(&path)?;
    let mut later_meet = fixture.meet.clone();
    later_meet.name = "Later Wisconsin meet label".into();
    later_meet.state = Some(UsJurisdiction::Wisconsin);
    store.append(Table::Meets, &later_meet)?;
    let mut later_school = fixture.historical.clone();
    later_school.name = "Later Wisconsin school label".into();
    later_school.state = Some(UsJurisdiction::Wisconsin);
    store.append(Table::Schools, &later_school)?;
    drop(store);
    assert_frozen(&path, &fixture, Scope::AllSources)?;
    let mut book: Xlsx<_> = open_workbook(&path)?;
    assert_prs(&book.worksheet_range("PRs")?, &fixture, Scope::AllSources)?;
    assert_performances(
        &book.worksheet_range("Performances_001")?,
        &fixture,
        Scope::AllSources,
    )?;
    check!(eq; std::fs::read(generation.join("frozen-input.json"))?, frozen);
    check!(eq; std::fs::read(&path)?, workbook);
    Ok(())
}
