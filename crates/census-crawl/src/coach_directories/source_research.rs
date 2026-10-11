use crate::net::FetchOutcome;
use census_domain::model::{
    CanonicalCoach, CanonicalSchool, CoachContactProgram, CoachRole, CoachTenure, ContactResearch,
    ContactResearchAttempt, ContactResearchOutcome as Outcome, ContactResearchSubject as Subject,
    Gender, SchoolYear,
};

pub(crate) fn staff_capture(
    school: &mut CanonicalSchool,
    year: SchoolYear,
    capture: &FetchOutcome,
    coaches: &[CanonicalCoach],
    baseline: Outcome,
) {
    let baseline = if chrono::DateTime::parse_from_rfc3339(&capture.fetched_at).is_err() {
        Outcome::Failed
    } else if capture.fetched_at.get(..10).and_then(SchoolYear::from_date) == Some(year) {
        baseline
    } else {
        baseline.combine(Outcome::Stale)
    };
    for program in ContactResearch::programs() {
        let outcome = assess(&program, year, coaches, baseline.clone());
        let attempt = ContactResearchAttempt { locator: capture.response_url.as_deref().map_or(capture.url.as_str(), |url| url).to_owned(),
            acquired_at: capture.fetched_at.clone(), source_sha256: Some(capture.content_digest.clone()), outcome: outcome.clone(),
            reason: "source-owned staff rows assessed without inferring current tenure from capture freshness".to_owned() };
        let row = ContactResearch {
            school: school.id.clone(),
            subject: Subject::Program(program),
            school_year: year,
            outcome,
            attempts: vec![attempt],
        };
        if !school.contact_research.contains(&row) {
            school.contact_research.push(row);
        }
    }
}

pub(crate) fn persist_staff_capture(
    ctx: &crate::AdapterContext<'_>,
    school: &CanonicalSchool,
    capture: &FetchOutcome,
    coaches: &[CanonicalCoach],
    baseline: Outcome,
) -> crate::CrawlResult<()> {
    let mut research = school_research(school)?;
    staff_capture(&mut research, ctx.school_year, capture, coaches, baseline);
    persist(ctx, &research)
}

pub(crate) fn persist_staff_attempt(
    ctx: &crate::AdapterContext<'_>,
    school: &CanonicalSchool,
    attempt: ContactResearchAttempt,
) -> crate::CrawlResult<()> {
    let mut research = school_research(school)?;
    crate::school_sites::retain_contact_attempt(&mut research, ctx.school_year, attempt);
    persist(ctx, &research)
}

pub(crate) fn persist_unattempted(
    ctx: &crate::AdapterContext<'_>,
    school: &CanonicalSchool,
) -> crate::CrawlResult<()> {
    let mut research = school_research(school)?;
    for program in ContactResearch::programs() {
        research.contact_research.push(ContactResearch {
            school: school.id.clone(),
            subject: Subject::Program(program),
            school_year: ctx.school_year,
            outcome: Outcome::Unattempted,
            attempts: Vec::new(),
        });
    }
    persist(ctx, &research)
}

fn school_research(school: &CanonicalSchool) -> crate::CrawlResult<CanonicalSchool> {
    let state = school.state.ok_or_else(|| crate::CrawlError::Invariant {
        detail: "staff research has no school jurisdiction".to_owned(),
    })?;
    let (research, _) = CanonicalSchool::new(
        state,
        &school.name,
        &school.normalized_name,
        school.city.as_deref(),
    );
    if research.id != school.id {
        return Err(crate::CrawlError::Invariant {
            detail: "staff research school owner differs from canonical source owner".to_owned(),
        });
    }
    Ok(research)
}

fn persist(ctx: &crate::AdapterContext<'_>, school: &CanonicalSchool) -> crate::CrawlResult<()> {
    let digest = census_domain::model::serialized_digest(school).map_err(|source| {
        crate::CrawlError::Canonical {
            table: "school contact research".to_owned(),
            source,
        }
    })?;
    let operation = format!("staff_capture_research_v1:{digest}");
    if ctx.effect_is_committed(&operation, &digest)? {
        return Ok(());
    }
    let mut batch = ctx.write_batch();
    batch.append_many(census_store::Table::Schools, std::slice::from_ref(school))?;
    batch.journal_done(
        "staff_capture_research_v1",
        &operation,
        &serde_json::json!({"school":school.id,"sha256":digest}),
    )?;
    batch.commit_once(&operation, &digest)?;
    Ok(())
}

fn assess(
    program: &CoachContactProgram,
    year: SchoolYear,
    coaches: &[CanonicalCoach],
    mut state: Outcome,
) -> Outcome {
    let mut current = false;
    for coach in coaches.iter().filter(|coach| matching(program, coach)) {
        let next = match coach.tenure_state(year) {
            Ok(CoachTenure::Current { .. }) if coach.gender != Gender::Unknown || *program == CoachContactProgram::SchoolAthletics => Outcome::CompletedClaims,
            Ok(CoachTenure::Current { .. }) => Outcome::Ambiguous,
            Ok(CoachTenure::Former { .. }) => Outcome::Stale,
            Ok(CoachTenure::Unknown) if coach.tenure_evidence.iter().any(|fact| matches!(fact.tenure, CoachTenure::Current { school_year } if school_year < year)) => Outcome::Stale,
            Ok(CoachTenure::Unknown) => Outcome::Ambiguous,
            Err(_) => Outcome::Conflict,
        };
        current |= next == Outcome::CompletedClaims;
        state = state.combine(next);
    }
    if current && state == Outcome::Stale {
        Outcome::CompletedClaims
    } else {
        state
    }
}

fn matching(program: &CoachContactProgram, coach: &CanonicalCoach) -> bool {
    match program {
        CoachContactProgram::SchoolAthletics => coach.role == CoachRole::AthleticDirector,
        CoachContactProgram::Team { sport, gender } => {
            coach.sport == Some(*sport)
                && (coach.gender == *gender
                    || matches!(coach.gender, Gender::Mixed | Gender::Unknown))
        }
    }
}
