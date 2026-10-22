use super::*;
use crate::workbook::publication::inventory;
use census_domain::model::{ProvisionalInclusionV1, QueryProfileInputV1, QueryProfileV1};
use std::error::Error;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

#[test]
fn reported_revision_is_selected_only_by_the_frozen_inclusion_setting() -> TestResult {
    let fixture = fixture::Fixture::new()?;
    let selected = fixture.dataset.selected_authority()?.profile();
    let excluded = selected.clone();
    let included = QueryProfileV1::new(QueryProfileInputV1 {
        include_provisional: ProvisionalInclusionV1::Include,
        ..selected.input().clone()
    })?;

    check!(eq; current_revision_from_profile(&excluded), REVISION);
    check!(eq; current_revision_from_profile(&included), REPORTED_REVISION);
    check!(ne; excluded.id()?, included.id()?);
    check!(eq; included.input().graduation_years, excluded.input().graduation_years);
    check!(eq; included.input().jurisdictions, excluded.input().jurisdictions);
    check!(eq; included.input().sports, excluded.input().sports);
    check!(eq; included.input().competition_categories, excluded.input().competition_categories);
    check!(eq; included.input().policies, excluded.input().policies);
    Ok(())
}

#[test]
fn reported_inventory_adds_only_reported_performances_and_requires_exact_counts() -> TestResult {
    let revision_eight = inventory::names(REVISION)?;
    let revision_nine = inventory::names(REPORTED_REVISION)?;
    check!(eq; revision_eight.len() + 1, revision_nine.len());
    check!(!revision_eight.contains(&"reported_performances.csv"));
    check!(revision_nine.contains(&"reported_performances.csv"));
    check!(eq;
        revision_nine.iter().copied()
            .filter(|name| *name != "reported_performances.csv")
            .collect::<Vec<_>>(),
        revision_eight,
    );

    let counts: inventory::RowCounts = revision_nine
        .iter()
        .map(|name| ((*name).to_owned(), 0))
        .collect();
    inventory::validate(REPORTED_REVISION, Some(&counts))?;
    inventory::compare(REPORTED_REVISION, Some(&counts), &counts)?;

    check!(inventory::validate(REPORTED_REVISION, None).is_err());
    let mut missing = counts.clone();
    missing.remove("reported_performances.csv");
    check!(inventory::validate(REPORTED_REVISION, Some(&missing)).is_err());
    let mut extra = counts.clone();
    extra.insert("unregistered.csv".to_owned(), 0);
    check!(inventory::validate(REPORTED_REVISION, Some(&extra)).is_err());
    let mut incorrect = counts.clone();
    incorrect.insert("reported_performances.csv".to_owned(), 1);
    check!(inventory::compare(REPORTED_REVISION, Some(&counts), &incorrect).is_err());

    let excluded_counts: inventory::RowCounts = revision_eight
        .iter()
        .map(|name| ((*name).to_owned(), 0))
        .collect();
    inventory::validate(REVISION, Some(&excluded_counts))?;
    check!(inventory::validate(REVISION, Some(&counts)).is_err());
    Ok(())
}

#[test]
fn reported_revision_binding_requires_a_selected_profile_binding() -> TestResult {
    let fixture = fixture::Fixture::new()?;
    let selected = SelectedProfile::of(&fixture.dataset)?;
    check!(binding(REVISION, &fixture.dataset)?.is_some());
    check!(binding(REPORTED_REVISION, &fixture.dataset).is_err());
    check!(validate(REPORTED_REVISION, Some(&selected)).is_ok());
    check!(validate(REPORTED_REVISION, None).is_err());
    check!(validate(REVISION, None).is_err());
    check!(validate(9, Some(&selected)).is_err());
    check!(inventory::names(9).is_err());
    Ok(())
}

#[path = "reported_tests/end_to_end.rs"]
mod end_to_end;
