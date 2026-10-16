use super::{publication, publish, ExportDataset, Fixture, GradYear, Scope, TestResult};
use crate::report::EventAdmission;
use census_domain::model::{
    EventDisposition, EventExclusionReason, EventKind, ProfileSelectionV1, QueryProfileV1,
};
use census_domain::profile_execution::{selected_qualification_contract_ref_v3, ContractRoleV1};
use serde_json::Value;

mod guards;
mod reducer;

#[tokio::test]
async fn zero_eligible_selected_event_scope_publishes_audit_and_replays_offline() -> TestResult {
    let mut fixture = Fixture::selected_event_scope().await?;
    let mut input = super::super::fixture::profile()?.input().clone();
    input.policies.cohort = selected_qualification_contract_ref_v3(ContractRoleV1::Cohort)?;
    input.policies.event = selected_qualification_contract_ref_v3(ContractRoleV1::Event)?;
    input.policies.identity = selected_qualification_contract_ref_v3(ContractRoleV1::Identity)?;
    input.policies.affiliation =
        selected_qualification_contract_ref_v3(ContractRoleV1::Affiliation)?;
    input.policies.contact = selected_qualification_contract_ref_v3(ContractRoleV1::Contact)?;
    input.policies.participation =
        selected_qualification_contract_ref_v3(ContractRoleV1::Participation)?;
    input.policies.publication =
        selected_qualification_contract_ref_v3(ContractRoleV1::Publication)?;
    input.source_plan = selected_qualification_contract_ref_v3(ContractRoleV1::SourcePlan)?;
    input.completion_contract = selected_qualification_contract_ref_v3(ContractRoleV1::Completion)?;
    let profile = QueryProfileV1::new(input)?;
    fixture.store.register_profile(&profile)?;
    fixture.selection = ProfileSelectionV1::new(fixture.store.store_identity()?, profile.id()?);
    let objects = super::super::temporary()?;
    let publication_root = super::super::temporary()?;
    let dataset = fixture.capture(objects.path())?;
    let audit = assert_admission(&dataset)?;
    reducer::synthetic_eligible_candidates(&dataset)?;
    guards::reject_option_overrides(&fixture, &dataset, publication_root.path())?;
    let lineage = dataset.lineage.clone();
    let path = publish(
        &fixture,
        &dataset,
        &publication_root.path().join("publication"),
    )?;
    let verified = publication::verify_published(&path)?;
    let generation = path.parent().ok_or("publication generation")?;
    let published: Value =
        serde_json::from_slice(&std::fs::read(generation.join("event-catalog.json"))?)?;
    check!(eq; published, audit);
    guards::manifest_revision(generation)?;
    fixture.remove_sources()?;
    objects.close()?;
    drop(dataset);
    let reopened =
        publication::open_publication(&path, &lineage, Scope::AllSources, GradYear::CO2027)?;
    check!(eq; assert_admission(&reopened.dataset)?, audit);
    reducer::synthetic_eligible_candidates(&reopened.dataset)?;
    check!(eq; publication::verify_published(&path)?.generation_digest, verified.generation_digest);
    guards::reject_rehashed_contact_year_override(&path)?;
    Ok(())
}

fn assert_admission(dataset: &ExportDataset) -> TestResult<Value> {
    let participation = dataset.verified_participation()?;
    for performance in &dataset.performances {
        let assessment = participation
            .assessment(&performance.id)
            .ok_or("participation assessment")?;
        check!(ne; assessment.outcome(), crate::export::ParticipationOutcome::Eligible);
    }
    let admission = EventAdmission::of(dataset)?;
    check!(eq; dataset.performances.len(), 4);
    check!(eq; admission.counts().included, 1);
    check!(admission.counts().unresolved.is_empty());
    for (key, expected) in [
        (
            "milesplit_result:201782263",
            EventDisposition::Included(EventKind::Track100m),
        ),
        (
            "milesplit_result:201782264",
            EventDisposition::Excluded(EventExclusionReason::OutsideSelectedSport),
        ),
        (
            "milesplit_result:201782265",
            EventDisposition::Excluded(EventExclusionReason::OutsideSelectedCompetitionCategory),
        ),
        (
            "milesplit_result:201782266",
            EventDisposition::Excluded(
                EventExclusionReason::OutsideSelectedSportAndCompetitionCategory,
            ),
        ),
    ] {
        let performance = dataset
            .performances
            .iter()
            .find(|row| row.source_key == key)
            .ok_or("retained selected-scope performance")?;
        check!(eq; admission.disposition(&performance.event), Some(&expected));
    }
    for reason in [
        EventExclusionReason::OutsideSelectedSport,
        EventExclusionReason::OutsideSelectedCompetitionCategory,
        EventExclusionReason::OutsideSelectedSportAndCompetitionCategory,
    ] {
        check!(eq; admission.counts().excluded.get(&reason), Some(&1));
    }
    Ok(serde_json::to_value(admission.audit())?)
}
