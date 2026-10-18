#[path = "../../../../../census-report/src/export/dataset/selected/tests/publication/controlled_three_marks/attachment/attachment.rs"]
mod attachment;
#[path = "../../../../../census-report/src/export/dataset/selected/tests/publication/controlled_three_marks/current_school_identity.rs"]
mod current_school_identity;
#[path = "../../../../../census-report/src/export/dataset/selected/tests/publication/controlled_three_marks/current_school_review/current_school_review.rs"]
mod current_school_review;
#[path = "../../../../../census-report/src/export/dataset/selected/tests/publication/controlled_three_marks/current_school_source.rs"]
mod current_school_source;
#[path = "../../../../../census-report/src/export/dataset/selected/tests/publication/controlled_three_marks/documentary_lifecycle.rs"]
mod documentary_lifecycle;
#[path = "../../../../../census-report/src/export/dataset/selected/tests/publication/controlled_three_marks/documentary_material/documentary_material.rs"]
mod documentary_material;
#[path = "../../../../../census-report/src/export/dataset/selected/tests/publication/controlled_three_marks/documentary_review.rs"]
mod documentary_review;
#[path = "../../../../../census-report/src/export/dataset/selected/tests/publication/controlled_three_marks/objects.rs"]
mod objects;
#[path = "../../../../../census-report/src/export/dataset/selected/tests/publication/controlled_three_marks/point_source.rs"]
mod point_source;
#[path = "../../../../../census-report/src/export/dataset/selected/tests/publication/controlled_three_marks/school_authority.rs"]
mod school_authority;
#[path = "../../../../../census-report/src/export/dataset/selected/tests/publication/controlled_three_marks/school_issuance.rs"]
mod school_issuance;
#[path = "../../../../../census-report/src/export/dataset/selected/tests/publication/controlled_three_marks/source/source.rs"]
mod source;
#[path = "../../../../../census-report/src/export/dataset/selected/tests/publication/controlled_three_marks/window_source.rs"]
mod window_source;

use census_domain::evidence_subject_attachment::AttachmentPredecessorV1;
use census_domain::model::{
    DiscoveryPriorityV1, Gender, GradYear, PerformanceWindowV1, ProfileDate, ProfileDateRangeV1,
    ProfilePoliciesV1, ProvisionalInclusionV1, QueryProfileInputV1, QueryProfileV1,
    RegionalMembershipV1, SchoolYear, Sport,
};
use census_domain::profile_execution::{
    selected_qualification_contract_ref_v5 as reference, ContractRoleV1 as Role,
};
use census_domain::UsJurisdiction;
use source::TestResult;

pub(in crate::restate_services) struct Controlled {
    current: census_domain::model::SchoolId,
    profile: QueryProfileV1,
    installed: source::InstalledSource,
    accepted: documentary_review::AcceptedSource,
}

impl Controlled {
    pub(in crate::restate_services) fn store(&self) -> &census_store::Store {
        &self.installed.store
    }

    pub(in crate::restate_services) fn public_id(&self) -> &str {
        self.installed.public.as_str()
    }

    pub(in crate::restate_services) fn candidate_id(&self) -> &str {
        self.installed.candidate.as_str()
    }

    pub(in crate::restate_services) fn current_school_id(&self) -> &str {
        self.current.as_str()
    }

    pub(in crate::restate_services) fn profile(&self) -> &QueryProfileV1 {
        &self.profile
    }

    pub(in crate::restate_services) fn selection(
        &self,
    ) -> TestResult<census_domain::model::ProfileSelectionV1> {
        Ok(census_domain::model::ProfileSelectionV1::new(
            self.store().store_identity()?,
            self.profile.id()?,
        ))
    }
}

pub(in crate::restate_services) fn install() -> TestResult<Controlled> {
    let installed = source::install()?;
    let current = current_school_source::install(&installed)?;
    current_school_identity::accept(&installed, &current)?;
    current_school_review::accept(&installed, &current)?;
    let points = point_source::install(&installed)?;
    let window = window_source::install(&installed)?;
    let point50 = points.get(1).ok_or("controlled May15 source missing")?;
    school_authority::establish(&installed, point50)?;
    let mut accepted = None;
    for (index, point) in points.iter().enumerate() {
        let reviewed = documentary_review::accept(
            &installed,
            point,
            documentary_material::Kind::Point,
            AttachmentPredecessorV1::First,
            &format!("controlled-service-status-{index}"),
        )?;
        if index == 1 {
            accepted = Some(reviewed);
        }
    }
    documentary_review::accept(
        &installed,
        &window,
        documentary_material::Kind::Window,
        AttachmentPredecessorV1::First,
        "controlled-service-window",
    )?;
    let mut input = profile()?.input().clone();
    input.jurisdictions = UsJurisdiction::CENSUS_SCOPE.to_vec();
    let profile = QueryProfileV1::new(input)?;
    installed.store.register_profile(&profile)?;
    Ok(Controlled {
        installed,
        current: current.school,
        profile,
        accepted: accepted.ok_or("controlled May15 review missing")?,
    })
}

pub(in crate::restate_services) fn revoke(controlled: &Controlled) -> TestResult {
    documentary_lifecycle::revoke(&controlled.installed, &controlled.accepted)?;
    documentary_lifecycle::replay_without_reactivation(&controlled.installed, &controlled.accepted)
}

fn profile() -> TestResult<QueryProfileV1> {
    let participation = reference(Role::Participation)?;
    if participation != census_domain::participation_candidates::participation_policy_v4()? {
        return Err("controlled participation policy drift".into());
    }
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![UsJurisdiction::Colorado],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::OutdoorTrack],
        competition_categories: vec![Gender::Boys],
        as_of: ProfileDate::parse("2026-10-16")?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2026).ok_or("controlled school year")?,
        source_plan: reference(Role::SourcePlan)?,
        acquisition_window: ProfileDateRangeV1::new(
            ProfileDate::parse("2026-01-01")?,
            ProfileDate::parse("2026-10-16")?,
        )?,
        completion_contract: reference(Role::Completion)?,
        policies: ProfilePoliciesV1 {
            cohort: reference(Role::Cohort)?,
            event: reference(Role::Event)?,
            identity: reference(Role::Identity)?,
            affiliation: reference(Role::Affiliation)?,
            contact: reference(Role::Contact)?,
            participation,
            publication: reference(Role::Publication)?,
        },
        include_provisional: ProvisionalInclusionV1::Exclude,
    })?)
}
