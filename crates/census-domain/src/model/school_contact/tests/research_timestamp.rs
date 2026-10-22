use crate::model::{
    school_contact_research, school_contact_source_research, school_mailbox_research,
    CanonicalSchool, CoachContactProgram, ContactResearch, ContactResearchAttempt,
    ContactResearchOutcome as Outcome, ContactResearchSubject as Subject, SchoolMailboxPurpose,
    SchoolYear,
};
use crate::UsJurisdiction;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SOURCE: &str = "https://school.test/athletics/staff";
const OTHER_SOURCE: &str = "https://school.test/athletics/directory";

fn attempt(at: &str, outcome: Outcome) -> ContactResearchAttempt {
    ContactResearchAttempt {
        locator: SOURCE.into(),
        acquired_at: at.into(),
        source_sha256: Some("a".repeat(64)),
        outcome,
        reason: "retained school research".into(),
    }
}

fn school(at: &str, outcome: Outcome) -> CanonicalSchool {
    let (mut school, _) = CanonicalSchool::new(
        UsJurisdiction::Iowa,
        "Source School",
        "source school",
        Some("Milford"),
    );
    school.contact_research.push(ContactResearch {
        school: school.id.clone(),
        subject: Subject::Program(CoachContactProgram::SchoolAthletics),
        school_year: SchoolYear::DEFAULT,
        outcome: outcome.clone(),
        attempts: vec![attempt(at, outcome)],
    });
    school
}

fn outcome(school: &CanonicalSchool) -> Outcome {
    school_contact_research(
        school,
        &CoachContactProgram::SchoolAthletics,
        SchoolYear::DEFAULT,
    )
}

#[test]
fn terminal_research_uses_existing_august_through_july_capture_interval() -> TestResult {
    for terminal in [Outcome::CompletedEmpty, Outcome::CompletedClaims] {
        for at in ["2026-08-01T00:00:00Z", "2027-07-31T23:59:59Z"] {
            check!(eq; outcome(&school(at, terminal.clone())), terminal);
        }
        for at in ["2026-07-31T23:59:59Z", "2027-08-01T00:00:00Z"] {
            check!(eq; outcome(&school(at, terminal.clone())), Outcome::Stale);
        }
    }
    Ok(())
}

#[test]
fn falsely_current_school_year_does_not_refresh_old_capture_evidence() -> TestResult {
    for at in ["2023-10-01T12:00:00Z", "2025-10-01T12:00:00Z"] {
        for terminal in [Outcome::CompletedEmpty, Outcome::CompletedClaims] {
            let school = school(at, terminal);
            let bytes = serde_json::to_vec(&school)?;
            for _ in 0..3 {
                check!(eq; outcome(&school), Outcome::Stale);
                check!(eq; serde_json::to_vec(&school)?, bytes);
            }
            let reopened: CanonicalSchool = serde_json::from_slice(&bytes)?;
            check!(eq; outcome(&reopened), Outcome::Stale);
            check!(eq; serde_json::to_vec(&reopened)?, bytes);
        }
    }
    Ok(())
}

#[test]
fn capture_year_keeps_existing_rfc3339_local_offset_calendar_semantics() -> TestResult {
    for at in ["2026-08-01T00:30:00+14:00", "2027-07-31T23:30:00-12:00"] {
        check!(eq; outcome(&school(at, Outcome::CompletedEmpty)), Outcome::CompletedEmpty);
    }
    for at in ["2026-07-31T23:30:00-12:00", "2027-08-01T00:30:00+14:00"] {
        check!(eq; outcome(&school(at, Outcome::CompletedEmpty)), Outcome::Stale);
    }
    Ok(())
}

#[test]
fn newest_stale_attempt_is_selected_before_classification_without_reviving_success() -> TestResult {
    let mut school = school("2026-09-01T00:00:00Z", Outcome::CompletedEmpty);
    let row = school
        .contact_research
        .first_mut()
        .ok_or("research fixture")?;
    row.attempts
        .push(attempt("2027-08-01T00:00:00Z", Outcome::CompletedEmpty));
    check!(eq; outcome(&school), Outcome::Stale);
    school
        .contact_research
        .first_mut()
        .ok_or("research fixture")?
        .attempts
        .reverse();
    check!(eq; outcome(&school), Outcome::Stale);
    Ok(())
}

