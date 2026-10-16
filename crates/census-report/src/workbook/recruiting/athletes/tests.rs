use super::*;
use crate::export::public_identity::tests::fixture::{merge, source_births, TestResult};
use crate::report::{Derivation, EventAdmission, Scope};
use census_domain::model::{IdentityStatus, RetainedConflict, SchoolYear, ATHLETE_IDENTITY_FAMILY};
use census_store::Table;
use std::collections::BTreeMap;

#[path = "../../publication/normalized/identity_aliases/tests/fixture.rs"]
mod fixture;

fn emitted(dataset: &crate::export::ExportDataset) -> TestResult<Vec<BTreeMap<String, Cell>>> {
    let derivation = Derivation::for_publication(dataset, Scope::AllSources, None, &[])?;
    let admission = EventAdmission::of(dataset)?;
    let recruiting = Dataset::of(
        &derivation,
        SchoolYear::new(2026).ok_or("school year")?,
        Vec::new(),
        &admission,
    )?;
    let mut rows = rows(&recruiting);
    let header = rows.next().ok_or("athlete headers")??;
    let header = header
        .into_iter()
        .map(|cell| match cell {
            Cell::Text(value) => Ok(value),
            _ => Err("non-text athlete header".into()),
        })
        .collect::<TestResult<Vec<_>>>()?;
    rows.map(|row| {
        let row = row?;
        check!(eq; row.len(), header.len());
        Ok(header.iter().cloned().zip(row).collect())
    })
    .collect()
}

#[test]
fn selected_recruiting_rows_use_public_births_and_retained_authority_without_unknown_rows(
) -> TestResult {
    let source = source_births()?;
    let dataset = fixture::selected_dataset(&source.store)?;
    let rows = emitted(&dataset)?;
    check!(eq; rows.len(), 2);
    let [first, second, unknown] = &source.athletes;
    for (athlete, birth) in [first, second].into_iter().zip(&source.births) {
        let row = rows
            .iter()
            .find(|row| row.get("Athlete ID") == Some(&Cell::text(birth.public_id().as_str())))
            .ok_or("selected athlete row")?;
        check!(eq; row.get("Canonical Candidate ID"), Some(&Cell::text(athlete.id.as_str())));
        check!(eq; row.get("Identity Status"), Some(&Cell::text("accepted_public_identity")));
        check!(eq; row.get("Identity Authority"), Some(&Cell::text("retained_public_history")));
        check!(eq; row.get("Retained Source Conflict"), Some(&Cell::Empty));
        check!(!row.contains_key("Review Status"));
        check!(!row.contains_key("Conflict Flag"));
    }
    check!(rows.iter().all(|row| {
        row.get("Canonical Candidate ID") != Some(&Cell::text(unknown.id.as_str()))
    }));
    Ok(())
}

#[test]
fn committed_merge_publishes_one_recruiting_row_using_retained_public_representative() -> TestResult
{
    let source = source_births()?;
    merge(&source)?;
    let dataset = fixture::selected_dataset(&source.store)?;
    let unchanged = serde_json::to_vec(&dataset)?;
    let rows = emitted(&dataset)?;
    let [row] = rows.as_slice() else {
        return Err("committed public merge did not publish exactly one recruiting row".into());
    };
    let [first, _, _] = &source.athletes;
    let [birth, _] = &source.births;
    check!(eq; row.get("Athlete ID"), Some(&Cell::text(birth.public_id().as_str())));
    check!(eq; row.get("Canonical Candidate ID"), Some(&Cell::text(first.id.as_str())));
    check!(eq; row.get("Identity Status"), Some(&Cell::text("accepted_public_identity")));
    check!(eq; row.get("Identity Authority"), Some(&Cell::text("retained_public_history")));
    check!(eq; serde_json::to_vec(&dataset)?, unchanged);
    Ok(())
}

#[test]
fn accepted_public_identity_preserves_retained_source_conflict_without_losing_authority(
) -> TestResult {
    let source = source_births()?;
    let [first, _, _] = &source.athletes;
    let mut conflicted = first.clone();
    conflicted.retained_conflicts.push(RetainedConflict::new(
        ATHLETE_IDENTITY_FAMILY,
        first.id.as_str(),
        "Synthetic retained source disagreement",
        "A later source conflict remains visible beside the committed public birth",
    ));
    source.store.append(Table::Athletes, &conflicted)?;
    let dataset = fixture::selected_dataset(&source.store)?;
    check!(eq; dataset.identities().status(first.id.as_str())?, IdentityStatus::RetainedConflict);
    let [birth, _] = &source.births;
    check!(eq; dataset.verified_public_identities()?.public_id(first.id.as_str()), Some(birth.public_id()));
    let rows = emitted(&dataset)?;
    let row = rows
        .iter()
        .find(|row| row.get("Athlete ID") == Some(&Cell::text(birth.public_id().as_str())))
        .ok_or("accepted conflicted athlete")?;
    check!(eq; row.get("Identity Status"), Some(&Cell::text("accepted_public_identity")));
    check!(eq; row.get("Identity Authority"), Some(&Cell::text("retained_public_history")));
    check!(eq; row.get("Retained Source Conflict"), Some(&Cell::text("yes")));
    check!(eq; dataset.athletes.iter().find(|athlete| athlete.id == first.id)
        .ok_or("retained athlete")?.retained_conflicts, conflicted.retained_conflicts);
    Ok(())
}
