use super::*;

use census_domain::model::{
    PerformanceIdentity, PerformanceResult, ProfilePoliciesV1, ProfileSelectionV1, QueryProfileV1,
};
use census_domain::profile_execution::{
    selected_qualification_contract_ref_v3 as reference, ContractRoleV1 as Role,
};

fn current_profile(fixture: &Fixture) -> TestResult<QueryProfileV1> {
    let mut input = fixture.profile.input().clone();
    input.source_plan = reference(Role::SourcePlan)?;
    input.completion_contract = reference(Role::Completion)?;
    input.policies = ProfilePoliciesV1 {
        cohort: reference(Role::Cohort)?,
        event: reference(Role::Event)?,
        identity: reference(Role::Identity)?,
        affiliation: reference(Role::Affiliation)?,
        contact: reference(Role::Contact)?,
        participation: census_domain::participation_candidates::participation_policy_v4()?,
        publication: reference(Role::Publication)?,
    };
    Ok(QueryProfileV1::new(input)?)
}

#[tokio::test]
async fn unresolved_v4_result_preserves_missing_facts_and_refuses_foreign_binding() -> TestResult {
    let fixture = Fixture::prepare(None).await?;
    let profile_id = fixture.profile.id()?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let lineage = &dataset.lineage;
    let query = CandidateQuery {
        profile: &fixture.profile,
        binding: CandidateBinding {
            profile: &profile_id,
            participation_rule: &fixture.profile.input().policies.participation,
            frozen: DeclaredFrozenView {
                lineage: object(&lineage.store_identity)?,
                revision: NonZeroU64::new(snapshot.sequence()).ok_or("revision")?,
                captured_at: DateTime::parse_from_rfc3339("2026-10-14T00:00:00Z")?
                    .with_timezone(&Utc),
                closure_object: object(&lineage.input_digest)?,
                family_inventory: object(&lineage.source_digest)?,
            },
        },
    };
    let (outcome, reasons) = unresolved::assess(query, &fixture.performance)?;
    assert_eq!(outcome, CandidateOutcome::Unresolved);
    for reason in [
        ParticipationReason::ResultOwnershipUnresolved,
        ParticipationReason::CohortUnresolved,
        ParticipationReason::MissingPerformanceDate,
    ] {
        assert!(reasons.contains(reason), "{reason:?}");
    }
    let malformed_rule = census_domain::model::ContractRefV1::new(1, 4, &"f".repeat(64))?;
    let foreign_profile = census_domain::model::ProfileId::parse(&"e".repeat(64))?;
    for binding in [
        CandidateBinding {
            participation_rule: &malformed_rule,
            ..query.binding
        },
        CandidateBinding {
            profile: &foreign_profile,
            ..query.binding
        },
    ] {
        let invalid = CandidateQuery { binding, ..query };
        assert!(matches!(
            unresolved::assess(invalid, &fixture.performance),
            Err(census_store::StoreError::Refused { .. }),
        ));
    }
    Ok(())
}

#[tokio::test]
async fn selected_v4_capture_retains_an_unowned_result_as_unresolved() -> TestResult {
    let fixture = Fixture::prepare(None).await?;
    let source = &fixture.performance;
    let unowned = CanonicalPerformance::new(
        PerformanceIdentity {
            athlete: &source.athlete,
            event: &source.event,
            meet: &source.meet,
            date: &source.date,
            source_key: "tfrrs_result:0",
        },
        PerformanceResult {
            team: &source.team,
            mark: source.mark.clone(),
            wind_mps: source.wind_mps,
            place: source.place,
            adjustment: source.adjustment.clone(),
        },
    )?;
    fixture.store.append(Table::Performances, &unowned)?;
    let selected = current_profile(&fixture)?;
    fixture.store.register_profile(&selected)?;
    let snapshot = fixture.store.snapshot();
    let selection = ProfileSelectionV1::new(fixture.store.store_identity()?, selected.id()?);
    let objects = fixture.objects();
    std::fs::create_dir_all(&objects)?;
    let dataset =
        ExportDataset::from_selected_snapshot(&fixture.store, &snapshot, &selection, &objects)?;
    let assessment = dataset
        .verified_participation()?
        .assessment(&unowned.id)
        .ok_or("unowned result must retain a participation assessment")?;
    assert_eq!(
        assessment.outcome(),
        crate::export::qualification::ParticipationOutcome::Unresolved,
    );
    assert!(assessment
        .reasons()
        .contains(ParticipationReason::ResultOwnershipUnresolved));
    Ok(())
}
