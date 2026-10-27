use super::TestResult;
use crate::export::{ExportDataset, VerifiedCurrentAffiliations, VerifiedParticipationView};
use crate::report::{ReportError, Scope};
use crate::workbook::Options;
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, DiscoveryPriorityV1, Evidence, Gender, GradYear,
    PerformanceWindowV1, ProfileDate, ProfileDateRangeV1, ProfilePoliciesV1, ProfileSelectionV1,
    ProvisionalInclusionV1, PublishedGraduation, QueryProfileInputV1, QueryProfileV1,
    RegionalMembershipV1, SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::profile_execution::{
    selected_qualification_contract_ref_v3 as reference, ContractRoleV1 as Role,
};
use census_domain::UsJurisdiction;
use census_store::{PublicIdentityWriterV1, Store};
use std::collections::BTreeMap;

#[test]
fn naked_normalized_writer_refuses_even_fully_verified_selected_zero_limit() -> TestResult {
    for limit in [None, Some(0)] {
        let (directory, store, writer) = setup()?;
        let (dataset, _objects) = selected_dataset(&store, &writer)?;
        dataset.verified_participation()?;
        dataset.verified_current_affiliations()?;
        let output = directory.path().join("uncreated");
        let result = crate::workbook::publication::normalized::write(
            &output,
            &dataset,
            &Options {
                address_corpus: None,
                grad_year: None,
                out: None,
                limit,
                scope: Scope::AllSources,
                jurisdictions: Vec::new(),
                school_year: SchoolYear::DEFAULT,
            },
            &[],
            &[],
        );
        match result {
            Err(ReportError::Invariant { detail }) => {
                check!(eq; detail, "selected participation policy requires verified frozen evidence");
            }
            other => return Err(format!("unexpected naked normalized result: {other:?}").into()),
        }
        check!(!output.exists());
    }
    Ok(())
}

fn setup() -> TestResult<(tempfile::TempDir, Store, PublicIdentityWriterV1)> {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    Store::create_public_identity_qualification_v3(&root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    Ok((directory, store, writer))
}

fn selected_dataset(
    store: &Store,
    writer: &PublicIdentityWriterV1,
) -> TestResult<(ExportDataset, tempfile::TempDir)> {
    let (school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Synthetic Guard High",
        "synthetic guard high",
        None,
    );
    store.append(census_store::Table::Schools, &school)?;
    let mut guard_athlete = CanonicalAthlete::new(
        &school_id,
        "Synthetic Guard Runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "9902"),
    )?;
    guard_athlete.sports.push(Sport::OutdoorTrack);
    let claim = PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: SourceRef::new(
            "synthetic-guard-cohort",
            Some("https://synthetic-guard.test/athletes/9902".into()),
        ),
    };
    guard_athlete
        .evidence
        .push(Evidence::parsed(claim.source.clone(), "2026-09-20"));
    guard_athlete.published_graduations.push(claim);
    store.append(census_store::Table::Athletes, &guard_athlete)?;
    writer.establish_source_bound(
        store,
        "selected-verified-guard-birth",
        &guard_athlete.id.cast(),
        store.snapshot().revision(),
        &writer.head(store)?,
    )?;
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
        VerifiedCurrentAffiliations::fixture(generation, profile_id, cutoff, BTreeMap::new()),
    )?;
    Ok((dataset, objects))
}

fn profile() -> TestResult<QueryProfileV1> {
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![UsJurisdiction::Wisconsin],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack],
        competition_categories: vec![Gender::Boys],
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
