use super::*;
use crate::export::public_identity::tests::fixture::{merge, source_births, TestResult};
use crate::export::{
    CurrentAffiliationResolution, ExportDataset, VerifiedCurrentAffiliations,
    VerifiedParticipationView,
};
use crate::report::{Derivation, EventAdmission, Scope};
use census_domain::model::{
    CanonicalSchool, DiscoveryPriorityV1, Gender, GradYear, IdentityStatus, PerformanceWindowV1,
    ProfileDate, ProfileDateRangeV1, ProfilePoliciesV1, ProfileSelectionV1, ProvisionalInclusionV1,
    QueryProfileInputV1, QueryProfileV1, RegionalMembershipV1, RetainedConflict, SchoolYear, Sport,
    ATHLETE_IDENTITY_FAMILY,
};
use census_domain::profile_execution::{
    selected_qualification_contract_ref_v3 as reference, ContractRoleV1 as Role,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::collections::BTreeMap;

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
    let (dataset, _objects) = selected_dataset(&source.store)?;
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
    let (dataset, _objects) = selected_dataset(&source.store)?;
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
    let (dataset, _objects) = selected_dataset(&source.store)?;
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

fn selected_dataset(store: &Store) -> TestResult<(ExportDataset, tempfile::TempDir)> {
    let mut appended: std::collections::HashSet<String> = std::collections::HashSet::new();
    for athlete in store.snapshot().athletes()? {
        let (mut school, _) = CanonicalSchool::new(
            UsJurisdiction::Michigan,
            format!("Synthetic school {}", athlete.id),
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
