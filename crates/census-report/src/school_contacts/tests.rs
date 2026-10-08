use super::*;
use census_domain::model::{ContactResearchAttempt, Gender, SourceRef, Sport};
use census_domain::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn school() -> CanonicalSchool {
    CanonicalSchool::new(
        UsJurisdiction::Arkansas,
        "Lifecycle High School",
        "lifecycle high school",
        None,
    )
    .0
}

fn year() -> Result<SchoolYear, Box<dyn std::error::Error>> {
    SchoolYear::new(2026).ok_or_else(|| "year".into())
}

fn record(
    school: &CanonicalSchool,
    outcome: ContactResearchOutcome,
    locator: &str,
    acquired_at: &str,
) -> TestResult<ContactResearch> {
    Ok(ContactResearch {
        school: school.id.clone(),
        subject: Subject::Program(CoachContactProgram::SchoolAthletics),
        school_year: year()?,
        outcome: outcome.clone(),
        attempts: vec![ContactResearchAttempt {
            locator: locator.to_owned(),
            acquired_at: acquired_at.to_owned(),
            source_sha256: Some("b".repeat(64)),
            outcome,
            reason: "explicit source assessment".to_owned(),
        }],
    })
}

#[test]
fn cen16_zero_coach_school_exposes_unattempted_programs() -> TestResult {
    let school = school();
    let rows = research_rows(std::slice::from_ref(&school), year()?)?;
    for program in programs() {
        let row = rows
            .iter()
            .find(|row| row.get(2) == Some(&program_label(&program)))
            .ok_or("owed program")?;
        check!(eq; row.get(4).map(String::as_str), Some("unattempted"));
    }
    let office = mailbox_row(&school, SchoolMailboxPurpose::SchoolOffice, year()?)?;
    check!(eq; office.get(5).map(String::as_str), Some("unattempted"));
    Ok(())
}

#[test]
fn cen16_source_retry_cannot_hide_another_blocked_source() -> TestResult {
    let mut school = school();
    school.contact_research.push(record(
        &school,
        ContactResearchOutcome::Blocked,
        "https://example.test/a",
        "2026-10-01T12:00:00Z",
    )?);
    school.contact_research.push(record(
        &school,
        ContactResearchOutcome::CompletedEmpty,
        "https://example.test/a",
        "2026-10-02T12:00:00Z",
    )?);
    check!(eq; research(&school, &CoachContactProgram::SchoolAthletics, year()?), ContactResearchOutcome::CompletedEmpty);
    school.contact_research.push(record(
        &school,
        ContactResearchOutcome::Blocked,
        "https://example.test/b",
        "2026-10-01T12:00:00Z",
    )?);
    check!(eq; research(&school, &CoachContactProgram::SchoolAthletics, year()?), ContactResearchOutcome::Blocked);
    let rows = research_rows(&[school], year()?)?;
    let audit = rows
        .iter()
        .find(|row| row.get(2).map(String::as_str) == Some("school_athletics"))
        .and_then(|row| row.get(5))
        .ok_or("research audit")?;
    let records: Vec<ContactResearch> = serde_json::from_str(audit)?;
    check!(records.iter().any(|row| row
        .attempts
        .iter()
        .any(|attempt| attempt.locator == "https://example.test/b"
            && attempt.outcome == ContactResearchOutcome::Blocked)));
    Ok(())
}

#[test]
fn cen16_unknown_gender_research_is_not_dropped_from_publication() -> TestResult {
    let mut school = school();
    let mut row = record(
        &school,
        ContactResearchOutcome::Ambiguous,
        "https://example.test/unknown",
        "2026-10-01T12:00:00Z",
    )?;
    row.subject = Subject::Program(CoachContactProgram::Team {
        sport: Sport::CrossCountry,
        gender: Gender::Unknown,
    });
    school.contact_research.push(row);
    let rows = research_rows(&[school], year()?)?;
    let row = rows
        .iter()
        .find(|row| row.get(2).map(String::as_str) == Some("CrossCountry:Unknown"))
        .ok_or("unknown gender research")?;
    check!(eq; row.get(4).map(String::as_str), Some("ambiguous"));
    Ok(())
}

