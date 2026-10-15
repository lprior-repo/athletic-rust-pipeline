use crate::model::*;
use crate::public_identity_history::HistoryError;

use super::fixture::*;
use super::*;

mod matching;

fn accepted_application(observed_at: &str) -> TestResult<AcceptedAthleteIdentity> {
    let school = SchoolId::mint("sch", &["WI", "Fixture School"]);
    let mut athlete = CanonicalAthlete::new(
        &school,
        "Synthetic Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::TfrrsAthlete, "7"),
    )?;
    athlete.evidence.push(Evidence::parsed(
        SourceRef::new("fixture", Some("https://fixture.test/a".into())),
        "2026-10-07",
    ));
    let mut index = AthleteIdentityIndex::default();
    index.observe(&athlete)?;
    let builder = IdentityProjectionBuilder::new(index, &[], &[])?;
    let application = builder
        .source_applications(observed_at)
        .next()
        .ok_or("missing source application")??;
    match application {
        IdentityApplication::Accepted(accepted) => Ok(accepted),
        IdentityApplication::Retained(issue) => Err(format!("fixture retained: {issue:?}").into()),
    }
}

fn accepted_reference(observed_at: &str) -> TestResult<CandidateDecisionReference> {
    Ok(CandidateDecisionReference::from_accepted(
        &accepted_application(observed_at)?,
        ReplayLimits::SMALL,
    )?)
}

#[test]
fn existing_accepted_identity_boundary_supplies_a_reference_not_a_new_authority() -> TestResult {
    let first = accepted_reference("2026-10-14")?;
    let second = accepted_reference("2026-10-15")?;
    check!(eq; first.kind, CandidateEvidenceKind::SourceBound);
    check!(eq; first.members.len(), 1);
    check!(eq; first, second);
    Ok(())
}

#[test]
fn merge_reference_must_cover_the_exact_current_components() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _first = fixture.append(establish("a", "1"))?;
    let _second = fixture.append(establish("b", "2"))?;
    fixture.merge(FIRST, &[FIRST, SECOND], &["a"], "merge:incomplete")?;
    check!(eq; fixture.project().err(), Some(HistoryError::EvidenceMismatch));
    Ok(())
}

#[test]
fn a_source_reference_cannot_establish_a_different_candidate() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _births = fixture.append(HistoryRequest::Establish {
        binding: binding("a", "1"),
        evidence: evidence("wrong-source", CandidateEvidenceKind::SourceBound, &["b"]),
    })?;
    check!(eq; fixture.project().err(), Some(HistoryError::EvidenceMismatch));
    Ok(())
}

#[test]
fn same_person_reference_is_not_correction_authority() -> TestResult {
    let mut fixture = chain()?;
    let mut split = split_request(&[&["a"], &["b", "c"]], vec![4])?;
    split.evidence.kind = CandidateEvidenceKind::SamePerson;
    let _births = fixture.append(HistoryRequest::Split(split))?;
    check!(eq; fixture.project().err(), Some(HistoryError::EvidenceMismatch));
    Ok(())
}
