use super::*;
use census_domain::model::{ReviewCase, ReviewState, ReviewVerdictRecord, ATHLETE_IDENTITY_FAMILY};

pub(super) enum Decision {
    Same,
    Different,
}

pub(super) fn install(
    store: &Store,
    members: &[AthleteCandidateId],
    decision: Decision,
) -> TestResult<String> {
    let value = match decision {
        Decision::Same => "same_person",
        Decision::Different => "different_person",
    };
    let first = members
        .first()
        .ok_or("controlled reviewed identity members")?;
    let subject = "Controlled synthetic reviewed-roster identity correction";
    let detail = format!("Controlled test {value}; no genuine source assertion or authenticated production human approval");
    let index = store.snapshot().athlete_identity_index()?;
    let mut case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        first.as_str(),
        subject,
        &detail,
        index.case_evidence(subject, &detail, members)?,
    );
    case.member_ids = members.to_vec();
    case.state = ReviewState::Resolved;
    let verdict = ReviewVerdictRecord {
        id: case.id.clone(),
        case_id: case.id.clone(),
        subject_id: case.subject_id.clone(),
        family: case.family.clone(),
        kind: "value_proposed".into(),
        field: "identity".into(),
        value: value.into(),
        accepted: true,
        confidence: 100,
        rationale: detail,
        reviewer: "controlled regression fixture; no production human approval".into(),
        observed_at: "2026-10-16".into(),
        member_ids: case.member_ids.clone(),
    };
    let mut cases: Vec<ReviewCase> = store.snapshot().scan(Table::ReviewCases)?;
    let mut verdicts: Vec<ReviewVerdictRecord> = store.snapshot().scan(Table::IdentityVerdicts)?;
    check!(cases.len() <= 3);
    check!(verdicts.len() <= 3);
    check!(cases.iter().all(|old| old.id != case.id));
    for old in &mut cases {
        old.state = ReviewState::Superseded;
    }
    let id = case.id.clone();
    cases.push(case);
    verdicts.push(verdict);
    store.replace_many(Table::ReviewCases, &cases)?;
    store.replace_many(Table::IdentityVerdicts, &verdicts)?;
    Ok(id)
}
