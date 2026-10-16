use super::TestResult;
use crate::export::ExportDataset;
use census_domain::model::{
    CanonicalPerformance, DiscoveryPriorityV1, Gender, GradYear, PerformanceWindowV1, ProfileDate,
    ProfileDateRangeV1, ProfilePoliciesV1, ProfileSelectionV1, ProvisionalInclusionV1,
    QueryProfileInputV1, QueryProfileV1, RegionalMembershipV1, SchoolYear, Sport,
};
use census_domain::profile_execution::{
    selected_qualification_contract_ref_v3 as reference, ContractRoleV1 as Role,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::path::Path;

pub(crate) mod consumer;
mod results;
mod roster;
pub(crate) use roster::{hash, retain};
mod transfer;

enum Composition {
    SeparateRoster,
    SameAthlete,
    TwoMeets,
    MixedTwoMeets(&'static str),
    NonSchool(&'static str),
    SelectedEventScope,
}

pub(crate) struct Fixture {
    pub directory: tempfile::TempDir,
    pub store: Store,
    pub selection: ProfileSelectionV1,
    pub performance: CanonicalPerformance,
    pub roster_athlete: census_domain::model::AthleteId,
    pub roster_school: census_domain::model::SchoolId,
}

impl Fixture {
    pub async fn selected_event_scope() -> TestResult<Self> {
        Self::prepare(Composition::SelectedEventScope).await
    }

    pub async fn new() -> TestResult<Self> {
        Self::prepare(Composition::SeparateRoster).await
    }

    pub async fn composed(two_meets: bool) -> TestResult<Self> {
        let composition = if two_meets {
            Composition::TwoMeets
        } else {
            Composition::SameAthlete
        };
        Self::prepare(composition).await
    }

    pub async fn append_second_meet(&self) -> TestResult {
        results::append_second(&self.store).await?;
        Ok(())
    }

    pub async fn non_school(division: &'static str) -> TestResult<Self> {
        Self::prepare(Composition::NonSchool(division)).await
    }

    pub async fn mixed_two_meets(division: &'static str) -> TestResult<Self> {
        Self::prepare(Composition::MixedTwoMeets(division)).await
    }

    async fn prepare(composition: Composition) -> TestResult<Self> {
        let directory = super::temporary()?;
        let root = directory.path().join("store");
        Store::create_public_identity_qualification_v3(&root)?;
        let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
        writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
        let performance = match composition {
            Composition::SelectedEventScope => results::selected::append_baseline(&store).await?,
            Composition::NonSchool(division) => results::append_division(&store, division).await?,
            _ => results::append(&store).await?,
        };
        let (roster_athlete, roster_school) = match composition {
            Composition::SelectedEventScope => {
                results::selected::append_outside_scope(&store).await?;
                roster::append_for(&store, &performance.athlete)?
            }
            Composition::SeparateRoster => roster::append(&store)?,
            Composition::SameAthlete | Composition::NonSchool(_) => {
                roster::append_for(&store, &performance.athlete)?
            }
            Composition::TwoMeets => {
                results::append_second(&store).await?;
                roster::append_for(&store, &performance.athlete)?
            }
            Composition::MixedTwoMeets(division) => {
                results::append_second_division(&store, division).await?;
                roster::append_for(&store, &performance.athlete)?
            }
        };
        let profile = profile()?;
        store.register_profile(&profile)?;
        for athlete in store.snapshot().athletes()? {
            writer.establish_source_bound(
                &store,
                &format!("selected-proof-source-birth-{}", athlete.id),
                &athlete.id.cast(),
                store.snapshot().revision(),
                &writer.head(&store)?,
            )?;
        }
        let selection = ProfileSelectionV1::new(store.store_identity()?, profile.id()?);
        Ok(Self {
            directory,
            store,
            selection,
            performance,
            roster_athlete,
            roster_school,
        })
    }

    pub fn capture(&self, objects: &Path) -> TestResult<ExportDataset> {
        Ok(ExportDataset::from_selected_snapshot(
            &self.store,
            &self.store.snapshot(),
            &self.selection,
            objects,
        )?)
    }

    pub fn remove_sources(self) -> TestResult {
        let Self {
            directory, store, ..
        } = self;
        drop(store);
        directory.close()?;
        Ok(())
    }
}

pub(super) fn profile() -> TestResult<QueryProfileV1> {
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![UsJurisdiction::Alabama, UsJurisdiction::Wisconsin],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack],
        competition_categories: vec![Gender::Girls],
        as_of: ProfileDate::parse("2026-10-01")?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2026).ok_or("school year")?,
        source_plan: reference(Role::SourcePlan)?,
        acquisition_window: ProfileDateRangeV1::new(
            ProfileDate::parse("2026-01-01")?,
            ProfileDate::parse("2026-10-14")?,
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
