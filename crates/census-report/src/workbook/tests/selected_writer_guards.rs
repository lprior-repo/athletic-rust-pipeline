use super::{athlete, school, TestResult};
use crate::export::ExportDataset;
use crate::report::{Derivation, EventAdmission, ReportError, Scope};
use crate::workbook::{Censuses, Options};
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
use std::path::PathBuf;

fn setup() -> TestResult<(tempfile::TempDir, Store, PublicIdentityWriterV1)> {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    Store::create_public_identity_qualification_v3(&root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    Ok((directory, store, writer))
}

#[test]
fn selected_workbook_without_verified_views_refuses_before_publication_effects() -> TestResult {
    for limit in [None, Some(0)] {
        let (directory, store, writer) = setup()?;
        let (dataset, _objects) = selected_dataset(&store, &writer)?;
        let root = directory.path().join("uncreated-publication");
        let options = options(Some(root.clone()), limit);
        refused(Censuses::of(&dataset, &store.out_dir(), &[]))?;
        let hdir = tempfile::tempdir()?;
        let hstore = Store::open(hdir.path().join("store"))?;
        let historical = ExportDataset::load(&hstore)?;
        let censuses = Censuses::of(&historical, &hstore.out_dir(), &[])?;
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
    let (directory, store, writer) = setup()?;
    let (dataset, _objects) = selected_dataset(&store, &writer)?;
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
    let (_directory, store, writer) = setup()?;
    let (dataset, _objects) = selected_dataset(&store, &writer)?;
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
        let (directory, store, writer) = setup()?;
        let (dataset, _objects) = selected_dataset(&store, &writer)?;
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
    let (_directory, store, writer) = setup()?;
    let (dataset, _objects) = selected_dataset(&store, &writer)?;
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
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "9901"),
    )?;
    guard_athlete.sports.push(Sport::OutdoorTrack);
    let claim = PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: SourceRef::new(
            "synthetic-guard-cohort",
            Some("https://synthetic-guard.test/athletes/9901".into()),
        ),
    };
    guard_athlete
        .evidence
        .push(Evidence::parsed(claim.source.clone(), "2026-09-20"));
    guard_athlete.published_graduations.push(claim);
    store.append(census_store::Table::Athletes, &guard_athlete)?;
    writer.establish_source_bound(
        store,
        "selected-guard-birth",
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
    dataset.strip_verified_affiliations_for_test();
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
