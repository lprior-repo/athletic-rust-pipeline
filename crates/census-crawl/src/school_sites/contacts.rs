use crate::net::FetchOutcome;
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{
    CanonicalSchool, CoachContactProgram, ContactResearch, ContactResearchAttempt,
    ContactResearchOutcome as Outcome, ContactResearchSubject as Subject, SchoolYear,
};
use census_store::Table;

#[tracing::instrument(skip(ctx, school))]
pub async fn collect_contacts(
    ctx: &AdapterContext<'_>,
    school: &mut CanonicalSchool,
) -> CrawlResult<AdapterReport> {
    let before = ctx.fetcher.stats().await;
    let checkpoint = (school.mailbox_claims.len(), school.contact_research.len());
    initialize(school, ctx.school_year)?;
    persist(ctx, school, checkpoint)?;
    crate::coach_directories::research_school_mailboxes(ctx, school).await?;
    let mut report = research_report(school, ctx.school_year);
    let after = ctx.fetcher.stats().await;
    report.requests = after
        .requests
        .checked_sub(before.requests)
        .ok_or_else(counter_error)?;
    report.from_cache = after
        .cache_hits
        .checked_sub(before.cache_hits)
        .ok_or_else(counter_error)?;
    report.rows = 1;
    if !matches!(
        report.disposition,
        crate::CollectionDisposition::Blocked | crate::CollectionDisposition::Failed
    ) {
        report.finish_frontier();
    }
    Ok(report)
}

fn research_report(school: &CanonicalSchool, year: SchoolYear) -> AdapterReport {
    let mut report = AdapterReport::new(super::SOURCE_ID, "school_contacts");
    for program in ContactResearch::programs() {
        let outcome = census_domain::model::school_contact_research(school, &program, year);
        if !outcome.is_terminal() {
            report
                .unfinished
                .push(format!("{}:{program:?}:{}", school.id, outcome.as_str()));
        }
        disposition(&mut report, &outcome);
    }
    for purpose in [
        census_domain::model::SchoolMailboxPurpose::SchoolOffice,
        census_domain::model::SchoolMailboxPurpose::AthleticsOffice,
    ] {
        let outcome = census_domain::model::school_mailbox_research(school, purpose, year);
        if !outcome.is_terminal() {
            report
                .unfinished
                .push(format!("{}:{purpose:?}:{}", school.id, outcome.as_str()));
        }
        disposition(&mut report, &outcome);
    }
    report
}

pub fn apply_contact_capture(
    school: &mut CanonicalSchool,
    year: SchoolYear,
    capture: &FetchOutcome,
) -> CrawlResult<Vec<String>> {
    let links = crate::coach_directories::apply_school_mailbox_capture(school, year, capture)?;
    let locator = capture
        .response_url
        .as_deref()
        .map_or(capture.url.as_str(), |url| url);
    let mut state = school
        .contact_research
        .iter()
        .filter(|row| matches!(row.subject, Subject::SchoolMailbox(_)) && row.school_year == year)
        .flat_map(|row| &row.attempts)
        .filter(|attempt| {
            attempt.locator == locator
                && attempt.acquired_at == capture.fetched_at
                && attempt.source_sha256.as_deref() == Some(capture.content_digest.as_str())
        })
        .fold(Outcome::CompletedEmpty, |state, attempt| {
            state.combine(attempt.outcome.clone())
        });
    let mut signals = super::Signals::default();
    let mut reason = "searched retained source page; candidate names never establish current tenure or mailbox claims".to_owned();
    if state.is_terminal() {
        let text = std::str::from_utf8(&capture.body).map_err(|_| CrawlError::Schema {
            url: locator.to_owned(),
            detail: "school contact page is not UTF-8".to_owned(),
        })?;
        if let Err(error) = super::analyse(&super::Rules::new()?, locator, text, &mut signals) {
            state = if matches!(error, CrawlError::Resource { .. }) {
                Outcome::Partial
            } else {
                Outcome::Failed
            };
            reason = error.to_string();
        }
    }
    for program in ContactResearch::programs() {
        let outcome = if state.is_terminal() {
            program_outcome(&program, &signals)
        } else {
            state.clone()
        };
        append(
            school,
            year,
            program,
            ContactResearchAttempt {
                locator: locator.to_owned(),
                acquired_at: capture.fetched_at.clone(),
                source_sha256: Some(capture.content_digest.clone()),
                outcome,
                reason: reason.clone(),
            },
        );
    }
    Ok(links)
}

pub(crate) fn retain_contact_attempt(
    school: &mut CanonicalSchool,
    year: SchoolYear,
    attempt: ContactResearchAttempt,
) {
    for program in ContactResearch::programs() {
        append(school, year, program, attempt.clone());
    }
}

