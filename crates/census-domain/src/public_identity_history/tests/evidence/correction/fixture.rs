use super::*;

pub(super) fn accepted(value: &str) -> TestResult<AcceptedAthleteIdentity> {
    let mut index = AthleteIdentityIndex::default();
    let mut members = Vec::new();
    let second_source = if value == "same_person" { "71" } else { "72" };
    for (school, source) in [("Correction A", "71"), ("Correction B", second_source)] {
        let mut athlete = CanonicalAthlete::new(
            &SchoolId::mint("sch", &["WI", school]),
            "Synthetic Runner",
            GradYear::CO2027,
            Gender::Girls,
            SourceIdentity::new(SourceNamespace::TfrrsAthlete, source),
        )?;
        athlete.evidence.push(Evidence::parsed(
            SourceRef::new(
                "correction fixture",
                Some(format!("https://fixture.test/{source}")),
            ),
            "2026-10-07",
        ));
        members.push(athlete.id.cast());
        index.observe(&athlete)?;
    }
    let subject = "Distinct retained provider identities";
    let detail = "Different provider objects remain separate";
    let mut case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        members.first().ok_or("member")?.as_str(),
        subject,
        detail,
        index.case_evidence(subject, detail, &members)?,
    );
    case.member_ids = members;
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
        rationale: detail.into(),
        reviewer: "deterministic fixture".into(),
        observed_at: "2026-10-14".into(),
        member_ids: case.member_ids.clone(),
    };
    let cases = [case];
    let verdicts = [verdict];
    let builder = IdentityProjectionBuilder::new(index, &cases, &verdicts)?;
    let (_, result) = builder
        .reviewed_applications("2026-10-14")
        .next()
        .ok_or("missing review")?;
    match result? {
        IdentityApplication::Accepted(accepted) => Ok(accepted),
        IdentityApplication::Retained(issue) => {
            Err(format!("correction retained: {issue:?}").into())
        }
    }
}
