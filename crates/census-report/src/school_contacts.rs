use crate::report::{ReportError, ReportResult};
use census_domain::model::{
    CanonicalSchool, CoachContactProgram, ContactResearch, ContactResearchOutcome,
    ContactResearchSubject as Subject, SchoolContactError, SchoolMailboxClaim,
    SchoolMailboxPurpose, SchoolYear,
};
use std::path::Path;

pub const MAILBOX_SHEET: &str = "School Contacts";
pub const RESEARCH_SHEET: &str = "Contact Research";
pub const MAILBOX_HEADERS: [&str; 10] = [
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
];
pub const RESEARCH_HEADERS: [&str; 6] = [
    "School ID",
    "School",
    "Subject",
    "School Year",
    "Research Outcome",
    "Retained Research",
];

pub fn mailbox(
    school: &CanonicalSchool,
    purpose: SchoolMailboxPurpose,
    year: SchoolYear,
) -> Result<Option<&SchoolMailboxClaim>, SchoolContactError> {
    census_domain::model::school_mailbox(school, purpose, year)
}

pub fn research(
    school: &CanonicalSchool,
    program: &CoachContactProgram,
    year: SchoolYear,
) -> ContactResearchOutcome {
    census_domain::model::school_contact_research(school, program, year)
}

pub fn programs() -> [CoachContactProgram; 7] {
    ContactResearch::programs()
}

pub fn mailbox_rows(
    schools: &[CanonicalSchool],
    year: SchoolYear,
) -> ReportResult<Vec<Vec<String>>> {
    let mut rows = vec![MAILBOX_HEADERS.map(str::to_owned).to_vec()];
    for school in ordered(schools) {
        for purpose in [
            SchoolMailboxPurpose::SchoolOffice,
            SchoolMailboxPurpose::AthleticsOffice,
        ] {
            rows.push(mailbox_row(school, purpose, year)?);
        }
    }
    Ok(rows)
}

pub fn research_rows(
    schools: &[CanonicalSchool],
    year: SchoolYear,
) -> ReportResult<Vec<Vec<String>>> {
    let mut rows = vec![RESEARCH_HEADERS.map(str::to_owned).to_vec()];
    for school in ordered(schools) {
        let programs = programs().map(Subject::Program);
        let mailboxes = [
            Subject::SchoolMailbox(SchoolMailboxPurpose::SchoolOffice),
            Subject::SchoolMailbox(SchoolMailboxPurpose::AthleticsOffice),
        ];
        let mut subjects: Vec<_> = programs.iter().chain(mailboxes.iter()).collect();
        for row in &school.contact_research {
            if !subjects.contains(&&row.subject) {
                subjects.push(&row.subject);
            }
        }
        for subject in subjects {
            rows.push(vec![
                school.id.to_string(),
                school.name.clone(),
                subject_label(subject),
                year.short(),
                subject_outcome(school, subject, year).as_str().to_owned(),
                encode(
                    &school
                        .contact_research
                        .iter()
                        .filter(|row| &row.subject == subject)
                        .collect::<Vec<_>>(),
                )?,
            ]);
        }
    }
    Ok(rows)
}

fn ordered(schools: &[CanonicalSchool]) -> Vec<&CanonicalSchool> {
    let mut schools: Vec<_> = schools.iter().collect();
    schools.sort_by(|left, right| left.id.cmp(&right.id));
    schools
}

