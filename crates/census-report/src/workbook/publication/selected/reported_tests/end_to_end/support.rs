use super::TestResult;
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam,
    CompetitionLevel, DiscoveryPriorityV1, EventIdentity, EventKind, EventSpecification, Evidence,
    ExactSeconds, Gender, GradYear, Mark, PerformanceIdentity, PerformanceResult,
    PerformanceWindowV1, ProfileDate, ProfileDateRangeV1, ProfilePoliciesV1,
    ProvisionalInclusionV1, PublishedGraduation, QueryProfileInputV1, QueryProfileV1,
    RegionalMembershipV1, SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
    TimingMethod,
};
use census_domain::profile_execution::{
    selected_qualification_contract_ref_v3 as contract_ref, ContractRoleV1 as ContractRole,
};
use census_domain::UsJurisdiction;

pub(super) fn athlete(
    school: &census_domain::model::SchoolId,
    name: &str,
    provider_id: &str,
) -> TestResult<CanonicalAthlete> {
    let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, provider_id)
        .with_url(format!("https://mi.milesplit.com/athletes/{provider_id}"));
    let mut athlete = CanonicalAthlete::new(school, name, GradYear::CO2027, Gender::Girls, source)?;
    athlete.sports.push(Sport::OutdoorTrack);
    let source_ref = SourceRef::new(
        "milesplit",
        Some(format!("https://mi.milesplit.com/athletes/{provider_id}")),
    );
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: source_ref.clone(),
    });
    athlete
        .evidence
        .push(Evidence::parsed(source_ref, "2026-10-14"));
    Ok(athlete)
}

pub(super) fn event_context(
    school: &census_domain::model::SchoolId,
    gender: Gender,
    date: &str,
    name: &str,
) -> TestResult<(CanonicalTeam, CanonicalMeet, CanonicalEvent)> {
    let team = CanonicalTeam {
        id: CanonicalTeam::mint(
            school,
            Sport::OutdoorTrack,
            gender,
            SchoolYear::new(2026).ok_or("season")?,
        ),
        school: school.clone(),
        sport: Sport::OutdoorTrack,
        gender,
        school_year: SchoolYear::new(2026).ok_or("season")?,
        level: None,
        source_identities: vec![SourceIdentity::new(
            SourceNamespace::MilesplitSchool,
            "95010",
        )],
        evidence: Vec::new(),
        retained_conflicts: Vec::new(),
    };
    let mut meet = CanonicalMeet::new_checked(
        Some(UsJurisdiction::Michigan),
        name,
        date,
        CompetitionLevel::Invitational,
    )?;
    meet.sports.push(Sport::OutdoorTrack);
    let event = crate::test_events::owned(CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::Track400m,
            gender,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?);
    Ok((team, meet, event))
}

pub(super) fn performance(
    athlete: &CanonicalAthlete,
    team: &CanonicalTeam,
    meet: &CanonicalMeet,
    event: &CanonicalEvent,
    key: &str,
    mark: &str,
) -> TestResult<CanonicalPerformance> {
    let mut performance = CanonicalPerformance::new(
        PerformanceIdentity {
            athlete: &athlete.id,
            event: &event.id,
            meet: &meet.id,
            date: &meet.date,
            source_key: key,
        },
        PerformanceResult {
            team: &team.id,
            mark: Mark::TimeSeconds(ExactSeconds::parse(mark)?),
            wind_mps: None,
            adjustment: None,
            place: Some(1),
        },
    )?;
    performance.timing = Some(TimingMethod::Fat);
    performance.source_athlete = athlete.source.clone();
    performance.evidence.push(Evidence::parsed(
        SourceRef::new(
            "milesplit",
            Some(format!("https://mi.milesplit.com/results/{key}")),
        ),
        &meet.date,
    ));
    Ok(performance)
}

pub(super) fn include_profile() -> TestResult<QueryProfileV1> {
    let reference = |role| contract_ref(role);
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
        contact_school_year: SchoolYear::new(2026).ok_or("school year")?,
        source_plan: reference(ContractRole::SourcePlan)?,
        acquisition_window: ProfileDateRangeV1::new(
            ProfileDate::parse("2026-01-01")?,
            ProfileDate::parse("2026-10-01")?,
        )?,
        completion_contract: reference(ContractRole::Completion)?,
        policies: ProfilePoliciesV1 {
            cohort: reference(ContractRole::Cohort)?,
            event: reference(ContractRole::Event)?,
            identity: reference(ContractRole::Identity)?,
            affiliation: reference(ContractRole::Affiliation)?,
            contact: reference(ContractRole::Contact)?,
            participation: reference(ContractRole::Participation)?,
            publication: reference(ContractRole::Publication)?,
        },
        include_provisional: ProvisionalInclusionV1::Include,
    })?)
}
