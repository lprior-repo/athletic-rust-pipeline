use super::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn digest(seed: u8) -> String {
    let mut out = String::with_capacity(64);
    for i in 0..64 {
        let v = (i as u8).wrapping_add(seed) % 16;
        let c = if v < 10 { b'0' + v } else { b'a' + (v - 10) };
        out.push(c as char);
    }
    out
}

fn grad(year: i16) -> TestResult<GradYear> {
    GradYear::new(year)
        .ok_or("a grad year builds")
        .map_err(Into::into)
}

fn school_year(start: i16) -> TestResult<SchoolYear> {
    SchoolYear::new(start)
        .ok_or("a school year builds")
        .map_err(Into::into)
}

#[test]
fn cohort_assessment_enforces_the_2027_rule_and_supersession() -> TestResult {
    let member = CohortAssessment::new(
        "ath:milesplit:88213".to_string(),
        GradYear::CO2027,
        CohortVerdict::Member2027,
        digest(1),
        "2026-09-19".to_string(),
    )?;
    check!(member.is_terminal_member());
    check!(member.supersedes().is_none());
    check!(CohortAssessment::new(
        "ath:milesplit:88213".to_string(),
        grad(2026)?,
        CohortVerdict::Member2027,
        digest(2),
        "2026-09-19".to_string(),
    )
    .is_err());
    check!(CohortAssessment::new(
        "ath:milesplit:88213".to_string(),
        GradYear::CO2027,
        CohortVerdict::NotMember,
        digest(2),
        "2026-09-19".to_string(),
    )
    .is_err());
    let review = CohortAssessment::new(
        "ath:milesplit:88213".to_string(),
        GradYear::CO2027,
        CohortVerdict::NeedsReview,
        digest(3),
        "2026-09-19".to_string(),
    )?;
    check!(!review.is_terminal_member());
    let next = member.supersede(
        CohortVerdict::NeedsReview,
        GradYear::CO2027,
        digest(4),
        "2026-09-20".to_string(),
    )?;
    check!(eq; next.supersedes(), Some(member.id()));
    let encoded = serde_json::to_string(&member)?;
    let decoded: CohortAssessment = serde_json::from_str(&encoded)?;
    check!(eq; decoded, member);
    Ok(())
}

#[test]
fn cohort_verdicts_cover_every_grad_year_in_range() -> TestResult {
    for year in 2020..=2040 {
        let class = grad(year)?;
        let member = CohortAssessment::new(
            "s".to_string(),
            class,
            CohortVerdict::Member2027,
            digest(1),
            "2026-09-19".to_string(),
        );
        check!(eq; member.is_ok(), year == 2027, "member2027 binds year {year}");
        let denied = CohortAssessment::new(
            "s".to_string(),
            class,
            CohortVerdict::NotMember,
            digest(1),
            "2026-09-19".to_string(),
        );
        check!(eq; denied.is_ok(), year != 2027, "not_member binds year {year}");
        check!(CohortAssessment::new(
            "s".to_string(),
            class,
            CohortVerdict::NeedsReview,
            digest(1),
            "2026-09-19".to_string(),
        )
        .is_ok());
    }
    Ok(())
}

#[test]
fn affiliations_and_appointments_are_time_scoped_and_checked() -> TestResult {
    let affiliation = AthleteAffiliation::new(
        "ath_subject_01".to_string(),
        "sch_subject_02".to_string(),
        school_year(2025)?,
        digest(5),
        "2026-09-19".to_string(),
    )?;
    check!(eq; affiliation.school_year(), school_year(2025)?);
    let encoded = serde_json::to_string(&affiliation)?;
    let decoded: AthleteAffiliation = serde_json::from_str(&encoded)?;
    check!(eq; decoded, affiliation);
    check!(CoachAppointment::new(AppointmentRequest {
        coach_id: "coa_subject_01".to_string(),
        school_id: "sch_subject_02".to_string(),
        role: CoachRole::Unknown,
        school_year: school_year(2025)?,
        evidence_digest: digest(6),
        observed_on: "2026-09-19".to_string(),
    })
    .is_err());
    let appointment = CoachAppointment::new(AppointmentRequest {
        coach_id: "coa_subject_01".to_string(),
        school_id: "sch_subject_02".to_string(),
        role: CoachRole::HeadCoach,
        school_year: school_year(2025)?,
        evidence_digest: digest(6),
        observed_on: "2026-09-19".to_string(),
    })?;
    check!(eq; appointment.role(), CoachRole::HeadCoach);
    let encoded = serde_json::to_string(&appointment)?;
    let decoded: CoachAppointment = serde_json::from_str(&encoded)?;
    check!(eq; decoded, appointment);
    Ok(())
}

#[test]
fn identity_decisions_keep_their_versioned_members() {
    let member = IdentityMember {
        subject: AthleteCandidateId::mint("athc", &["milesplit", "88213"]),
        evidence_digest: digest(7),
    };
    let decision = AppliedAthleteIdentity {
        id: "athlete:milesplit:88213:1".to_string(),
        policy: ATHLETE_IDENTITY_POLICY,
        kind: AppliedIdentityKind::SourceBound,
        members: vec![member],
        canonical_id: None,
        case_id: None,
        verdict_digest: None,
        observed_at: "2026-09-19".to_string(),
    };
    assert!(refuse_identity_loss(&decision).is_ok());
    let mut wrong_policy = decision.clone();
    wrong_policy.policy = ATHLETE_IDENTITY_POLICY.wrapping_add(1);
    assert!(refuse_identity_loss(&wrong_policy).is_err());
    let mut duplicated = decision.clone();
    duplicated.members.push(duplicated.members[0].clone());
    assert!(refuse_identity_loss(&duplicated).is_err());
    let mut emptied = decision.clone();
    emptied.members.clear();
    assert!(refuse_identity_loss(&emptied).is_err());
}
