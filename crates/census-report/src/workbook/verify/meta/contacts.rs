use super::{Expect, Sheet};
use crate::report::{ReportError, ReportResult};
use census_domain::model::{
    school_contact_research, school_mailbox, school_mailbox_research, CanonicalSchool,
    CoachContactProgram, ContactResearch, ContactResearchOutcome as Outcome,
    ContactResearchSubject as Subject, SchoolContactError, SchoolMailboxClaim,
    SchoolMailboxPurpose as Purpose, SchoolYear,
};

pub(in crate::workbook) fn expected(
    schools: &[CanonicalSchool],
    year: SchoolYear,
) -> ReportResult<[Sheet; 2]> {
    let mut ordered: Vec<_> = schools.iter().collect();
    ordered.sort_by(|left, right| left.id.cmp(&right.id));
    let mut mailboxes = vec![super::header(&[
        "School ID",
        "School",
        "Purpose",
        "School Year",
        "Mailbox",
        "Contact State",
        "Source URL",
        "Capture SHA256",
        "Acquired At",
        "Retained Claims",
    ])];
    let mut research = vec![super::header(&[
        "School ID",
        "School",
        "Subject",
        "School Year",
        "Research Outcome",
        "Retained Research",
    ])];
    for school in ordered {
        for purpose in [Purpose::SchoolOffice, Purpose::AthleticsOffice] {
            mailboxes.push(mailbox_row(school, purpose, year)?);
        }
        for subject in subjects(school) {
            research.push(research_row(school, &subject, year)?);
        }
    }
    Ok([
        ("School Contacts", mailboxes),
        ("Contact Research", research),
    ])
}

fn subjects(school: &CanonicalSchool) -> Vec<Subject> {
    let mut subjects: Vec<_> = ContactResearch::programs()
        .into_iter()
        .map(Subject::Program)
        .collect();
    subjects.extend([
        Subject::SchoolMailbox(Purpose::SchoolOffice),
        Subject::SchoolMailbox(Purpose::AthleticsOffice),
    ]);
    for record in &school.contact_research {
        if !subjects.contains(&record.subject) {
            subjects.push(record.subject.clone());
        }
    }
    subjects
}

fn identity(school: &CanonicalSchool, label: String, year: SchoolYear) -> Vec<Expect> {
    vec![
        Expect::Text(school.id.to_string()),
        Expect::text(&school.name),
        Expect::Text(label),
        Expect::Text(year.short()),
    ]
}

fn mailbox_row(
    school: &CanonicalSchool,
    purpose: Purpose,
    year: SchoolYear,
) -> ReportResult<Vec<Expect>> {
    let retained: Vec<_> = school
        .mailbox_claims
        .iter()
        .filter(|claim| claim.purpose == purpose)
        .collect();
    let (claim, state) = selected(school, purpose, year, &retained);
    let mut row = identity(school, purpose_label(purpose).to_owned(), year);
    match claim {
        Some(claim) => row.extend([
            Expect::text(&claim.mailbox),
            Expect::text(state),
            claim
                .source
                .url
                .as_deref()
                .map_or(Expect::Empty, Expect::text),
            Expect::text(&claim.source_sha256),
            Expect::text(&claim.acquired_at),
        ]),
        None => row.extend([
            Expect::Empty,
            Expect::text(state),
            Expect::Empty,
            Expect::Empty,
            Expect::Empty,
        ]),
    }
    row.push(Expect::Text(encode(&retained)?));
    Ok(row)
}

fn selected<'a>(
    school: &'a CanonicalSchool,
    purpose: Purpose,
    year: SchoolYear,
    retained: &[&SchoolMailboxClaim],
) -> (Option<&'a SchoolMailboxClaim>, &'static str) {
    match school_mailbox(school, purpose, year) {
        Ok(Some(claim)) => (Some(claim), "current_claim"),
        Ok(None) => {
            let outcome = school_mailbox_research(school, purpose, year);
            let state = match outcome {
                Outcome::Unattempted if retained.iter().any(|claim| claim.school_year < year) => {
                    "stale"
                }
                Outcome::CompletedClaims => "partial",
                outcome => outcome.as_str(),
            };
            (None, state)
        }
        Err(SchoolContactError::Conflict) => (None, "conflict"),
        Err(SchoolContactError::ForeignSchool) => (None, "foreign_school"),
        Err(SchoolContactError::ResearchIncomplete(outcome)) => (None, outcome),
        Err(SchoolContactError::InvalidMailbox | SchoolContactError::InvalidCapture) => {
            (None, "invalid_claim")
        }
    }
}

fn research_row(
    school: &CanonicalSchool,
    subject: &Subject,
    year: SchoolYear,
) -> ReportResult<Vec<Expect>> {
    let (label, outcome) = match subject {
        Subject::Program(program) => (
            program_label(program),
            school_contact_research(school, program, year),
        ),
        Subject::SchoolMailbox(purpose) => (
            purpose_label(*purpose).to_owned(),
            school_mailbox_research(school, *purpose, year),
        ),
    };
    let retained: Vec<_> = school
        .contact_research
        .iter()
        .filter(|record| &record.subject == subject)
        .collect();
    let mut row = identity(school, label, year);
    row.extend([
        Expect::text(outcome.as_str()),
        Expect::Text(encode(&retained)?),
    ]);
    Ok(row)
}

fn program_label(program: &CoachContactProgram) -> String {
    match program {
        CoachContactProgram::SchoolAthletics => "school_athletics".to_owned(),
        CoachContactProgram::Team { sport, gender } => {
            format!("{}:{}", sport.stable_key(), gender.stable_key())
        }
    }
}

fn purpose_label(purpose: Purpose) -> &'static str {
    match purpose {
        Purpose::SchoolOffice => "school_office",
        Purpose::AthleticsOffice => "athletics_office",
    }
}

fn encode(value: &impl serde::Serialize) -> ReportResult<String> {
    serde_json::to_string(value).map_err(|error| ReportError::Invariant {
        detail: format!("encoding frozen contact evidence: {error}"),
    })
}
