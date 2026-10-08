use super::*;
use crate::export::provenance::{current_coach_contacts, ContactSelectionError};
use census_domain::model::Evidence;

pub(super) const CAPTURE_A: &str = "https://example.invalid/mailbox-a";
pub(super) const CAPTURE_B: &str = "https://example.invalid/name-only-b";
pub(super) const ACQUIRED_A: &str = "2026-08-01T12:34:56Z";
pub(super) const ACQUIRED_B: &str = "2026-09-20T12:34:56Z";
pub(super) const MAILBOX_A: &str = "coach@example.invalid";

pub(super) fn captured_coach(owner: &SchoolId) -> TestResult<CanonicalCoach> {
    let mut coach = CanonicalCoach::new(
        owner,
        "Bound coach",
        Some(Sport::OutdoorTrack),
        Gender::Boys,
        CoachRole::HeadCoach,
    );
    coach.professional_email = Some(MAILBOX_A.into());
    let mut fact = claim(CoachTenure::Current {
        school_year: year()?,
    });
    fact.source.url = Some(CAPTURE_A.into());
    fact.retrieved_at = ACQUIRED_A.into();
    fact.claim = Some(CoachContactClaim {
        coach: coach.id.clone(),
        school: coach.school.clone(),
        role: coach.role,
        program: CoachContactProgram::Team {
            sport: Sport::OutdoorTrack,
            gender: Gender::Boys,
        },
        mailbox: Some(MAILBOX_A.into()),
    });
    let mut newer = fact.clone();
    newer.source.url = Some(CAPTURE_B.into());
    newer.source_sha256 = "b".repeat(64);
    newer.retrieved_at = ACQUIRED_B.into();
    newer.claim.as_mut().ok_or("name binding")?.mailbox = None;
    coach.tenure_evidence = vec![fact, newer];
    coach.evidence.push(Evidence::parsed(
        SourceRef::new("fixture", Some(CAPTURE_B.into())),
        ACQUIRED_B,
    ));
    Ok(coach)
}

#[test]
fn cen15_mailbox_keeps_its_capture_when_newer_generic_name_evidence_exists() -> TestResult {
    let mut coach = captured_coach(&school())?;
    for _ in 0..2 {
        let admitted = current_coach_contacts(&coach, year()?)?.ok_or("current contact")?;
        let mailbox = admitted.professional().ok_or("professional contact")?;
        check!(eq; mailbox.mailbox(), MAILBOX_A);
        check!(eq; mailbox.claim().mailbox.as_deref(), Some(MAILBOX_A));
        check!(eq; mailbox.tenure().source.url.as_deref(), Some(CAPTURE_A));
        check!(eq; mailbox.tenure().source_sha256, "a".repeat(64));
        check!(eq; mailbox.tenure().retrieved_at, ACQUIRED_A);
        check!(eq; admitted.name_only().ok_or("name evidence")?.source.url.as_deref(), Some(CAPTURE_B));
        let result = selected(&[coach.clone()], &athlete())?;
        check!(eq; result.email, MAILBOX_A);
        check!(eq; result.coach_id, coach.id.as_str());
        check!(eq; result.source_url, CAPTURE_A);
        check!(eq; result.source_sha256, "a".repeat(64));
        check!(eq; result.observed_on, ACQUIRED_A);
        coach.tenure_evidence.reverse();
    }
    Ok(())
}

#[test]
fn cen15_professional_personal_and_name_claims_keep_distinct_captures() -> TestResult {
    let mut coach = captured_coach(&school())?;
    coach.personal_email = Some("coach@outlook.com".into());
    let mut personal = coach.tenure_evidence.first().ok_or("mailbox fact")?.clone();
    personal.source.url = Some("https://example.invalid/personal-c".into());
    personal.source_sha256 = "c".repeat(64);
    personal.retrieved_at = "2026-08-15T00:00:00Z".into();
    personal.claim.as_mut().ok_or("personal binding")?.mailbox = coach.personal_email.clone();
    coach.tenure_evidence.push(personal);
    for _ in 0..2 {
        let facts = contacts(&[coach.clone()], year()?);
        let athlete = athlete();
        let scope = scoped(facts.get(school().as_str()), &athlete);
        let rows: Vec<_> = scope
            .mailboxes()
            .map(|(_, mailbox, source)| {
                (
                    mailbox.to_owned(),
                    source.source_url.clone(),
                    source.source_sha256.clone(),
                    source.observed_on.clone(),
                )
            })
            .collect();
        check!(eq; rows, vec![
            (MAILBOX_A.into(), CAPTURE_A.into(), "a".repeat(64), ACQUIRED_A.into()),
            ("coach@outlook.com".into(), "https://example.invalid/personal-c".into(), "c".repeat(64), "2026-08-15T00:00:00Z".into()),
        ]);
        check!(eq; scope.preferred().source_url, CAPTURE_A);
        coach.tenure_evidence.reverse();
    }
    Ok(())
}

