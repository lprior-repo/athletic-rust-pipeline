use super::super::context::{DatedRosterMembership, ObservationDayConvention, RosterEvidence};
use super::{invalid, Gap, RowNote};
use census_crawl::milesplit::LocatedRosterAthlete;
use census_domain::model::{AthleteId, Evidence, QueryProfileV1};
use census_store::StoreResult;

pub(super) enum Outcome {
    Current,
    Unavailable(Option<Gap>),
}

pub(super) fn retain(
    context: &mut RosterEvidence,
    athlete: &AthleteId,
    evidence: &Evidence,
    note: &RowNote,
    member: &LocatedRosterAthlete,
    profile: &QueryProfileV1,
) -> StoreResult<Outcome> {
    if let Some(assertion) = member.current_roster() {
        context.retain_dated(
            athlete,
            evidence,
            DatedRosterMembership {
                observed_on: profile.input().as_of.clone(),
                observed_at: note.fetched_at.clone(),
                day_convention: ObservationDayConvention::UtcCaptureDay,
                assertion: assertion.clone(),
                capture_sha256: note.capture_sha256.clone(),
            },
        )?;
    }
    let Some(dated) = context.dated_membership(athlete, evidence)? else {
        return Ok(Outcome::Unavailable(
            member.temporal_unavailability().and_then(diagnostic),
        ));
    };
    let assertion = &dated.assertion;
    if dated.observed_on != profile.input().as_of
        || dated.observed_at != note.fetched_at
        || dated.day_convention != ObservationDayConvention::UtcCaptureDay
        || dated.capture_sha256 != note.capture_sha256
        || assertion.owner() != note.provider_team_id
        || assertion.member() != note.provider_athlete_id
        || assertion.row() != note.row
        || !profile
            .input()
            .sports
            .iter()
            .any(|sport| assertion.supports(*sport))
    {
        return Err(invalid(
            "current roster assertion differs from verified member ownership",
        ));
    }
    Ok(Outcome::Current)
}

fn diagnostic(reason: census_crawl::milesplit::RosterTemporalUnavailable) -> Option<Gap> {
    use census_crawl::milesplit::RosterTemporalUnavailable as Reason;
    match reason {
        Reason::MissingStatement => None,
        Reason::AmbiguousStatement => Some(Gap::TemporalStatementAmbiguous),
        Reason::MalformedMarkup => Some(Gap::TemporalMarkupMalformed),
        Reason::ResourceLimit => Some(Gap::TemporalResourceLimited),
        Reason::InactiveMember => Some(Gap::TemporalMemberInactive),
    }
}
