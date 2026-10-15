use super::*;
use crate::bests::{ParticipationExclusionOutcome, ParticipationExclusionReason};
use crate::export::VerifiedParticipationView;
use census_domain::model::{ContractRefV1, Id, Mark, ProfileId};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn participation_exclusion_byte_budget_admits_exactly_then_refuses() -> TestResult {
    let performance = performance();
    let bytes = exclusion_bytes(&performance)?;
    let mut exclusions = Exclusions::new();
    exclusions
        .budget
        .charge(crate::grouping::STAGE_BYTES - bytes, 0)?;
    boundary(&mut exclusions, &performance, "encoded bytes")
}

#[test]
fn participation_exclusion_work_budget_admits_exactly_then_refuses_duplicate_growth() -> TestResult
{
    let performance = performance();
    let mut exclusions = Exclusions::new();
    exclusions
        .budget
        .charge(0, crate::grouping::STAGE_WORK - 1)?;
    boundary(&mut exclusions, &performance, "work units")
}

fn boundary(
    exclusions: &mut Exclusions,
    performance: &CanonicalPerformance,
    expected_resource: &str,
) -> TestResult {
    let view = VerifiedParticipationView::fixture(
        "budget-fixture".into(),
        ProfileId::parse(&"a".repeat(64))?,
        ContractRefV1::new(1, 1, &"b".repeat(64))?,
        std::collections::BTreeMap::new(),
    );
    let selection = ParticipationSelection::Selected(&view);
    check!(exclusions.admit(selection, performance)? == CandidateAdmission::Withheld);
    check!(eq; exclusions.rows.len(), 1);
    match exclusions.admit(selection, performance) {
        Err(crate::report::ReportError::Grouping(GroupingError::BudgetExceeded {
            stage,
            resource,
            ..
        })) => {
            check!(eq; stage, STAGE);
            check!(eq; resource, expected_resource);
        }
        Err(error) => return Err(format!("unexpected exclusion budget error: {error}").into()),
        Ok(_) => {
            return Err("participation exclusions exceeded their complete-stage budget".into())
        }
    }
    check!(eq; exclusions.rows.len(), 1);
    let row = exclusions.rows.first().ok_or("admitted exclusion")?;
    check!(eq; row.performance_id(), &performance.id);
    check!(eq; row.outcome(), ParticipationExclusionOutcome::Unresolved);
    check!(eq; row.reason(), ParticipationExclusionReason::MissingParticipationEvidence);
    Ok(())
}

fn performance() -> CanonicalPerformance {
    CanonicalPerformance {
        id: Id::mint("perf", &["budget"]),
        athlete: Id::mint("ath", &["budget"]),
        team: Id::mint("team", &["budget"]),
        event: Id::mint("evt", &["budget"]),
        meet: Id::mint("meet", &["budget"]),
        date: "2026-05-01".into(),
        mark: Mark::Raw("unresolved".into()),
        wind_mps: None,
        adjustment: None,
        place: None,
        heat: None,
        round: None,
        timing: None,
        observed_grade: None,
        evidence: Vec::new(),
        source_key: "budget".into(),
        source_athlete: None,
        retained_conflicts: Vec::new(),
        combined_completion: None,
    }
}
