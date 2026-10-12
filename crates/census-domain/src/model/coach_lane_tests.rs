use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const OBSERVED_ON: &str = "2026-10-11";
const DIGEST: &str = "9f2b1c4d5e6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c";
const FIXTURE_ARM: &str = "coach_directory_fixture";
const FIXTURE_BASIS: &str = "fixture lane selected for the 49-row obligation matrix";

fn season() -> Result<SchoolYear, Box<dyn std::error::Error>> {
    Ok(SchoolYear::new(2026).ok_or("school year 2026")?)
}

fn pending_row(
    jurisdiction: UsJurisdiction,
) -> Result<CoachLaneObligation, Box<dyn std::error::Error>> {
    Ok(CoachLaneObligation::pending(PendingCoachLane {
        jurisdiction,
        season: season()?,
        revision: 1,
        arm: FIXTURE_ARM.to_string(),
        arm_basis: FIXTURE_BASIS.to_string(),
        observed_on: OBSERVED_ON.to_string(),
    })?)
}

fn armed_row(
    jurisdiction: UsJurisdiction,
) -> Result<CoachLaneObligation, Box<dyn std::error::Error>> {
    let evidence = CoachLaneEvidence::new(7, 9, DIGEST, OBSERVED_ON)?;
    let resolution = CoachLaneResolution::resolved(
        CoachLaneRole::Published,
        CoachLaneTenure::Current,
        CoachLaneMailbox::Published,
        OBSERVED_ON,
        30,
        "fixture coach columns carry a role and a season",
    )?;
    Ok(pending_row(jurisdiction)?.arm(evidence, resolution)?)
}

fn accepted_refusal_row(
    jurisdiction: UsJurisdiction,
) -> Result<CoachLaneObligation, Box<dyn std::error::Error>> {
    let refusal = CoachLaneRefusal::new(
        SourceRef::id(FIXTURE_ARM),
        "fixture association publishes no coach roles for this season",
        OBSERVED_ON,
    )?;
    let recorded = pending_row(jurisdiction)?.record(
        CoachLaneStage::SuccessfulEmpty,
        Some(refusal),
        CoachLaneResolution::unresolved(OBSERVED_ON),
    )?;
    let acceptance = CoachLaneAcceptance::new("census-ops", "jpx4.accepted_refusal", OBSERVED_ON)?;
    Ok(recorded.accept_refusal(acceptance)?)
}

