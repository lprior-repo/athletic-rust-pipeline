use super::{CanonicalSchool, FetchOutcome, Outcome, SchoolYear, Subject};

pub(super) fn capture_outcome(
    school: &CanonicalSchool,
    year: SchoolYear,
    capture: &FetchOutcome,
    locator: &str,
) -> (Outcome, String) {
    school
        .contact_research
        .iter()
        .filter(|row| matches!(row.subject, Subject::SchoolMailbox(_)) && row.school_year == year)
        .flat_map(|row| &row.attempts)
        .filter(|attempt| {
            attempt.locator == locator
                && attempt.acquired_at == capture.fetched_at
                && attempt.source_sha256.as_deref() == Some(capture.content_digest.as_str())
        })
        .fold(
            (Outcome::CompletedEmpty, String::new()),
            |(state, reason), attempt| {
                let combined = state.clone().combine(attempt.outcome.clone());
                let reason = if combined != state {
                    attempt.reason.clone()
                } else {
                    reason
                };
                (combined, reason)
            },
        )
}
