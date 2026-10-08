use super::{
    CoachContactProgram, ContactResearchAttempt, ContactResearchOutcome as Outcome,
    ContactResearchSubject as Subject, SchoolContactError, SchoolMailboxClaim,
    SchoolMailboxPurpose, SchoolYear,
};
use crate::model::CanonicalSchool;
use std::collections::BTreeMap;

pub fn mailbox(
    school: &CanonicalSchool,
    purpose: SchoolMailboxPurpose,
    year: SchoolYear,
) -> Result<Option<&SchoolMailboxClaim>, SchoolContactError> {
    let selected = select(school, purpose, year)?;
    if let Some(claim) = selected {
        let outcome = assess(school, &Subject::SchoolMailbox(purpose), year, |attempt| {
            claim.source.url.as_deref() == Some(attempt.locator.as_str())
                && attempt.source_sha256.as_deref() == Some(claim.source_sha256.as_str())
                && attempt.acquired_at == claim.acquired_at
        });
        if outcome != Outcome::CompletedClaims {
            return Err(SchoolContactError::ResearchIncomplete(outcome.as_str()));
        }
    }
    Ok(selected)
}

fn select(
    school: &CanonicalSchool,
    purpose: SchoolMailboxPurpose,
    year: SchoolYear,
) -> Result<Option<&SchoolMailboxClaim>, SchoolContactError> {
    let (selected, stale) = school
        .mailbox_claims
        .iter()
        .filter(|claim| claim.purpose == purpose && claim.school_year == year)
        .try_fold(
            (None, false),
            |(selected, stale): (Option<(&SchoolMailboxClaim, CaptureKey<'_>)>, bool), claim| {
                let at = claim.validated_at()?;
                if claim.school != school.id {
                    return Err(SchoolContactError::ForeignSchool);
                }
                if !captured_in(&at, year) {
                    return Ok((selected, true));
                }
                let key = (
                    at,
                    claim.source.url.as_deref(),
                    claim.source_sha256.as_str(),
                );
                let next = match selected {
                    Some((previous, _))
                        if !previous
                            .mailbox
                            .trim()
                            .eq_ignore_ascii_case(claim.mailbox.trim()) =>
                    {
                        return Err(SchoolContactError::Conflict)
                    }
                    Some((previous, previous_key)) if previous_key >= key => {
                        Some((previous, previous_key))
                    }
                    _ => Some((claim, key)),
                };
                Ok((next, stale))
            },
        )?;
    match (selected, stale) {
        (Some((claim, _)), _) => Ok(Some(claim)),
        (None, true) => Err(SchoolContactError::ResearchIncomplete("stale")),
        (None, false) => Ok(None),
    }
}

type CaptureKey<'a> = (
    chrono::DateTime<chrono::FixedOffset>,
    Option<&'a str>,
    &'a str,
);

fn captured_in(at: &chrono::DateTime<chrono::FixedOffset>, year: SchoolYear) -> bool {
    let Ok(capture_year) = i16::try_from(chrono::Datelike::year(at)) else {
        return false;
    };
    let Ok(month) = u8::try_from(chrono::Datelike::month(at)) else {
        return false;
    };
    SchoolYear::containing(capture_year, month) == Some(year)
}

pub fn research(
    school: &CanonicalSchool,
    program: &CoachContactProgram,
    year: SchoolYear,
) -> Outcome {
    assess(school, &Subject::Program(program.clone()), year, |_| true)
}

pub fn mailbox_research(
    school: &CanonicalSchool,
    purpose: SchoolMailboxPurpose,
    year: SchoolYear,
) -> Outcome {
    assess(school, &Subject::SchoolMailbox(purpose), year, |_| true)
}

pub fn source_research(
    school: &CanonicalSchool,
    program: &CoachContactProgram,
    year: SchoolYear,
    locator: &str,
) -> Outcome {
    assess(
        school,
        &Subject::Program(program.clone()),
        year,
        |attempt| attempt.locator == locator,
    )
}

fn assess(
    school: &CanonicalSchool,
    subject: &Subject,
    year: SchoolYear,
    matches: impl Fn(&ContactResearchAttempt) -> bool,
) -> Outcome {
    let mut attempts = BTreeMap::new();
    let mut state = Outcome::CompletedEmpty;
    for row in school
        .contact_research
        .iter()
        .filter(|row| &row.subject == subject && row.school_year == year)
    {
        if row.school != school.id {
            return Outcome::Conflict;
        }
        if row.attempts.is_empty() && row.outcome != Outcome::Unattempted {
            state = state.combine(Outcome::Partial);
        }
        for attempt in row.attempts.iter().filter(|attempt| matches(attempt)) {
            retain(&mut attempts, attempt, &row.outcome);
        }
    }
    if attempts.is_empty() && state == Outcome::CompletedEmpty {
        return if school.contact_research.iter().any(|row| {
            &row.subject == subject && row.school_year < year && row.attempts.iter().any(&matches)
        }) {
            Outcome::Stale
        } else {
            Outcome::Unattempted
        };
    }
    let admitted_empty = attempts
        .values()
        .any(|(_, _, outcome, tied)| !tied && *outcome == Outcome::CompletedEmpty);
    for (_, (_, _, outcome, tied)) in attempts {
        state = state.combine(if tied { Outcome::Conflict } else { outcome });
    }
    if admitted_empty && state == Outcome::Exhausted {
        Outcome::CompletedEmpty
    } else {
        state
    }
}

type Timestamp = Option<chrono::DateTime<chrono::FixedOffset>>;
type Attempts<'a> = BTreeMap<&'a str, (&'a ContactResearchAttempt, Timestamp, Outcome, bool)>;

fn retain<'a>(attempts: &mut Attempts<'a>, next: &'a ContactResearchAttempt, summary: &Outcome) {
    let at = chrono::DateTime::parse_from_rfc3339(&next.acquired_at).ok();
    let invalid = (at.is_none() && next.outcome != Outcome::Failed)
        || next.locator.trim().is_empty()
        || !super::boundary::attempt(next);
    let outcome = match (summary, &next.outcome) {
        (Outcome::Exhausted, Outcome::CompletedEmpty | Outcome::CompletedClaims) => {
            next.outcome.clone()
        }
        _ => next.outcome.clone().combine(summary.clone()),
    };
    let slot =
        attempts
            .entry(next.locator.as_str())
            .or_insert((next, at, outcome.clone(), invalid));
    match at.cmp(&slot.1) {
        std::cmp::Ordering::Greater if slot.1.is_none() && slot.2 == Outcome::Failed => {
            *slot = (next, at, outcome.combine(Outcome::Failed), invalid);
        }
        std::cmp::Ordering::Greater => *slot = (next, at, outcome, invalid),
        std::cmp::Ordering::Equal => slot.3 |= invalid || outcome != slot.2,
        std::cmp::Ordering::Less if at.is_none() && outcome == Outcome::Failed => {
            slot.2 = slot.2.clone().combine(outcome)
        }
        std::cmp::Ordering::Less if at.is_none() => slot.3 = true,
        _ => {}
    }
}