#[test]
fn cen07_generic_office_capture_is_independent_of_blocked_ad_source() -> TestResult {
    let mut school = school();
    school.mailbox_claims.push(SchoolMailboxClaim {
        school: school.id.clone(),
        purpose: SchoolMailboxPurpose::SchoolOffice,
        mailbox: "office@school.edu".to_owned(),
        school_year: year()?,
        source: SourceRef::new(
            "school_sites",
            Some("https://example.test/office".to_owned()),
        ),
        source_sha256: "a".repeat(64),
        acquired_at: "2026-10-01T12:00:00Z".to_owned(),
        statement: "School office: office@school.edu".to_owned(),
    });
    let mut office = record(
        &school,
        ContactResearchOutcome::CompletedClaims,
        "https://example.test/office",
        "2026-10-01T12:00:00Z",
    )?;
    office.subject = Subject::SchoolMailbox(SchoolMailboxPurpose::SchoolOffice);
    if let Some(attempt) = office.attempts.first_mut() {
        attempt.source_sha256 = Some("a".repeat(64));
    }
    school.contact_research.push(office);
    school.contact_research.push(record(
        &school,
        ContactResearchOutcome::Blocked,
        "https://example.test/ad",
        "2026-10-02T12:00:00Z",
    )?);
    check!(eq; research(&school, &CoachContactProgram::SchoolAthletics, year()?), ContactResearchOutcome::Blocked);
    check!(eq; mailbox(&school, SchoolMailboxPurpose::SchoolOffice, year()?)?.map(|claim| claim.mailbox.as_str()), Some("office@school.edu"));
    Ok(())
}

#[test]
fn cen07_ad_claims_do_not_mark_never_attempted_office_complete() -> TestResult {
    let mut school = school();
    school.contact_research.push(record(
        &school,
        ContactResearchOutcome::CompletedClaims,
        "https://example.test/ad",
        "2026-10-01T12:00:00Z",
    )?);
    let row = mailbox_row(&school, SchoolMailboxPurpose::SchoolOffice, year()?)?;
    check!(eq; row.get(4).map(String::as_str), Some(""));
    check!(eq; row.get(5).map(String::as_str), Some("unattempted"));
    Ok(())
}

#[test]
fn cen07_same_capture_ad_conflict_does_not_withhold_generic_office() -> TestResult {
    let mut school = school();
    let mut captured = record(
        &school,
        ContactResearchOutcome::CompletedClaims,
        "https://example.test/office",
        "2026-10-01T12:00:00Z",
    )?;
    captured.subject = Subject::SchoolMailbox(SchoolMailboxPurpose::SchoolOffice);
    school.mailbox_claims.push(SchoolMailboxClaim {
        school: school.id.clone(),
        purpose: SchoolMailboxPurpose::SchoolOffice,
        mailbox: "office@school.edu".to_owned(),
        school_year: year()?,
        source: SourceRef::new(
            "school_sites",
            Some("https://example.test/office".to_owned()),
        ),
        source_sha256: "b".repeat(64),
        acquired_at: "2026-10-01T12:00:00Z".to_owned(),
        statement: "School office: office@school.edu".to_owned(),
    });
    school.contact_research.push(captured);
    school.contact_research.push(record(
        &school,
        ContactResearchOutcome::Conflict,
        "https://example.test/office",
        "2026-10-01T12:00:00Z",
    )?);
    check!(eq; research(&school, &CoachContactProgram::SchoolAthletics, year()?), ContactResearchOutcome::Conflict);
    check!(eq; mailbox(&school, SchoolMailboxPurpose::SchoolOffice, year()?)?.map(|claim| claim.mailbox.as_str()), Some("office@school.edu"));
    let mut later = record(
        &school,
        ContactResearchOutcome::Blocked,
        "https://example.test/office",
        "2026-10-02T12:00:00Z",
    )?;
    later.subject = Subject::SchoolMailbox(SchoolMailboxPurpose::SchoolOffice);
    school.contact_research.push(later);
    check!(eq; mailbox(&school, SchoolMailboxPurpose::SchoolOffice, year()?)?.map(|claim| claim.mailbox.as_str()), Some("office@school.edu"));
    check!(eq; census_domain::model::school_mailbox_research(&school, SchoolMailboxPurpose::SchoolOffice, year()?), ContactResearchOutcome::Blocked);
    Ok(())
}

#[test]
fn cen16_partial_row_cannot_complete_from_only_empty_attempts() -> TestResult {
    let mut school = school();
    let mut partial = record(
        &school,
        ContactResearchOutcome::CompletedEmpty,
        "https://example.test/directory",
        "2026-10-01T12:00:00Z",
    )?;
    partial.outcome = ContactResearchOutcome::Partial;
    school.contact_research.push(partial);
    check!(eq; research(&school, &CoachContactProgram::SchoolAthletics, year()?), ContactResearchOutcome::Partial);
    school.contact_research.push(record(
        &school,
        ContactResearchOutcome::CompletedEmpty,
        "https://example.test/directory",
        "2026-10-02T12:00:00Z",
    )?);
    check!(eq; research(&school, &CoachContactProgram::SchoolAthletics, year()?), ContactResearchOutcome::CompletedEmpty);
    Ok(())
}

