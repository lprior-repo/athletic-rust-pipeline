use super::*;
use census_crawl::milesplit::{roster_entities, Site, TeamRef};
use census_crawl::participation_evidence::{
    provider_school::ProviderSchoolPacketV1, RetainedBodyEncodingV1, RetainedHtmlCaptureV1,
    RetainedHttpProvenanceV1,
};
use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;
use census_domain::UsJurisdiction;
use census_store::{RosterEnrollmentReviewV1, RosterReviewActorV1};

pub(super) const AT: &str = "2026-10-16T00:49:43Z";

pub(super) struct Fixture {
    pub directory: tempfile::TempDir,
    pub store: Store,
    pub writer: census_store::PublicIdentityWriterV1,
    pub profile: QueryProfileV1,
}

pub(super) struct Capture {
    pub packet: ProviderSchoolPacketV1,
    pub school: CanonicalSchool,
    pub program: CanonicalTeam,
    pub athlete: CanonicalAthlete,
}

impl Fixture {
    pub fn new() -> TestResult<Self> {
        let directory = tempfile::tempdir()?;
        Store::create_public_identity_qualification_v3(directory.path())?;
        let (store, writer) =
            Store::open_public_identity_qualification_v3(directory.path())?.into_parts();
        writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
        let inputs = store.inspect_roster_source_enrollment_v1()?;
        let prepared = store.prepare_roster_source_enrollment_v1(
            inputs,
            RosterEnrollmentReviewV1 {
                operation: RosterAuthorityTextV1::parse("controlled-report-fixture-enrollment")?,
                actor: RosterReviewActorV1::LocalOperator,
                reviewed_at: instant()?,
                rationale: RosterAuthorityTextV1::parse(
                    "Controlled test only; no production human approval or source qualification",
                )?,
            },
        )?;
        let bytes = prepared.approval_bytes().to_vec();
        let digest = prepared.approval_digest().to_string();
        store.apply_roster_source_enrollment_v1(prepared, &bytes, &digest)?;
        let profile = profile("2026-10-16")?;
        store.register_profile(&profile)?;
        Ok(Self {
            directory,
            store,
            writer,
            profile,
        })
    }

    pub fn accept(&self, athlete: &AthleteId) -> TestResult {
        self.writer.establish_source_bound(
            &self.store,
            &format!("controlled-report-accept-{athlete}"),
            &athlete.cast(),
            self.store.snapshot().revision(),
            &self.writer.head(&self.store)?,
        )?;
        Ok(())
    }

    pub fn dataset(&self) -> TestResult<ExportDataset> {
        let snapshot = self.store.snapshot();
        let mut dataset =
            ExportDataset::load_public_identity_snapshot_fixture(&self.store, &snapshot)?;
        dataset.bind_selected_authority_fixture_from_snapshot(&self.profile, &snapshot)?;
        dataset.bind_public_identity_fixture_from_snapshot(&snapshot)?;
        Ok(dataset)
    }

    pub fn assess(&self) -> TestResult<(super::super::super::VerifiedCurrentAffiliations, Value)> {
        let dataset = self.dataset()?;
        let snapshot = self.store.snapshot();
        let objects = self.directory.path().join("report-objects");
        std::fs::create_dir_all(&objects)?;
        let mut grant = AttachmentReplayGrantV1::new();
        let claims = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
        let athletes = crate::export::qualification::physical_observations::capture::<
            CanonicalAthlete,
        >(&dataset, &snapshot, Table::Athletes, &objects, budget())?;
        let bound = BoundRosterSourceViewV1::capture(
            &dataset,
            &self.profile,
            &objects,
            &claims,
            athletes,
            instant()?,
            &mut grant,
        )?;
        let (pinned, inventory) =
            super::super::assess(&dataset, &self.profile, &bound, &mut grant)?;
        Ok((
            pinned.into_verified(),
            serde_json::from_slice(inventory.bytes())?,
        ))
    }
}

pub(super) fn budget() -> census_store::ReadBudget {
    census_store::ReadBudget {
        rows: 200_000,
        bytes: 256 * 1024 * 1024,
        record_bytes: 1024 * 1024,
    }
}

pub(super) fn instant() -> TestResult<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(AT)?.to_utc())
}

pub(super) fn profile(as_of: &str) -> TestResult<QueryProfileV1> {
    use census_domain::profile_execution::{
        selected_qualification_contract_ref_v4 as contract, ContractRoleV1::*,
    };
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![UsJurisdiction::Michigan],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::IndoorTrack, Sport::OutdoorTrack, Sport::CrossCountry],
        competition_categories: vec![Gender::Boys, Gender::Girls],
        as_of: ProfileDate::parse(as_of)?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2026).ok_or("controlled year")?,
        source_plan: contract(SourcePlan)?,
        acquisition_window: ProfileDateRangeV1::new(
            ProfileDate::parse("2026-10-01")?,
            ProfileDate::parse("2026-10-31")?,
        )?,
        completion_contract: contract(Completion)?,
        policies: ProfilePoliciesV1 {
            cohort: contract(Cohort)?,
            event: contract(Event)?,
            identity: contract(Identity)?,
            affiliation: contract(Affiliation)?,
            contact: contract(Contact)?,
            participation: contract(Participation)?,
            publication: contract(Publication)?,
        },
        include_provisional: ProvisionalInclusionV1::Exclude,
    })?)
}

#[path = "fixture_install.rs"]
mod installation;
pub(super) use installation::{install, install_padded, install_row, PhysicalProjection};
