use crate::model::{
    AthleteIdentityIndex, CanonicalAthlete, Evidence, Gender, GradYear, IdentityApplication,
    IdentityProjectionBuilder, ReviewCase, ReviewState, ReviewVerdictRecord, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef, ATHLETE_IDENTITY_FAMILY,
};

use super::*;

fn athlete(school: &str) -> TestResult<CanonicalAthlete> {
    let mut athlete = CanonicalAthlete::new(
        &SchoolId::mint("sch", &["WI", school]),
        "Synthetic Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::TfrrsAthlete, "9"),
    )?;
    athlete.evidence.push(Evidence::parsed(
        SourceRef::new("fixture", Some("https://fixture.test/runner".into())),
        "2026-10-07",
    ));
    Ok(athlete)
}

#[test]
fn an_existing_accepted_same_person_decision_can_establish_the_initial_group() -> TestResult {
    let first = athlete("School A")?;
    let mut second = athlete("School B")?;
    second.add_identity(SourceIdentity::new(SourceNamespace::MilesplitAthlete, "1"));
    let members = vec![first.id.cast(), second.id.cast()];
    let mut index = AthleteIdentityIndex::default();
    index.observe(&first)?;
    index.observe(&second)?;
    let mut case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        first.id.as_str(),
        "Shared retained provider identity",
        "Two source-owned observations of one provider athlete",
        index.case_evidence(
            "Shared retained provider identity",
            "Two source-owned observations of one provider athlete",
            &members,
        )?,
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
        value: "same_person".into(),
        accepted: true,
        confidence: 100,
        rationale: "Shared retained provider identity".into(),
        reviewer: "deterministic fixture".into(),
        observed_at: "2026-10-07".into(),
        member_ids: case.member_ids.clone(),
    };
    let cases = [case];
    let verdicts = [verdict];
    let builder = IdentityProjectionBuilder::new(index, &cases, &verdicts)?;
    let (_, application) = builder
        .reviewed_applications("2026-10-14")
        .next()
        .ok_or("missing reviewed application")?;
    let accepted = match application? {
        IdentityApplication::Accepted(accepted) => accepted,
        IdentityApplication::Retained(issue) => return Err(format!("retained: {issue:?}").into()),
    };
    let evidence = CandidateDecisionReference::from_accepted(&accepted, ReplayLimits::SMALL)?;
    check!(eq; evidence.kind, CandidateEvidenceKind::SamePerson);
    check!(evidence.matches_applied(accepted.record(), ReplayLimits::SMALL)?);
    let mut reversed_reference = evidence.clone();
    reversed_reference.members.reverse();
    check!(!reversed_reference.matches_applied(accepted.record(), ReplayLimits::SMALL)?);
    let mut reversed_record = accepted.record().clone();
    reversed_record.members.reverse();
    check!(!evidence.matches_applied(&reversed_record, ReplayLimits::SMALL)?);
    let representative = accepted
        .record()
        .canonical_id
        .as_ref()
        .ok_or("missing accepted representative")?
        .cast();
    let mut first_binding = binding("first", "9");
    first_binding.candidate = first.id.cast();
    let mut second_binding = binding("second", "1");
    second_binding.source.namespace = SourceNamespace::MilesplitAthlete;
    second_binding.candidate = second.id.cast();
    let mut fixture = Fixture::new()?;
    check!(eq; fixture.append(HistoryRequest::EstablishMatchedGroup {
        bindings: vec![second_binding, first_binding],
        evidence,
    })?, vec![PublicAthleteId::parse(FIRST)?]);
    let projection = fixture.project()?;
    check!(eq; projection.resolve_candidate(&second.id.cast())?, PublicResolution::Canonical(ResolvedSuccessor {
        public_id: PublicAthleteId::parse(FIRST)?,
        representative_candidate: representative,
        established: EstablishmentOrdinal { sequence: 1, position: 0 },
    }));
    Ok(())
}
