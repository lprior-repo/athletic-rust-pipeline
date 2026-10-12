use super::super::coalesce;
use super::{bounded_claims, bounded_contacts, GroupBudget, WORKBOOK_GROUP_BUDGET};

use crate::report::ReportError;
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Gender, RetainedConflict, SchoolId,
    SchoolYear, Sport,
};
use census_domain::UsJurisdiction;

#[path = "tests/sheet.rs"]
mod sheet;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn school_id(state: UsJurisdiction, name: &str) -> SchoolId {
    CanonicalSchool::mint(state, name, &normalize_name(name), None)
}

fn coach(
    school: &SchoolId,
    name: &str,
    sport: Option<Sport>,
    gender: Gender,
    role: CoachRole,
) -> CanonicalCoach {
    CanonicalCoach::new(school, name, sport, gender, role)
}

fn refusal(error: ReportError) -> TestResult<crate::grouping::GroupingError> {
    match error {
        ReportError::Grouping(error) => Ok(error),
        other => Err(format!("the refusal is not a typed grouping error: {other}").into()),
    }
}

#[test]
fn oversized_coach_payload_is_refused_before_coalescing() -> TestResult {
    let school = school_id(UsJurisdiction::Wisconsin, "Payload High");
    let mut observation = coach(
        &school,
        "Runner",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    observation.name = "x".repeat(1_048_577);
    let error = match bounded_claims(&[observation], WORKBOOK_GROUP_BUDGET) {
        Err(error) => refusal(error)?,
        Ok(_) => return Err("a single oversized record was cloned".into()),
    };
    check!(matches!(
        error,
        crate::grouping::GroupingError::BudgetExceeded {
            stage: "coaches",
            resource: "encoded row bytes",
            available: 1_048_576,
            ..
        }
    ));
    Ok(())
}

#[test]
fn zero_owner_budget_refuses_the_first_observation() -> TestResult {
    let school = school_id(UsJurisdiction::Wisconsin, "Zero High");
    let observation = coach(&school, "Runner", None, Gender::Mixed, CoachRole::HeadCoach);
    let owner = observation.id.to_string();
    let error = match bounded_claims(&[observation], GroupBudget::new(0, 1)) {
        Err(error) => refusal(error)?,
        Ok(_) => return Err("a vacant owner ignored its budget".into()),
    };
    check!(eq; error, crate::grouping::GroupingError::BudgetExceeded {
        stage: "coaches", group: owner, resource: "claims", needed: 1, available: 0,
    });
    Ok(())
}

#[test]
fn bounded_grouping_refuses_an_oversized_owner_group_and_retains_its_evidence() -> TestResult {
    let school = school_id(UsJurisdiction::Wisconsin, "Owner Budget High");
    let template = coach(
        &school,
        "Repeated Runner",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    let observations: Vec<CanonicalCoach> = (0..4).map(|_| template.clone()).collect();
    let error = match bounded_claims(&observations, GroupBudget::new(3, 16)) {
        Err(error) => error,
        Ok(_) => return Err("an owner group of four claims passed a three-claim budget".into()),
    };
    check!(eq; refusal(error)?, crate::grouping::GroupingError::BudgetExceeded {
        stage: "coaches", group: template.id.to_string(), resource: "claims",
        needed: 4, available: 3,
    });
    Ok(())
}

#[test]
fn bounded_grouping_refuses_an_oversized_school_group_and_retains_its_evidence() -> TestResult {
    let school = school_id(UsJurisdiction::Minnesota, "School Budget High");
    let observations: Vec<CanonicalCoach> =
        ["First Owner", "Second Owner", "Third Owner", "Fourth Owner"]
            .into_iter()
            .map(|name| {
                coach(
                    &school,
                    name,
                    Some(Sport::CrossCountry),
                    Gender::Mixed,
                    CoachRole::HeadCoach,
                )
            })
            .collect();
    let error = match bounded_claims(&observations, GroupBudget::new(8, 3)) {
        Err(error) => error,
        Ok(_) => return Err("a school group of four claims passed a three-claim budget".into()),
    };
    check!(eq; refusal(error)?, crate::grouping::GroupingError::BudgetExceeded {
        stage: "coaches", group: school.to_string(), resource: "coach observations",
        needed: 4, available: 3,
    });
    Ok(())
}

#[test]
fn bounded_grouping_refuses_an_oversized_school_before_building_school_contacts() -> TestResult {
    let school = school_id(UsJurisdiction::Minnesota, "Contact Budget High");
    let observations: Vec<CanonicalCoach> = ["Alpha Runner", "Beta Runner", "Gamma Runner"]
        .into_iter()
        .map(|name| {
            coach(
                &school,
                name,
                Some(Sport::CrossCountry),
                Gender::Mixed,
                CoachRole::HeadCoach,
            )
        })
        .collect();
    let school_year = SchoolYear::new(2026).ok_or("invalid fixture season")?;
    let error = match bounded_contacts(&observations, school_year, GroupBudget::new(8, 2)) {
        Err(error) => error,
        Ok(_) => return Err("an oversized school group reached the contact builder".into()),
    };
    check!(eq; refusal(error)?, crate::grouping::GroupingError::BudgetExceeded {
        stage: "coaches", group: school.to_string(), resource: "coach observations",
        needed: 3, available: 2,
    });
    Ok(())
}

#[test]
fn bounded_grouping_accepts_a_group_exactly_at_its_owner_budget() -> TestResult {
    let school = school_id(UsJurisdiction::Wisconsin, "Exact Budget High");
    let base = coach(
        &school,
        "Exact Runner",
        Some(Sport::IndoorTrack),
        Gender::Girls,
        CoachRole::HeadCoach,
    );
    let mut first = base.clone();
    first.professional_email = Some("first@example.test".to_string());
    let mut second = base.clone();
    second.professional_email = Some("second@example.test".to_string());
    let mut third = base.clone();
    third.professional_email = Some("third@example.test".to_string());
    let observations = vec![first, second, third];
    let claims = bounded_claims(&observations, GroupBudget::new(3, 3))?;
    check!(eq; claims.len(), 1, "one owner id folds into one claim");
    check!(eq; claims[0].id, base.id);
    check!(
        eq;
        claims[0].professional_email,
        Some("first@example.test".to_string()),
        "the first occurrence keeps its published mailbox"
    );
    Ok(())
}

#[test]
fn parity_grouping_matches_the_oracle_claims_on_duplicate_claims_conflicts_and_repeated_ids(
) -> TestResult {
    let wi = school_id(UsJurisdiction::Wisconsin, "Parity West High");
    let mn = school_id(UsJurisdiction::Minnesota, "Parity East High");
    let mut first = coach(
        &wi,
        "Repeated Runner",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    first.professional_email = Some("first@example.test".to_string());
    first.retained_conflicts.push(RetainedConflict::new(
        "Coach identity",
        first.id.as_str(),
        "Repeated Runner",
        "observation rows disagree",
    ));
    let mut second = coach(
        &wi,
        "Repeated Runner",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    second.professional_email = Some("second@example.test".to_string());
    let mut third = coach(
        &wi,
        "Repeated Runner",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    third.phone = Some("555-0100".to_string());
    third.retained_conflicts.push(RetainedConflict::new(
        "Coach identity",
        third.id.as_str(),
        "Repeated Runner",
        "observation rows disagree",
    ));
    let mut other_school = coach(
        &mn,
        "Repeated Runner",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    other_school.professional_email = Some("other@example.test".to_string());
    let mut conflicting = coach(
        &wi,
        "Conflict Runner",
        Some(Sport::CrossCountry),
        Gender::Boys,
        CoachRole::HeadCoach,
    );
    conflicting.retained_conflicts.push(RetainedConflict::new(
        "Canonical id collision",
        conflicting.id.as_str(),
        "Conflict Runner",
        "the kept claim differs",
    ));
    let observations = vec![first, second, third, other_school, conflicting];
    let oracle = coalesce::claims(&observations);
    let bounded = bounded_claims(&observations, WORKBOOK_GROUP_BUDGET)?;
    check!(eq; bounded.len(), oracle.len(), "both paths fold the same owner groups");
    check!(eq; bounded, oracle, "the bounded claims equal the oracle claims");
    check!(
        eq;
        serde_json::to_string(&bounded)?,
        serde_json::to_string(&oracle)?,
        "the bounded grouping serializes byte-for-byte as the oracle"
    );
    let merged = bounded
        .iter()
        .find(|claim| claim.name == "Repeated Runner" && claim.school == wi)
        .ok_or("the merged Wisconsin claim is missing")?;
    check!(
        eq;
        merged.professional_email,
        Some("first@example.test".to_string()),
        "the first occurrence keeps its published mailbox"
    );
    check!(eq; merged.retained_conflicts.len(), 1, "the conflict row survives the fold");
    let mut claim_ids: Vec<&str> = bounded.iter().map(|claim| claim.id.as_str()).collect();
    claim_ids.sort_unstable();
    let mut observation_ids: Vec<&str> = observations
        .iter()
        .map(|observation| observation.id.as_str())
        .collect();
    observation_ids.sort_unstable();
    observation_ids.dedup();
    check!(
        eq;
        claim_ids,
        observation_ids,
        "claims come from the observation owner ids, not from merged census coaches"
    );
    let repeated = bounded
        .iter()
        .filter(|claim| claim.name == "Repeated Runner")
        .count();
    check!(eq; repeated, 2, "one claim per owner id, across schools");
    Ok(())
}