#[test]
fn cen15_name_only_observation_cannot_replace_mailbox_provenance_in_either_order() -> TestResult {
    let bound = captured_coach(&school())?;
    let mut named = bound.clone();
    named.professional_email = None;
    named
        .tenure_evidence
        .retain(|fact| fact.source.url.as_deref() == Some(CAPTURE_B));
    for rows in [[bound.clone(), named.clone()], [named.clone(), bound]] {
        let result = selected(&rows, &athlete())?;
        check!(eq; result.email, MAILBOX_A);
        check!(eq; result.source_url, CAPTURE_A);
        check!(eq; result.source_sha256, "a".repeat(64));
        check!(eq; result.observed_on, ACQUIRED_A);
    }
    let result = selected(&[named], &athlete())?;
    check!(eq; result.state, ContactState::CoachNameOnly);
    check!(eq; result.email, "");
    check!(eq; result.source_url, CAPTURE_B);
    check!(eq; result.source_sha256, "b".repeat(64));
    check!(eq; result.observed_on, ACQUIRED_B);
    Ok(())
}

#[test]
fn cen15_conflicting_mailboxes_withhold_contact_and_capture_in_either_order() -> TestResult {
    let first = captured_coach(&school())?;
    let mut second = first.clone();
    second.professional_email = Some("other@example.invalid".into());
    second.tenure_evidence.retain(|fact| {
        fact.claim
            .as_ref()
            .is_some_and(|claim| claim.mailbox.is_some())
    });
    let fact = second.tenure_evidence.first_mut().ok_or("second fact")?;
    fact.source.url = Some("https://example.invalid/conflicting-c".into());
    fact.claim.as_mut().ok_or("second binding")?.mailbox = second.professional_email.clone();
    for rows in [[first.clone(), second.clone()], [second, first]] {
        let facts = contacts(&rows, year()?);
        let athlete = athlete();
        let scope = scoped(facts.get(school().as_str()), &athlete);
        let result = scope.preferred();
        check!(eq; result.state, ContactState::ContactConflict);
        check!(eq; result.email, "");
        check!(eq; result.source_url, "");
        check!(eq; result.source_sha256, "");
        check!(eq; result.observed_on, "");
        check!(eq; scope.all_emails(), "");
        check!(eq; scope.mailboxes().count(), 0);
    }
    Ok(())
}

#[test]
fn cen15_conflicting_claims_inside_one_merged_row_are_not_silently_discarded() -> TestResult {
    let mut coach = captured_coach(&school())?;
    let mut conflicting = coach.tenure_evidence.first().ok_or("mailbox fact")?.clone();
    conflicting
        .claim
        .as_mut()
        .ok_or("conflicting binding")?
        .mailbox = Some("other@example.invalid".into());
    coach.tenure_evidence.push(conflicting);
    for _ in 0..2 {
        check!(matches!(
            current_coach_contacts(&coach, year()?),
            Err(ContactSelectionError::MailboxConflict)
        ));
        check!(eq; selected(&[coach.clone()], &athlete())?.state, ContactState::ContactConflict);
        coach.tenure_evidence.reverse();
    }
    Ok(())
}

#[test]
fn cen15_repeated_mailbox_claims_select_one_exact_capture_independently_of_order() -> TestResult {
    let mut coach = captured_coach(&school())?;
    let mut second = coach.tenure_evidence.first().ok_or("mailbox fact")?.clone();
    second.source.url = Some("https://example.invalid/mailbox-z".into());
    second.source_sha256 = "c".repeat(64);
    coach.tenure_evidence.push(second);
    for _ in 0..2 {
        let result = selected(&[coach.clone()], &athlete())?;
        check!(eq; result.email, MAILBOX_A);
        check!(eq; result.source_url, "https://example.invalid/mailbox-z");
        check!(eq; result.source_sha256, "c".repeat(64));
        check!(eq; result.observed_on, ACQUIRED_A);
        coach.tenure_evidence.reverse();
    }
    Ok(())
}

#[test]
fn cen15_missing_mailbox_capture_url_cannot_borrow_a_generic_evidence_url() -> TestResult {
    let mut coach = captured_coach(&school())?;
    coach
        .tenure_evidence
        .first_mut()
        .ok_or("mailbox fact")?
        .source
        .url = None;
    check!(matches!(
        current_coach_contacts(&coach, year()?),
        Err(ContactSelectionError::MissingCaptureUrl)
    ));
    let result = selected(&[coach], &athlete())?;
    check!(eq; result.state, ContactState::ContactEvidenceInvalid);
    check!(eq; result.email, "");
    check!(eq; result.source_url, "");
    Ok(())
}
