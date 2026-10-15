use super::{athlete, school, TestResult};
use crate::export::ExportDataset;
use crate::report::{Derivation, EventAdmission, ReportError, Scope};
use crate::workbook::{Censuses, Options};
use census_domain::model::{ContractRefV1, ProfileDate, ProfileId, SchoolYear};
use census_store::Store;
use std::path::PathBuf;

#[test]
fn selected_workbook_without_verified_views_refuses_before_publication_effects() -> TestResult {
    for limit in [None, Some(0)] {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path().join("store"))?;
        let dataset = selected_dataset(&store)?;
        let root = directory.path().join("uncreated-publication");
        let options = options(Some(root.clone()), limit);
        refused(Censuses::of(&dataset, &store.out_dir(), &[]))?;
        let historical = ExportDataset::load(&store)?;
        let censuses = Censuses::of(&historical, &store.out_dir(), &[])?;
        let before = store.snapshot().sequence();
        refused(crate::workbook::build_from(
            &dataset, &store, &options, &censuses,
        ))?;
        check!(!root.exists());
        check!(eq; store.snapshot().sequence(), before);
    }
    Ok(())
}

#[test]
fn selected_recruiting_csv_cannot_fall_back_to_historical_affiliation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let dataset = selected_dataset(&store)?;
    let path = directory.path().join("recruiting.csv");
    let previous = b"prior recruiting artifact remains intact\n";
    std::fs::write(&path, previous)?;
    refused(
        Derivation::of(&dataset, Scope::AllSources, Some(2027), &[]).and_then(|derivation| {
            crate::workbook::write_recruiting_csv(&derivation, SchoolYear::DEFAULT, &path)
        }),
    )?;
    check!(eq; std::fs::read(path)?.as_slice(), previous);
    Ok(())
}

#[test]
fn selected_recruiting_projection_rejects_naked_empty_bests() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let dataset = selected_dataset(&store)?;
    let admission = EventAdmission::of(&dataset)?;
    refused(
        Derivation::of(&dataset, Scope::AllSources, Some(2027), &[]).and_then(|derivation| {
            crate::workbook::recruiting::Recruiting::of(
                &derivation,
                SchoolYear::DEFAULT,
                Vec::new(),
                &admission,
            )
            .map(|_| ())
        }),
    )
}

#[test]
fn selected_normalized_writer_without_verified_views_refuses_even_at_zero_limit() -> TestResult {
    for limit in [None, Some(0)] {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path().join("store"))?;
        let dataset = selected_dataset(&store)?;
        let destination = directory.path().join("uncreated-normalized");
        refused(crate::workbook::publication::normalized::write(
            &destination,
            &dataset,
            &options(None, limit),
            &[],
            &[],
        ))?;
        check!(!destination.exists());
    }
    Ok(())
}

#[test]
fn selected_unverified_bests_reducers_refuse_before_zero_limit_can_hide_evidence() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let dataset = selected_dataset(&store)?;
    let admission = EventAdmission::of(&dataset)?;
    for limit in [None, Some(0)] {
        let options = crate::bests::Options {
            scope: Scope::AllSources,
            limit,
            ..crate::bests::Options::default()
        };
        refused(crate::bests::build_from_dataset(&dataset, &options))?;
        refused(crate::bests::build_with_exclusions(&dataset, &options))?;
        refused(crate::bests::build_admitted_outcome(&admission, &options))?;
        refused(
            Derivation::of(&dataset, Scope::AllSources, Some(2027), &[]).and_then(|derivation| {
                crate::bests::build_from_derivation(&derivation, &admission, limit)
            }),
        )?;
    }
    Ok(())
}

#[test]
fn historical_recruiting_csv_still_supports_an_empty_result_population() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let school_id = school(&store, "Synthetic School", None)?;
    athlete(&store, &school_id, "Synthetic Runner")?;
    let dataset = ExportDataset::load(&store)?;
    let derivation = Derivation::of(&dataset, Scope::AllSources, Some(2027), &[])?;
    let path = directory.path().join("historical-recruiting.csv");
    crate::workbook::write_recruiting_csv(&derivation, SchoolYear::DEFAULT, &path)?;
    check!(path.is_file());
    let records = csv::Reader::from_path(path)?
        .records()
        .collect::<Result<Vec<_>, _>>()?;
    check!(eq; records.len(), 1);
    Ok(())
}

fn selected_dataset(store: &Store) -> TestResult<ExportDataset> {
    let school_id = school(store, "Synthetic School", None)?;
    athlete(store, &school_id, "Synthetic Runner")?;
    let mut dataset = ExportDataset::load(store)?;
    dataset.bind_selection_for_test(
        ProfileId::parse(&"a".repeat(64))?,
        ProfileDate::parse("2026-10-01")?,
    )?;
    dataset.bind_participation_rule_for_test(ContractRefV1::new(1, 1, &"b".repeat(64))?)?;
    Ok(dataset)
}

fn options(out: Option<PathBuf>, limit: Option<usize>) -> Options {
    Options {
        grad_year: Some(2027),
        out,
        limit,
        scope: Scope::AllSources,
        jurisdictions: Vec::new(),
        school_year: SchoolYear::DEFAULT,
    }
}

fn refused<T>(result: crate::report::ReportResult<T>) -> TestResult {
    match result {
        Err(ReportError::Invariant { detail }) if detail.contains("selected") => Ok(()),
        Err(error) => Err(format!("wrong refusal for missing selected views: {error}").into()),
        Ok(_) => Err("selected output accepted without its verified publication views".into()),
    }
}