fn append(
    school: &mut CanonicalSchool,
    year: SchoolYear,
    program: CoachContactProgram,
    attempt: ContactResearchAttempt,
) {
    let row = ContactResearch {
        school: school.id.clone(),
        subject: Subject::Program(program),
        school_year: year,
        outcome: attempt.outcome.clone(),
        attempts: vec![attempt],
    };
    if !school.contact_research.contains(&row) {
        school.contact_research.push(row);
    }
}

fn program_outcome(program: &CoachContactProgram, signals: &super::Signals) -> Outcome {
    let candidate = match program {
        CoachContactProgram::SchoolAthletics => !signals.ad_hits.is_empty(),
        CoachContactProgram::Team { sport, gender } => signals.coach_hits.iter().any(|hit| {
            hit.sport == *sport
                && (hit.gender == *gender
                    || matches!(
                        hit.gender,
                        census_domain::model::Gender::Unknown | census_domain::model::Gender::Mixed
                    ))
        }),
    };
    if candidate {
        Outcome::Ambiguous
    } else {
        Outcome::CompletedEmpty
    }
}

fn initialize(school: &mut CanonicalSchool, year: SchoolYear) -> CrawlResult<()> {
    if school.contact_research.len() > 61_440 {
        return Err(CrawlError::Resource {
            resource: "school contact research",
            requested: school.contact_research.len(),
            limit: 65_536,
        });
    }
    for program in ContactResearch::programs() {
        if school
            .contact_research
            .iter()
            .any(|row| row.school_year == year && row.subject == Subject::Program(program.clone()))
        {
            continue;
        }
        school.contact_research.push(ContactResearch {
            school: school.id.clone(),
            subject: Subject::Program(program),
            school_year: year,
            outcome: Outcome::Unattempted,
            attempts: Vec::new(),
        });
    }
    Ok(())
}

pub(crate) fn persist(
    ctx: &AdapterContext<'_>,
    school: &CanonicalSchool,
    checkpoint: (usize, usize),
) -> CrawlResult<()> {
    let claims =
        school
            .mailbox_claims
            .get(checkpoint.0..)
            .ok_or_else(|| CrawlError::Invariant {
                detail: "school mailbox checkpoint reversed".to_owned(),
            })?;
    let research =
        school
            .contact_research
            .get(checkpoint.1..)
            .ok_or_else(|| CrawlError::Invariant {
                detail: "school research checkpoint reversed".to_owned(),
            })?;
    if claims.is_empty() && research.is_empty() {
        return Ok(());
    }
    let state = school.state.ok_or_else(|| CrawlError::Invariant {
        detail: "school contact owner has no jurisdiction".to_owned(),
    })?;
    let (mut delta, _) = CanonicalSchool::new(
        state,
        &school.name,
        &school.normalized_name,
        school.city.as_deref(),
    );
    if delta.id != school.id {
        return Err(CrawlError::Invariant {
            detail: "school contact owner identity differs from its canonical fields".to_owned(),
        });
    }
    delta.school_website.clone_from(&school.school_website);
    delta
        .mailbox_claims
        .try_reserve(claims.len())
        .map_err(|_| CrawlError::Resource {
            resource: "school contact claims",
            requested: claims.len(),
            limit: 65_536,
        })?;
    delta
        .contact_research
        .try_reserve(research.len())
        .map_err(|_| CrawlError::Resource {
            resource: "school contact attempts",
            requested: research.len(),
            limit: 65_536,
        })?;
    delta.mailbox_claims.extend_from_slice(claims);
    delta.contact_research.extend_from_slice(research);
    commit(ctx, &delta)
}

fn commit(ctx: &AdapterContext<'_>, school: &CanonicalSchool) -> CrawlResult<()> {
    let digest = census_domain::model::serialized_digest(school).map_err(|source| {
        CrawlError::Canonical {
            table: "schools".to_owned(),
            source,
        }
    })?;
    let operation = format!("school_sites_contacts_v1:{}:{}", school.id, digest);
    if ctx.effect_is_committed(&operation, &digest)? {
        return Ok(());
    }
    let mut batch = ctx.write_batch();
    batch.append_many(Table::Schools, std::slice::from_ref(school))?;
    batch.journal_done(
        "school_sites_contacts_v1",
        &operation,
        &serde_json::json!({"school":school.id,"sha256":digest}),
    )?;
    batch.commit_once(&operation, &digest)?;
    Ok(())
}

fn disposition(report: &mut AdapterReport, state: &Outcome) {
    match state {
        Outcome::Blocked => report.disposition = crate::CollectionDisposition::Blocked,
        Outcome::Failed if report.disposition != crate::CollectionDisposition::Blocked => {
            report.disposition = crate::CollectionDisposition::Failed
        }
        _ => {}
    }
}

fn counter_error() -> CrawlError {
    CrawlError::Arithmetic {
        detail: "school contact statistics reversed".to_owned(),
    }
}
