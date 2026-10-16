use super::{expected_sheets, Expect};
use crate::export::public_identity::tests::fixture::{merge, source_births, TestResult};
use crate::export::{
    CurrentAffiliationResolution, ExportDataset, VerifiedCurrentAffiliations,
    VerifiedParticipationView,
};
use crate::workbook::verify::expectations::Expectations;
use census_domain::model::{
    CanonicalSchool, DiscoveryPriorityV1, Gender, GradYear, PerformanceWindowV1, ProfileDate,
    ProfileDateRangeV1, ProfilePoliciesV1, ProfileSelectionV1, ProvisionalInclusionV1,
    QueryProfileInputV1, QueryProfileV1, RegionalMembershipV1, SchoolYear, Sport,
    COHORT_UNVERIFIED_FAMILY, IDENTITY_UNVERIFIED_FAMILY,
};
use census_domain::profile_execution::{
    selected_qualification_contract_ref_v3 as reference, ContractRoleV1 as Role,
};
use census_domain::UsJurisdiction;
use std::collections::BTreeMap;

mod fixture;

use fixture::{expected, field, options, validate, write};

#[test]
fn selected_meta_counts_accepted_people_and_keeps_unknown_candidate_review() -> TestResult {
    let source = source_births()?;
    let (dataset, _objects) = selected_dataset(&source.store)?;
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
    let (before, _before_objects) = selected_dataset(&source.store)?;
    merge(&source)?;
    let (after, _after_objects) = selected_dataset(&source.store)?;
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
    let (dataset, _objects) = selected_dataset(&source.store)?;
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
    let (dataset, _objects) = selected_dataset(&source.store)?;
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
    let (before, _before_objects) = selected_dataset(&source.store)?;
    crate::report::split_identity_fixture(&source, case_id)?;
    let (after, _objects) = selected_dataset(&source.store)?;
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

fn selected_dataset(store: &census_store::Store) -> TestResult<(ExportDataset, tempfile::TempDir)> {
    use census_store::Table;
    let mut appended: std::collections::HashSet<String> = std::collections::HashSet::new();
    for athlete in store.snapshot().athletes()? {
        let (mut school, _) = CanonicalSchool::new(
            UsJurisdiction::Michigan,
            &format!("Synthetic school {}", athlete.id),
            athlete.school.as_str(),
            None,
        );
        school.id = athlete.school.clone();
        school.evidence = athlete.evidence.clone();
        if appended.insert(school.id.as_str().to_owned()) {
            store.append(Table::Schools, &school).or_else(|error| {
                if error.to_string().contains("already") || error.to_string().contains("duplicate")
                {
                    Ok(())
                } else {
                    Err(error)
                }
            })?;
        }
    }
    let profile = profile()?;
    store.register_profile(&profile)?;
    let selection = ProfileSelectionV1::new(store.store_identity()?, profile.id()?);
    let objects = tempfile::tempdir()?;
    let mut dataset = ExportDataset::from_selected_snapshot(
        store,
        &store.snapshot(),
        &selection,
        objects.path(),
    )?;
    dataset.ensure_current_selected_write()?;
    let generation = dataset.lineage.input_generation.clone();
    let profile_id = profile.id()?;
    let cutoff = profile.input().as_of.clone();
    let rule = profile.input().policies.participation.clone();
    dataset.bind_verified_views_for_test(
        VerifiedParticipationView::fixture(
            generation.clone(),
            profile_id.clone(),
            rule,
            BTreeMap::new(),
        ),
        VerifiedCurrentAffiliations::fixture(
            generation,
            profile_id,
            cutoff,
            dataset
                .athletes
                .iter()
                .map(|athlete| {
                    (
                        athlete.id.clone(),
                        CurrentAffiliationResolution::Resolved {
                            school: athlete.school.clone(),
                            evidence_refs: vec![format!("synthetic-affiliation-{}", athlete.id)],
                        },
                    )
                })
                .collect(),
        ),
    )?;
    Ok((dataset, objects))
}

fn profile() -> TestResult<QueryProfileV1> {
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![UsJurisdiction::Michigan],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack],
        competition_categories: vec![Gender::Girls],
        as_of: ProfileDate::parse("2026-10-01")?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2026).ok_or("profile school year")?,
        source_plan: reference(Role::SourcePlan)?,
        acquisition_window: ProfileDateRangeV1::new(
            ProfileDate::parse("2026-01-01")?,
            ProfileDate::parse("2026-10-01")?,
        )?,
        completion_contract: reference(Role::Completion)?,
        policies: ProfilePoliciesV1 {
            cohort: reference(Role::Cohort)?,
            event: reference(Role::Event)?,
            identity: reference(Role::Identity)?,
            affiliation: reference(Role::Affiliation)?,
            contact: reference(Role::Contact)?,
            participation: reference(Role::Participation)?,
            publication: reference(Role::Publication)?,
        },
        include_provisional: ProvisionalInclusionV1::Exclude,
    })?)
}
