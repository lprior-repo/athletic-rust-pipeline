use crate::export::{
    CurrentAffiliationResolution, ExportDataset, VerifiedCurrentAffiliations,
    VerifiedParticipationView,
};
use crate::report::{Derivation, Scope};
use crate::workbook::Options;
use census_domain::model::{
    CanonicalAthlete, CanonicalPerformance, CanonicalSchool, DiscoveryPriorityV1, Evidence, Gender,
    GradYear, PerformanceWindowV1, ProfileDate, ProfileDateRangeV1, ProfilePoliciesV1,
    ProfileSelectionV1, ProvisionalInclusionV1, PublishedGraduation, QueryProfileInputV1,
    QueryProfileV1, RegionalMembershipV1, SchoolYear, SourceIdentity, SourceNamespace, SourceRef,
    Sport,
};
use census_domain::profile_execution::{
    selected_qualification_contract_ref_v3 as reference, ContractRoleV1 as Role,
};
use census_store::{PublicIdentityWriterV1, Store, Table};
use std::collections::BTreeMap;

mod qualification;

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

pub(in crate::workbook) struct Fixture {
    pub dataset: ExportDataset,
    pub options: Options,
    _directory: tempfile::TempDir,
    _objects: tempfile::TempDir,
}

impl Fixture {
    pub fn new() -> TestResult<Self> {
        let (school, athlete) = school_and_athlete()?;
        Self::from_qualified_rows(school, vec![athlete])
    }

    pub fn from_rows(
        school: CanonicalSchool,
        athletes: Vec<CanonicalAthlete>,
        performances: Vec<CanonicalPerformance>,
    ) -> TestResult<Self> {
        let (directory, store, writer) = setup()?;
        store.append(Table::Schools, &school)?;
        store.append_many(Table::Athletes, &athletes)?;
        store.append_many(Table::Performances, &performances)?;
        finish(directory, store, writer)
    }

    fn from_qualified_rows(
        school: CanonicalSchool,
        athletes: Vec<CanonicalAthlete>,
    ) -> TestResult<Self> {
        let (directory, store, writer) = setup()?;
        store.append(Table::Schools, &school)?;
        store.append_many(Table::Athletes, &athletes)?;
        let [athlete] = athletes.as_slice() else {
            return Err("selected publication fixture qualifies exactly one athlete".into());
        };
        qualification::qualify(&store, &school, athlete)?;
        finish(directory, store, writer)
    }

    pub fn bind(&mut self) -> TestResult<()> {
        let profile = self
            .dataset
            .selected_profile_id()
            .ok_or("captured selected profile")?
            .clone();
        let cutoff = self
            .dataset
            .selected_as_of()
            .ok_or("captured selected cutoff")?
            .clone();
        let rule = self
            .dataset
            .selected_participation_rule()
            .ok_or("captured participation rule")?
            .clone();
        let generation = self.dataset.lineage.input_generation.clone();
        let participation = VerifiedParticipationView::fixture(
            generation.clone(),
            profile.clone(),
            rule,
            BTreeMap::new(),
        );
        let affiliations = VerifiedCurrentAffiliations::fixture(
            generation,
            profile,
            cutoff,
            self.dataset
                .athletes
                .iter()
                .map(|athlete| {
                    (
                        athlete.id.clone(),
                        CurrentAffiliationResolution::Resolved {
                            school: athlete.school.clone(),
                            evidence_refs: vec!["synthetic-selected-affiliation-proof".into()],
                        },
                    )
                })
                .collect(),
        );
        self.dataset
            .bind_verified_views_for_test(participation, affiliations)?;
        Ok(())
    }

    pub fn derivation(&self) -> TestResult<Derivation<'_>> {
        Ok(Derivation::selected(
            &self.dataset,
            self.options.scope,
            self.options.grad_year,
            &self.options.jurisdictions,
            self.dataset.verified_current_affiliations()?,
        )?)
    }
}

fn setup() -> TestResult<(tempfile::TempDir, Store, PublicIdentityWriterV1)> {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    Store::create_public_identity_qualification_v3(&root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    Ok((directory, store, writer))
}

fn finish(
    directory: tempfile::TempDir,
    store: Store,
    writer: PublicIdentityWriterV1,
) -> TestResult<Fixture> {
    for athlete in store.snapshot().athletes()? {
        writer.establish_source_bound(
            &store,
            &format!("selected-fixture-birth-{}", athlete.id),
            &athlete.id.cast(),
            store.snapshot().revision(),
            &writer.head(&store)?,
        )?;
    }
    let profile = michigan_profile()?;
    store.register_profile(&profile)?;
    let selection = ProfileSelectionV1::new(store.store_identity()?, profile.id()?);
    let objects = tempfile::tempdir()?;
    let dataset = ExportDataset::from_selected_snapshot(
        &store,
        &store.snapshot(),
        &selection,
        objects.path(),
    )?;
    let mut fixture = Fixture {
        dataset,
        options: Options {
            address_corpus: None,
            grad_year: Some(2027),
            out: None,
            limit: None,
            scope: Scope::AllSources,
            jurisdictions: vec![census_domain::UsJurisdiction::Michigan],
            school_year: SchoolYear::new(2026).ok_or("selected fixture school year")?,
        },
        _directory: directory,
        _objects: objects,
    };
    fixture.bind()?;
    Ok(fixture)
}

fn michigan_profile() -> TestResult<QueryProfileV1> {
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![census_domain::UsJurisdiction::Michigan],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack],
        competition_categories: vec![Gender::Girls],
        as_of: ProfileDate::parse("2026-10-01")?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2026).ok_or("selected fixture school year")?,
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

pub(in crate::workbook::publication) fn school_and_athlete(
) -> TestResult<(CanonicalSchool, CanonicalAthlete)> {
    let (school, school_id) = CanonicalSchool::new(
        census_domain::UsJurisdiction::Michigan,
        "Synthetic Selected High",
        "synthetic selected high",
        None,
    );
    let mut athlete = CanonicalAthlete::new(
        &school_id,
        "Synthetic Selected Athlete",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "9501"),
    )?;
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: SourceRef::new("synthetic-selected-cohort", None),
    });
    athlete.evidence.push(Evidence::parsed(
        SourceRef::new(
            "synthetic-selected-fixture",
            Some("https://synthetic-selected.test/athletes/9501".into()),
        ),
        "2026-10-14",
    ));
    Ok((school, athlete))
}
