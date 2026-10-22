use super::*;
use census_domain::model::SchoolMailboxPurpose;

fn reused_research(records: &[CanonicalSchool], expected: ContactState) -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    for record in records {
        store.append(Table::Schools, record)?;
    }
    drop(store);
    let store = Store::open(dir.path())?;
    let before = crate::export::ExportDataset::load(&store)?
        .lineage
        .snapshot_sequence;
    let athlete = athlete()?;
    for _ in 0..2 {
        let dataset = crate::export::ExportDataset::load(&store)?;
        check!(eq; dataset.lineage.snapshot_sequence, before);
        let retained = dataset.schools.get(&school()).ok_or("missing school")?;
        for research in records.iter().flat_map(|record| &record.contact_research) {
            check!(retained.contact_research.contains(research));
        }
        let mut contacts = contacts(&dataset.coach_observations, year()?);
        attach_research(&mut contacts, std::slice::from_ref(retained), year()?);
        let selected = scoped(contacts.get(school().as_str()), &athlete);
        let result = selected.preferred();
        check!(eq; result.state, expected);
        check!(eq; result.name, "");
        check!(eq; result.email, "");
    }
    Ok(())
}

fn recapture(record: &mut CanonicalSchool, at: &str) {
    for research in &mut record.contact_research {
        for attempt in &mut research.attempts {
            attempt.acquired_at = at.into();
            attempt.source_sha256 = Some("b".repeat(64));
        }
    }
}

#[test]
fn newer_same_source_completion_replaces_prior_failure_after_reopen() -> TestResult {
    let blocked = researched_school(ContactResearchOutcome::Blocked)?;
    let mut completed = researched_school(ContactResearchOutcome::CompletedEmpty)?;
    recapture(&mut completed, "2026-09-01T00:00:00Z");
    for records in [[blocked.clone(), completed.clone()], [completed, blocked]] {
        reused_research(&records, ContactState::ContactResearchEmpty)?;
    }
    Ok(())
}

#[test]
fn equal_time_conflicting_research_survives_reopen_in_either_order() -> TestResult {
    let blocked = researched_school(ContactResearchOutcome::Blocked)?;
    let completed = researched_school(ContactResearchOutcome::CompletedEmpty)?;
    for records in [[blocked.clone(), completed.clone()], [completed, blocked]] {
        reused_research(&records, ContactState::ContactResearchBlocked)?;
    }
    Ok(())
}

#[test]
fn another_source_completion_does_not_erase_a_retained_failure() -> TestResult {
    let blocked = researched_school(ContactResearchOutcome::Blocked)?;
    let mut completed = researched_school(ContactResearchOutcome::CompletedEmpty)?;
    recapture(&mut completed, "2026-09-01T00:00:00Z");
    for research in &mut completed.contact_research {
        for attempt in &mut research.attempts {
            attempt.locator = "https://other.example.invalid/staff".into();
        }
    }
    reused_research(&[completed, blocked], ContactState::ContactResearchBlocked)
}

#[test]
fn persisted_terminal_summary_without_capture_is_not_completion() -> TestResult {
    for missing in 0..ContactResearch::programs().len() {
        let mut record = researched_school(ContactResearchOutcome::CompletedEmpty)?;
        let research = record
            .contact_research
            .get_mut(missing)
            .ok_or("missing research")?;
        research.attempts.clear();
        reused_research(&[record], ContactState::ContactResearchBlocked)?;
    }
    Ok(())
}

#[test]
fn persisted_foreign_school_research_is_not_completion() -> TestResult {
    let mut record = researched_school(ContactResearchOutcome::CompletedEmpty)?;
    let research = record
        .contact_research
        .first_mut()
        .ok_or("missing research")?;
    research.school = SchoolId::mint("sch", &["another school"]);
    reused_research(&[record], ContactState::ContactResearchBlocked)
}

#[test]
fn persisted_terminal_attempt_requires_valid_capture() -> TestResult {
    for invalid in 0..4 {
        let mut record = researched_school(ContactResearchOutcome::CompletedEmpty)?;
        let research = record
            .contact_research
            .first_mut()
            .ok_or("missing research")?;
        let attempt = research.attempts.first_mut().ok_or("missing attempt")?;
        match invalid {
            0 => attempt.source_sha256 = None,
            1 => attempt.source_sha256 = Some("not-a-capture-digest".into()),
            2 => attempt.acquired_at = "not-a-retrieval-time".into(),
            _ => attempt.locator = "not-a-source-url".into(),
        }
        reused_research(&[record], ContactState::ContactResearchBlocked)?;
    }
    Ok(())
}

#[test]
fn cold_reopen_does_not_refresh_prior_season_research() -> TestResult {
    let mut record = researched_school(ContactResearchOutcome::CompletedEmpty)?;
    recapture(&mut record, "2025-08-01T00:00:00Z");
    for research in &mut record.contact_research {
        research.school_year = SchoolYear::new(2025).ok_or("invalid fixture season")?;
    }
    reused_research(&[record], ContactState::ContactResearchBlocked)
}

#[test]
fn one_completed_program_cannot_complete_all_seven_categories() -> TestResult {
    let mut record = researched_school(ContactResearchOutcome::CompletedEmpty)?;
    record.contact_research.truncate(1);
    reused_research(&[record], ContactState::ContactResearchUnknown)
}

#[test]
fn persisted_unattempted_research_remains_unknown_after_reopen() -> TestResult {
    reused_research(
        &[researched_school(ContactResearchOutcome::Unattempted)?],
        ContactState::ContactResearchUnknown,
    )
}

#[test]
fn office_mailbox_research_does_not_complete_coach_research() -> TestResult {
    let mut record = researched_school(ContactResearchOutcome::CompletedEmpty)?;
    record.contact_research.truncate(1);
    let research = record
        .contact_research
        .first_mut()
        .ok_or("missing research")?;
    research.subject = ContactResearchSubject::SchoolMailbox(SchoolMailboxPurpose::SchoolOffice);
    reused_research(&[record], ContactState::ContactResearchUnknown)
}