fn matrix() -> Result<CoachLaneMatrix, Box<dyn std::error::Error>> {
    let rows = UsJurisdiction::CENSUS_SCOPE
        .iter()
        .enumerate()
        .map(|(index, &jurisdiction)| {
            if index % 2 == 0 {
                armed_row(jurisdiction)
            } else {
                accepted_refusal_row(jurisdiction)
            }
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    Ok(CoachLaneMatrix::new(rows))
}

#[test]
fn coach_lane_matrix_covers_all_49_jurisdictions_and_closes_every_row() -> TestResult {
    let matrix = matrix()?;
    check!(eq; matrix.rows().len(), 49);
    check!(eq; matrix.rows().len(), UsJurisdiction::CENSUS_SCOPE.len());
    check!(
        matrix.rows().iter().all(CoachLaneObligation::is_terminal),
        "every row must be armed or accepted-refused, never left open"
    );
    check!(eq;
        matrix
            .rows()
            .iter()
            .filter(|row| row.status() == CoachLaneStatus::Armed)
            .count(),
        25
    );
    check!(eq;
        matrix
            .rows()
            .iter()
            .filter(|row| row.status() == CoachLaneStatus::AcceptedRefusal)
            .count(),
        24
    );
    check!(eq;
        matrix
            .rows()
            .iter()
            .map(CoachLaneObligation::jurisdiction)
            .collect::<Vec<_>>(),
        UsJurisdiction::CENSUS_SCOPE.to_vec()
    );
    check!(eq; matrix.verify(), Ok(()));
    Ok(())
}

#[test]
fn coach_lane_matrix_rejects_a_missing_jurisdiction() -> TestResult {
    let matrix = matrix()?;
    let missing = UsJurisdiction::Wyoming;
    let rows = matrix
        .rows()
        .iter()
        .filter(|row| row.jurisdiction() != missing)
        .cloned()
        .collect::<Vec<_>>();
    check!(eq;
        CoachLaneMatrix::new(rows).verify(),
        Err(CoachLaneError::MissingJurisdiction {
            jurisdiction: missing
        })
    );
    Ok(())
}

#[test]
fn coach_lane_matrix_rejects_an_out_of_scope_row() -> TestResult {
    let matrix = matrix()?;
    let mut rows = matrix.rows().to_vec();
    rows.push(armed_row(UsJurisdiction::Alaska)?);
    check!(eq;
        CoachLaneMatrix::new(rows).verify(),
        Err(CoachLaneError::OutsideCensusScope {
            jurisdiction: UsJurisdiction::Alaska
        })
    );
    Ok(())
}

#[test]
fn coach_lane_matrix_rejects_a_reasonless_refusal() -> TestResult {
    let matrix = matrix()?;
    let tampered = CoachLaneObligation {
        status: CoachLaneStatus::Blocked,
        refusal: Some(CoachLaneRefusal {
            source: SourceRef::id(FIXTURE_ARM),
            reason: String::new(),
            observed_on: OBSERVED_ON.to_string(),
        }),
        resolution: CoachLaneResolution::unresolved(OBSERVED_ON),
        ..pending_row(UsJurisdiction::Nevada)?
    };
    let mut rows = matrix
        .rows()
        .iter()
        .filter(|row| row.jurisdiction() != UsJurisdiction::Nevada)
        .cloned()
        .collect::<Vec<_>>();
    rows.push(tampered);
    check!(eq;
        CoachLaneMatrix::new(rows).verify(),
        Err(CoachLaneError::OutOfRange {
            field: "refusal.reason"
        })
    );
    check!(CoachLaneRefusal::new(SourceRef::id(FIXTURE_ARM), String::new(), OBSERVED_ON).is_err());
    Ok(())
}

#[test]
fn coach_lane_recorded_refusal_cannot_close_the_lane() -> TestResult {
    let pending = pending_row(UsJurisdiction::Vermont)?;
    let refusal = CoachLaneRefusal::new(
        SourceRef::id(FIXTURE_ARM),
        "fixture association publishes no coach directory",
        OBSERVED_ON,
    )?;
    let recorded = pending.clone().record(
        CoachLaneStage::Blocked,
        Some(refusal),
        CoachLaneResolution::unresolved(OBSERVED_ON),
    )?;
    check!(
        !recorded.is_terminal(),
        "a merely recorded refusal leaves the lane open"
    );
    check!(eq; recorded.status(), CoachLaneStatus::Blocked);
    check!(recorded.refusal().is_some());
    check!(recorded.acceptance().is_none());
    let matrix = matrix()?;
    let mut rows = matrix
        .rows()
        .iter()
        .filter(|row| row.jurisdiction() != UsJurisdiction::Vermont)
        .cloned()
        .collect::<Vec<_>>();
    rows.push(recorded.clone());
    check!(eq;
        CoachLaneMatrix::new(rows).verify(),
        Err(CoachLaneError::LaneNotClosed {
            jurisdiction: UsJurisdiction::Vermont,
            status: CoachLaneStatus::Blocked
        })
    );
    let acceptance = CoachLaneAcceptance::new("census-ops", "jpx4.accepted_refusal", OBSERVED_ON)?;
    check!(eq;
        pending.clone().accept_refusal(acceptance.clone()),
        Err(CoachLaneError::MissingRefusal)
    );
    let accepted = recorded.accept_refusal(acceptance)?;
    check!(accepted.is_terminal());
    check!(eq; accepted.status(), CoachLaneStatus::AcceptedRefusal);
    check!(eq;
        accepted.acceptance().ok_or("acceptance")?.actor(),
        "census-ops"
    );
    check!(eq;
        accepted.refusal().ok_or("refusal")?.reason(),
        "fixture association publishes no coach directory"
    );
    check!(eq; accepted.evidence().map(CoachLaneEvidence::digest), None);
    check!(accepted
        .accept_refusal(CoachLaneAcceptance::new(
            "census-ops",
            "jpx4.accepted_refusal",
            OBSERVED_ON
        )?)
        .is_err());
    Ok(())
}

#[test]
fn coach_lane_armed_lane_requires_evidence() -> TestResult {
    check!(CoachLaneEvidence::new(0, 5, DIGEST, OBSERVED_ON).is_err());
    check!(CoachLaneEvidence::new(5, 0, DIGEST, OBSERVED_ON).is_err());
    check!(CoachLaneEvidence::new(5, 5, "not-a-digest", OBSERVED_ON).is_err());
    check!(CoachLaneEvidence::new(5, 5, DIGEST, "2026-02-30").is_err());
    check!(CoachLaneEvidence::new(5, 5, DIGEST, OBSERVED_ON)?
        .with_source_url("ftp://fixture/coaches")
        .is_err());
    let pending = pending_row(UsJurisdiction::Texas)?;
    let unevidenced = CoachLaneObligation {
        status: CoachLaneStatus::Armed,
        evidence: None,
        ..pending.clone()
    };
    check!(eq; unevidenced.validate(), Err(CoachLaneError::MissingEvidence));
    let matrix = matrix()?;
    let mut rows = matrix
        .rows()
        .iter()
        .filter(|row| row.jurisdiction() != UsJurisdiction::Texas)
        .cloned()
        .collect::<Vec<_>>();
    rows.push(unevidenced);
    check!(eq;
        CoachLaneMatrix::new(rows).verify(),
        Err(CoachLaneError::MissingEvidence)
    );
    let evidence = CoachLaneEvidence::new(5, 9, DIGEST, OBSERVED_ON)?
        .with_source_url("https://fixture.example/coaches")?;
    let armed = pending.arm(evidence, CoachLaneResolution::unresolved(OBSERVED_ON))?;
    check!(armed.is_terminal());
    check!(eq; armed.status(), CoachLaneStatus::Armed);
    check!(eq; armed.evidence().map(CoachLaneEvidence::schools), Some(5));
    check!(eq; armed.evidence().map(CoachLaneEvidence::contacts), Some(9));
    check!(eq; armed.refusal(), None);
    check!(eq;
        armed.evidence().and_then(CoachLaneEvidence::source_url),
        Some("https://fixture.example/coaches")
    );
    Ok(())
}

#[test]
fn coach_lane_row_round_trips_through_json_and_rejects_tampering() -> TestResult {
    for row in [
        armed_row(UsJurisdiction::Iowa)?,
        accepted_refusal_row(UsJurisdiction::SouthDakota)?,
    ] {
        let decoded: CoachLaneObligation = serde_json::from_value(serde_json::to_value(&row)?)?;
        check!(eq; decoded, row);
    }
    let armed = armed_row(UsJurisdiction::Iowa)?;
    let mut encoded = serde_json::to_value(&armed)?;
    encoded["status"] = serde_json::Value::String("armed_without_evidence".to_string());
    check!(
        serde_json::from_value::<CoachLaneObligation>(encoded).is_err(),
        "an unmodelled status cannot be read back"
    );
    let mut encoded = serde_json::to_value(&armed)?;
    encoded["evidence"] = serde_json::Value::Null;
    check!(
        serde_json::from_value::<CoachLaneObligation>(encoded).is_err(),
        "an armed row without evidence cannot be read back"
    );
    let mut encoded = serde_json::to_value(&armed)?;
    encoded["updated_on"] = serde_json::Value::String("1899-01-01".to_string());
    check!(
        serde_json::from_value::<CoachLaneObligation>(encoded).is_err(),
        "a payload dated after the row update cannot be read back"
    );
    let mut encoded = serde_json::to_value(&armed)?;
    encoded["surprise"] = serde_json::Value::Null;
    check!(
        serde_json::from_value::<CoachLaneObligation>(encoded).is_err(),
        "an unknown field cannot be read back"
    );
    let refused = accepted_refusal_row(UsJurisdiction::SouthDakota)?;
    let mut encoded = serde_json::to_value(&refused)?;
    encoded["refusal"]["reason"] = serde_json::Value::String(String::new());
    check!(
        serde_json::from_value::<CoachLaneObligation>(encoded).is_err(),
        "a reasonless refusal cannot be read back"
    );
    let mut encoded = serde_json::to_value(&refused)?;
    encoded["acceptance"] = serde_json::Value::Null;
    check!(
        serde_json::from_value::<CoachLaneObligation>(encoded).is_err(),
        "a terminal refusal without an acceptance event cannot be read back"
    );
    Ok(())
}