#[test]
fn newer_in_year_attempt_can_recover_older_stale_evidence_at_the_same_source() -> TestResult {
    let mut school = school("2025-09-01T00:00:00Z", Outcome::CompletedEmpty);
    let row = school
        .contact_research
        .first_mut()
        .ok_or("research fixture")?;
    row.attempts
        .push(attempt("2026-09-01T00:00:00Z", Outcome::CompletedEmpty));
    check!(eq; outcome(&school), Outcome::CompletedEmpty);
    school
        .contact_research
        .first_mut()
        .ok_or("research fixture")?
        .attempts
        .reverse();
    check!(eq; outcome(&school), Outcome::CompletedEmpty);
    Ok(())
}

#[test]
fn stale_evidence_from_another_source_is_not_discarded_or_promoted_to_completion() -> TestResult {
    let mut school = school("2026-09-01T00:00:00Z", Outcome::CompletedEmpty);
    let row = school
        .contact_research
        .first_mut()
        .ok_or("research fixture")?;
    let mut stale = attempt("2025-09-01T00:00:00Z", Outcome::CompletedEmpty);
    stale.locator = OTHER_SOURCE.into();
    row.attempts.push(stale);
    check!(eq; outcome(&school), Outcome::Stale);
    check!(eq; school_contact_source_research(&school, &CoachContactProgram::SchoolAthletics,
        SchoolYear::DEFAULT, SOURCE), Outcome::CompletedEmpty);
    check!(eq; school_contact_source_research(&school, &CoachContactProgram::SchoolAthletics,
        SchoolYear::DEFAULT, OTHER_SOURCE), Outcome::Stale);
    Ok(())
}

#[test]
fn out_of_year_attempts_preserve_genuine_denial_error_and_conflict_outcomes() -> TestResult {
    for preserved in [
        Outcome::Failed,
        Outcome::Blocked,
        Outcome::Exhausted,
        Outcome::Partial,
        Outcome::Ambiguous,
        Outcome::Conflict,
    ] {
        check!(eq; outcome(&school("2025-09-01T00:00:00Z", preserved.clone())), preserved);
    }
    for summary in [
        Outcome::Failed,
        Outcome::Blocked,
        Outcome::Ambiguous,
        Outcome::Conflict,
    ] {
        let mut school = school("2025-09-01T00:00:00Z", Outcome::CompletedEmpty);
        school
            .contact_research
            .first_mut()
            .ok_or("research fixture")?
            .outcome = summary.clone();
        check!(eq; outcome(&school), summary);
    }
    Ok(())
}

#[test]
fn malformed_terminal_timestamp_and_foreign_owner_remain_conflicts() -> TestResult {
    for at in ["", "2026-08-01", "not-a-timestamp", "2026-02-30T00:00:00Z"] {
        check!(eq; outcome(&school(at, Outcome::CompletedEmpty)), Outcome::Conflict);
    }
    let mut school = school("2025-09-01T00:00:00Z", Outcome::CompletedEmpty);
    let (_, foreign) = CanonicalSchool::new(UsJurisdiction::Iowa, "Other", "other", None);
    school
        .contact_research
        .first_mut()
        .ok_or("research fixture")?
        .school = foreign;
    check!(eq; outcome(&school), Outcome::Conflict);
    check!(eq; outcome(&self::school("", Outcome::Failed)), Outcome::Failed);
    Ok(())
}

