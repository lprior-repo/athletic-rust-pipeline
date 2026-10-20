use super::TestResult;
use census_domain::model::{
    DiscoveryPriorityV1, Gender, GradYear, PerformanceWindowV1, ProfileDate, ProfileDateRangeV1,
    ProfilePoliciesV1, ProfileSelectionV1, ProvisionalInclusionV1, QueryProfileInputV1,
    QueryProfileV1, RegionalMembershipV1, SchoolYear, Sport,
};
use census_domain::profile_execution::{
    selected_qualification_contract_ref_v3 as reference, ContractRoleV1 as Role,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::path::{Path, PathBuf};

#[path = "fixture/golden.rs"]
mod golden;
#[path = "fixture/golden_journal.rs"]
mod golden_journal;
#[path = "fixture/results.rs"]
mod results;
#[path = "fixture/roster.rs"]
mod roster;

pub(super) struct Fixture {
    pub directory: tempfile::TempDir,
    pub root: PathBuf,
    pub selection: PathBuf,
    pub profile: QueryProfileV1,
    pub athlete: String,
    pub performance: String,
    pub public_id: String,
    pub roster_reference: String,
}

impl Fixture {
    pub async fn create() -> TestResult<Self> {
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("store");
        Store::create_public_identity_qualification_v3(&root)?;
        let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
        writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
        let performance = results::append(&store)?;
        let (_, _, roster_reference) = roster::append_for(&store, &performance.athlete)?;
        for athlete in store.snapshot().athletes()? {
            writer.establish_source_bound(
                &store,
                &format!("selected-cli-birth-{}", athlete.id),
                &athlete.id.cast(),
                store.snapshot().revision(),
                &writer.head(&store)?,
            )?;
        }
        let profile = profile()?;
        let selection = directory.path().join("selection.json");
        save_selection(&store, &profile, &selection)?;
        let accepted = store
            .snapshot()
            .accepted_public_identity_snapshot()?
            .ok_or("accepted identity")?;
        let census_domain::public_identity_history::PublicResolution::Canonical(resolved) =
            accepted.resolve_candidate(&performance.athlete.cast())?
        else {
            return Err("public identity was not canonical".into());
        };
        let public_id = resolved.public_id.as_str().to_owned();
        let athlete = performance.athlete.as_str().to_owned();
        let performance = performance.id.as_str().to_owned();
        drop(accepted);
        drop(store);
        Ok(Self {
            directory,
            root,
            selection,
            profile,
            athlete,
            performance,
            public_id,
            roster_reference,
        })
    }

    pub fn evidence_state(&self) -> TestResult<(String, String, u64, String)> {
        let (store, writer) =
            Store::open_public_identity_qualification_v3(&self.root)?.into_parts();
        let snapshot = store.snapshot();
        let digest = snapshot.tables_digest(&[
            Table::Athletes,
            Table::Schools,
            Table::Teams,
            Table::Meets,
            Table::Events,
            Table::Performances,
        ])?;
        let head = writer.head(&store)?;
        Ok((
            digest,
            serde_json::to_string(&store.store_format()?)?,
            head.sequence,
            head.digest,
        ))
    }

    pub fn select_profile(&self, input: QueryProfileInputV1, name: &str) -> TestResult<PathBuf> {
        let profile = QueryProfileV1::new(input)?;
        let (store, _writer) =
            Store::open_public_identity_qualification_v3(&self.root)?.into_parts();
        let path = self.directory.path().join(name);
        save_selection(&store, &profile, &path)?;
        Ok(path)
    }

    pub fn select_current_roster_policy(&self) -> TestResult<PathBuf> {
        use census_domain::profile_execution::selected_qualification_contract_ref_v5 as current;
        use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;
        let (store, _writer) =
            Store::open_public_identity_qualification_v3(&self.root)?.into_parts();
        let inputs = store.inspect_roster_source_enrollment_v1()?;
        let prepared = store.prepare_roster_source_enrollment_v1(
            inputs,
            census_store::RosterEnrollmentReviewV1 {
                operation: RosterAuthorityTextV1::parse("selected-cli-current-roster")?,
                actor: census_store::RosterReviewActorV1::LocalOperator,
                reviewed_at: chrono::DateTime::parse_from_rfc3339("2026-10-01T00:00:00Z")?.to_utc(),
                rationale: RosterAuthorityTextV1::parse(
                    "Enable reviewed roster capability without accepting any membership",
                )?,
            },
        )?;
        let approval = prepared.approval_bytes().to_vec();
        let digest = prepared.approval_digest().to_owned();
        let applied = store.apply_roster_source_enrollment_v1(prepared, &approval, &digest)?;
        check!(applied.written());
        let mut input = self.profile.input().clone();
        input.source_plan = current(Role::SourcePlan)?;
        input.completion_contract = current(Role::Completion)?;
        input.policies = ProfilePoliciesV1 {
            cohort: current(Role::Cohort)?,
            event: current(Role::Event)?,
            identity: current(Role::Identity)?,
            affiliation: current(Role::Affiliation)?,
            contact: current(Role::Contact)?,
            participation: current(Role::Participation)?,
            publication: current(Role::Publication)?,
        };
        let profile = QueryProfileV1::new(input)?;
        let path = self.directory.path().join("current-roster-selection.json");
        save_selection(&store, &profile, &path)?;
        Ok(path)
    }
}

fn save_selection(store: &Store, profile: &QueryProfileV1, path: &Path) -> TestResult {
    store.register_profile(profile)?;
    let selection = ProfileSelectionV1::new(store.store_identity()?, profile.id()?);
    std::fs::write(path, serde_json::to_vec(&selection)?)?;
    Ok(())
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