#[test]
fn cen16_directory_exhaustion_retains_admitted_empty_without_certifying_failed_sources(
) -> TestResult {
    let mut school = school();
    school.contact_research.push(record(
        &school,
        ContactResearchOutcome::Exhausted,
        "https://example.test/index",
        "2026-10-01T12:00:00Z",
    )?);
    check!(eq; research(&school, &CoachContactProgram::SchoolAthletics, year()?), ContactResearchOutcome::Exhausted);
    school.contact_research.push(record(
        &school,
        ContactResearchOutcome::CompletedEmpty,
        "https://example.test/summary",
        "2026-10-01T12:00:00Z",
    )?);
    check!(eq; research(&school, &CoachContactProgram::SchoolAthletics, year()?), ContactResearchOutcome::CompletedEmpty);
    school.contact_research.push(record(
        &school,
        ContactResearchOutcome::Failed,
        "https://example.test/other",
        "2026-10-01T12:00:00Z",
    )?);
    check!(eq; research(&school, &CoachContactProgram::SchoolAthletics, year()?), ContactResearchOutcome::Failed);
    Ok(())
}

#[test]
fn cen16_exhaustion_inside_one_history_does_not_erase_actual_empty_capture() -> TestResult {
    let mut school = school();
    let mut history = record(
        &school,
        ContactResearchOutcome::CompletedEmpty,
        "https://example.test/summary",
        "2026-10-01T12:00:00Z",
    )?;
    history.outcome = ContactResearchOutcome::Exhausted;
    history.attempts.extend(
        record(
            &school,
            ContactResearchOutcome::Exhausted,
            "https://example.test/index",
            "2026-10-02T12:00:00Z",
        )?
        .attempts,
    );
    school.contact_research.push(history);
    check!(eq; research(&school, &CoachContactProgram::SchoolAthletics, year()?), ContactResearchOutcome::CompletedEmpty);
    check!(eq; census_domain::model::school_contact_source_research(&school, &CoachContactProgram::SchoolAthletics, year()?, "https://example.test/index"), ContactResearchOutcome::Exhausted);
    Ok(())
}

#[test]
fn cen16_every_research_state_is_distinct_in_zero_coach_publication() -> TestResult {
    for (outcome, label) in [
        (ContactResearchOutcome::Unattempted, "unattempted"),
        (ContactResearchOutcome::CompletedEmpty, "completed_empty"),
        (ContactResearchOutcome::CompletedClaims, "completed_claims"),
        (ContactResearchOutcome::Partial, "partial"),
        (ContactResearchOutcome::Blocked, "blocked"),
        (ContactResearchOutcome::Failed, "failed"),
        (ContactResearchOutcome::Exhausted, "exhausted"),
        (ContactResearchOutcome::Stale, "stale"),
        (ContactResearchOutcome::Ambiguous, "ambiguous"),
        (ContactResearchOutcome::Conflict, "conflict"),
    ] {
        let mut school = school();
        school.contact_research.push(record(
            &school,
            outcome.clone(),
            "https://example.test/source",
            "2026-10-01T12:00:00Z",
        )?);
        let rows = research_rows(&[school], year()?)?;
        let row = rows
            .iter()
            .find(|row| row.get(2).map(String::as_str) == Some("school_athletics"))
            .ok_or("published research state")?;
        check!(eq; row.get(4).map(String::as_str), Some(label));
        let retained: Vec<ContactResearch> =
            serde_json::from_str(row.get(5).ok_or("retained research audit")?)?;
        let attempt = retained
            .first()
            .and_then(|row| row.attempts.first())
            .ok_or("retained original attempt")?;
        check!(eq; attempt.outcome, outcome);
        check!(eq; attempt.locator, "https://example.test/source");
    }
    Ok(())
}

#[test]
fn invalid_acquisition_failure_remains_failed_in_both_replay_orders() -> TestResult {
    for reverse in [false, true] {
        let mut school = school();
        let failed = record(
            &school,
            ContactResearchOutcome::Failed,
            "https://example.test/office",
            "not-a-timestamp",
        )?;
        let completed = record(
            &school,
            ContactResearchOutcome::CompletedEmpty,
            "https://example.test/office",
            "2026-10-01T12:00:00Z",
        )?;
        let mut records = vec![failed, completed];
        for row in &mut records {
            row.subject = Subject::SchoolMailbox(SchoolMailboxPurpose::SchoolOffice);
        }
        if reverse {
            records.reverse();
        }
        school.contact_research = records;
        check!(eq; census_domain::model::school_mailbox_research(&school, SchoolMailboxPurpose::SchoolOffice, year()?), ContactResearchOutcome::Failed);
        let row = mailbox_row(&school, SchoolMailboxPurpose::SchoolOffice, year()?)?;
        check!(eq; row.get(4).map(String::as_str), Some(""));
        check!(eq; row.get(5).map(String::as_str), Some("failed"));
        let rows = research_rows(&[school], year()?)?;
        let row = rows
            .iter()
            .find(|row| row.get(2).map(String::as_str) == Some("school_office"))
            .ok_or("office research")?;
        let retained: Vec<ContactResearch> =
            serde_json::from_str(row.get(5).ok_or("retained attempts")?)?;
        check!(retained.iter().any(|row| row
            .attempts
            .iter()
            .any(|attempt| attempt.acquired_at == "not-a-timestamp"
                && attempt.outcome == ContactResearchOutcome::Failed)));
    }
    Ok(())
}
