use super::{expected_sheets, Expect};
use crate::export::public_identity::tests::fixture::{merge, source_births, TestResult};
use crate::workbook::verify::expectations::Expectations;
use census_domain::model::{COHORT_UNVERIFIED_FAMILY, IDENTITY_UNVERIFIED_FAMILY};

mod fixture;
#[path = "../../publication/normalized/identity_aliases/tests/fixture.rs"]
mod selected;

use fixture::{expected, field, options, validate, write};

#[test]
fn selected_meta_counts_accepted_people_and_keeps_unknown_candidate_review() -> TestResult {
    let source = source_births()?;
    let dataset = selected::selected_dataset(&source.store)?;
    let rows = expected(&dataset, "Run Metrics")?;
    check!(eq; field(&rows, "Frozen table", 1)?, "Retained records");
    check!(eq; field(&rows, "athlete candidates", 1)?, "3");
    check!(eq; field(&rows, "Accepted candidate records", 1)?, "2");
    check!(eq; field(&rows, "Withheld or unknown candidate records", 1)?, "1");
    check!(eq; field(&rows, "Accepted public people", 1)?, "2");
    check!(eq; field(&rows, "Athletes", 1)?, "2");
    check!(eq; field(&rows, "Athletes", 2)?, "2");
    check!(eq; field(&rows, "Athletes", 3)?, "reconciled");
    let review = expected(&dataset, "Review")?;
    let withheld = review
        .iter()
        .find(|row| {
            row.first()
                .is_some_and(|label| label == "Public identity candidate withheld")
        })
        .ok_or("unknown candidate review")?;
    check!(eq; withheld.get(2).ok_or("candidate key")?, source.athletes[2].id.as_str());
    check!(withheld
        .get(6)
        .is_some_and(|detail| detail.contains("does not establish a distinct person")));
    check!(!review.iter().any(|row| row
        .first()
        .is_some_and(|label| label == IDENTITY_UNVERIFIED_FAMILY)));
    check!(eq; review.iter().filter(|row| row.first().is_some_and(|label| label == COHORT_UNVERIFIED_FAMILY)).count(), 2);
    let path = source.directory.path().join("selected-meta.xlsx");
    write(&dataset, &path)?;
    validate(&dataset, &path)?;
    Ok(())
}

#[test]
fn selected_meta_merge_changes_people_without_changing_candidate_audit() -> TestResult {
    let source = source_births()?;
    let before = selected::selected_dataset(&source.store)?;
    merge(&source)?;
    let after = selected::selected_dataset(&source.store)?;
    let rows = expected(&after, "Run Metrics")?;
    check!(eq; field(&rows, "Accepted public people", 1)?, "1");
    check!(eq; field(&rows, "Accepted candidate records", 1)?, "2");
    check!(eq; field(&rows, "Withheld or unknown candidate records", 1)?, "1");
    check!(eq; field(&rows, "Athletes", 1)?, "1");
    check!(eq; field(&rows, "Athletes", 2)?, "1");
    check!(eq; field(&expected(&before, "Run Metrics")?, "Accepted public people", 1)?, "2");
    let path = source.directory.path().join("merged-meta.xlsx");
    write(&after, &path)?;
    validate(&after, &path)?;
    Ok(())
}

#[test]
fn selected_meta_readback_rejects_unknown_candidate_key_replaced_by_accepted_key() -> TestResult {
    let source = source_births()?;
    let dataset = selected::selected_dataset(&source.store)?;
    let expectations = Expectations::of(&dataset, &options()?)?;
    let sheets = expected_sheets(&expectations)?;
    let (name, mut rows) = sheets
        .into_iter()
        .find(|(name, _)| *name == "Review")
        .ok_or("review")?;
    let row = rows.iter_mut().find(|row| matches!(row.first(), Some(Expect::Text(label)) if label == "Public identity candidate withheld")).ok_or("withheld candidate")?;
    *row.get_mut(2).ok_or("candidate key")? = Expect::text(source.athletes[0].id.as_str());
    fixture::reject(&dataset, source.directory.path(), name, &rows, "Review!")
}

#[test]
fn selected_meta_readback_rejects_raw_candidate_total_as_accepted_people() -> TestResult {
    let source = source_births()?;
    let dataset = selected::selected_dataset(&source.store)?;
    let expectations = Expectations::of(&dataset, &options()?)?;
    let (name, mut rows) = expected_sheets(&expectations)?
        .into_iter()
        .find(|(name, _)| *name == "Run Metrics")
        .ok_or("metrics")?;
    let row = rows.iter_mut().find(|row| matches!(row.first(), Some(Expect::Text(label)) if label == "Accepted public people")).ok_or("people count")?;
    *row.get_mut(1).ok_or("people value")? = Expect::Number(3.0);
    fixture::reject(
        &dataset,
        source.directory.path(),
        name,
        &rows,
        "Run Metrics!",
    )
}

#[test]
fn selected_meta_local_split_restores_people_and_keeps_candidate_review() -> TestResult {
    let source = source_births()?;
    let reviewed = crate::export::public_identity::tests::fixture::prepare_merge(&source)?;
    let census_store::ReviewedIdentityRequestV1::Merge { case_id, .. } = &reviewed else {
        return Err("expected reviewed merge fixture".into());
    };
    source.writer.apply_reviewed_identity(
        &source.store,
        "meta-before-local-split",
        &reviewed,
        source.store.snapshot().revision(),
        &source.writer.head(&source.store)?,
    )?;
    let before = selected::selected_dataset(&source.store)?;
    crate::report::split_identity_fixture(&source, case_id)?;
    let after = selected::selected_dataset(&source.store)?;
    let rows = expected(&after, "Run Metrics")?;
    check!(eq; field(&rows, "Accepted public people", 1)?, "2");
    check!(eq; field(&rows, "Accepted candidate records", 1)?, "2");
    check!(eq; field(&rows, "Withheld or unknown candidate records", 1)?, "1");
    check!(eq; field(&rows, "Athletes", 1)?, "2");
    check!(eq; field(&rows, "Athletes", 2)?, "2");
    check!(eq; field(&expected(&before, "Run Metrics")?, "Accepted public people", 1)?, "1");
    let path = source.directory.path().join("split-meta.xlsx");
    write(&after, &path)?;
    validate(&after, &path)?;
    Ok(())
}