#[test]
fn stale_empty_capture_cannot_suppress_exhaustion_from_another_source() -> TestResult {
    let mut school = school("2025-09-01T00:00:00Z", Outcome::CompletedEmpty);
    let mut exhausted = attempt("2026-09-01T00:00:00Z", Outcome::Exhausted);
    exhausted.locator = OTHER_SOURCE.into();
    school.contact_research.push(ContactResearch {
        school: school.id.clone(),
        subject: Subject::Program(CoachContactProgram::SchoolAthletics),
        school_year: SchoolYear::DEFAULT,
        outcome: Outcome::Exhausted,
        attempts: vec![exhausted],
    });
    check!(eq; outcome(&school), Outcome::Stale);
    Ok(())
}

#[test]
fn mailbox_research_uses_the_same_capture_year_rule_without_publishing_a_mailbox() -> TestResult {
    let mut school = school("2025-09-01T00:00:00Z", Outcome::CompletedEmpty);
    school
        .contact_research
        .first_mut()
        .ok_or("research fixture")?
        .subject = Subject::SchoolMailbox(SchoolMailboxPurpose::SchoolOffice);
    check!(eq; school_mailbox_research(&school, SchoolMailboxPurpose::SchoolOffice,
        SchoolYear::DEFAULT), Outcome::Stale);
    check!(school.mailbox_claims.is_empty());
    Ok(())
}

#[test]
fn equal_timestamp_terminal_disagreement_stays_conflict_even_outside_year() -> TestResult {
    let mut school = school("2025-09-01T00:00:00Z", Outcome::CompletedEmpty);
    school
        .contact_research
        .first_mut()
        .ok_or("research fixture")?
        .attempts
        .push(attempt("2025-09-01T00:00:00Z", Outcome::CompletedClaims));
    check!(eq; outcome(&school), Outcome::Conflict);
    Ok(())
}

#[test]
fn equal_instants_with_different_local_capture_years_are_conflicts_in_both_orders() -> TestResult {
    for terminal in [Outcome::CompletedEmpty, Outcome::CompletedClaims] {
        let mut school = school("2026-08-01T00:30:00+02:00", terminal.clone());
        school
            .contact_research
            .first_mut()
            .ok_or("research fixture")?
            .attempts
            .push(attempt("2026-07-31T22:30:00Z", terminal));
        check!(eq; outcome(&school), Outcome::Conflict);
        school
            .contact_research
            .first_mut()
            .ok_or("research fixture")?
            .attempts
            .reverse();
        check!(eq; outcome(&school), Outcome::Conflict);
    }
    Ok(())
}

#[test]
fn equal_instants_with_matching_capture_year_classifications_preserve_existing_ties() -> TestResult
{
    for terminal in [Outcome::CompletedEmpty, Outcome::CompletedClaims] {
        for (first, second, expected) in [
            (
                "2026-09-01T00:30:00+02:00",
                "2026-08-31T22:30:00Z",
                terminal.clone(),
            ),
            (
                "2025-09-01T00:30:00+02:00",
                "2025-08-31T22:30:00Z",
                Outcome::Stale,
            ),
        ] {
            let mut school = school(first, terminal.clone());
            school
                .contact_research
                .first_mut()
                .ok_or("research fixture")?
                .attempts
                .push(attempt(second, terminal.clone()));
            check!(eq; outcome(&school), expected);
            school
                .contact_research
                .first_mut()
                .ok_or("research fixture")?
                .attempts
                .reverse();
            check!(eq; outcome(&school), expected);
        }
    }
    Ok(())
}

#[test]
fn equal_instant_year_disagreement_does_not_weaken_matching_denial_outcomes() -> TestResult {
    for preserved in [Outcome::Failed, Outcome::Blocked, Outcome::Exhausted] {
        let mut school = school("2026-08-01T00:30:00+02:00", preserved.clone());
        school
            .contact_research
            .first_mut()
            .ok_or("research fixture")?
            .attempts
            .push(attempt("2026-07-31T22:30:00Z", preserved.clone()));
        check!(eq; outcome(&school), preserved);
        school
            .contact_research
            .first_mut()
            .ok_or("research fixture")?
            .attempts
            .reverse();
        check!(eq; outcome(&school), preserved);
    }
    Ok(())
}