fn mailbox_row(
    school: &CanonicalSchool,
    purpose: SchoolMailboxPurpose,
    year: SchoolYear,
) -> ReportResult<Vec<String>> {
    let claims: Vec<_> = school
        .mailbox_claims
        .iter()
        .filter(|claim| claim.purpose == purpose)
        .collect();
    let (selected, state) = match mailbox(school, purpose, year) {
        Ok(Some(claim)) => (Some(claim), "current_claim"),
        Ok(None) => (None, missing_state(school, purpose, year, &claims)),
        Err(SchoolContactError::Conflict) => (None, "conflict"),
        Err(SchoolContactError::ForeignSchool) => (None, "foreign_school"),
        Err(SchoolContactError::ResearchIncomplete(outcome)) => (None, outcome),
        Err(SchoolContactError::InvalidMailbox | SchoolContactError::InvalidCapture) => {
            (None, "invalid_claim")
        }
    };
    Ok(vec![
        school.id.to_string(),
        school.name.clone(),
        purpose_label(purpose).to_owned(),
        year.short(),
        selected.map_or_else(String::new, |claim| claim.mailbox.clone()),
        state.to_owned(),
        selected
            .and_then(|claim| claim.source.url.clone())
            .map_or(String::new(), core::convert::identity),
        selected.map_or_else(String::new, |claim| claim.source_sha256.clone()),
        selected.map_or_else(String::new, |claim| claim.acquired_at.clone()),
        encode(&claims)?,
    ])
}

fn missing_state(
    school: &CanonicalSchool,
    purpose: SchoolMailboxPurpose,
    year: SchoolYear,
    claims: &[&SchoolMailboxClaim],
) -> &'static str {
    let outcome = census_domain::model::school_mailbox_research(school, purpose, year);
    match outcome {
        ContactResearchOutcome::Unattempted
            if claims.iter().any(|claim| claim.school_year < year) =>
        {
            "stale"
        }
        ContactResearchOutcome::CompletedClaims => "partial",
        outcome => outcome.as_str(),
    }
}

pub fn program_label(program: &CoachContactProgram) -> String {
    match program {
        CoachContactProgram::SchoolAthletics => "school_athletics".to_owned(),
        CoachContactProgram::Team { sport, gender } => {
            format!("{}:{}", sport.stable_key(), gender.stable_key())
        }
    }
}

fn subject_label(subject: &Subject) -> String {
    match subject {
        Subject::Program(program) => program_label(program),
        Subject::SchoolMailbox(purpose) => purpose_label(*purpose).to_owned(),
    }
}

fn subject_outcome(
    school: &CanonicalSchool,
    subject: &Subject,
    year: SchoolYear,
) -> ContactResearchOutcome {
    match subject {
        Subject::Program(program) => research(school, program, year),
        Subject::SchoolMailbox(purpose) => {
            census_domain::model::school_mailbox_research(school, *purpose, year)
        }
    }
}

fn purpose_label(purpose: SchoolMailboxPurpose) -> &'static str {
    match purpose {
        SchoolMailboxPurpose::SchoolOffice => "school_office",
        SchoolMailboxPurpose::AthleticsOffice => "athletics_office",
    }
}

fn encode(value: &impl serde::Serialize) -> ReportResult<String> {
    serde_json::to_string(value).map_err(|error| ReportError::Invariant {
        detail: format!("encoding school contact audit: {error}"),
    })
}

pub fn write_csv(
    directory: &Path,
    schools: &[CanonicalSchool],
    year: SchoolYear,
) -> ReportResult<()> {
    write_rows(
        &directory.join("school-contacts.csv"),
        mailbox_rows(schools, year)?,
    )?;
    write_rows(
        &directory.join("contact-research.csv"),
        research_rows(schools, year)?,
    )
}

fn write_rows(path: &Path, rows: Vec<Vec<String>>) -> ReportResult<()> {
    let mut writer = csv::Writer::from_path(path).map_err(|error| csv_error(path, error))?;
    for row in rows {
        let row = row
            .into_iter()
            .map(crate::csv_safety::protect_owned)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| ReportError::Invariant {
                detail: format!("allocating school contact CSV field: {error}"),
            })?;
        writer
            .write_record(row)
            .map_err(|error| csv_error(path, error))?;
    }
    writer
        .flush()
        .map_err(|error| crate::report::io_error(path, error))?;
    writer
        .get_ref()
        .sync_all()
        .map_err(|error| crate::report::io_error(path, error))
}

fn csv_error(path: &Path, error: csv::Error) -> ReportError {
    crate::report::io_error(path, std::io::Error::other(error))
}

#[cfg(test)]
mod tests;
