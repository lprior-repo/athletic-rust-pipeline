use super::super::{capture::CapturedRoster, invalid};
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Evidence, QueryProfileV1, SourceNamespace, Sport,
};
use census_domain::roster_affiliation_authority::RosterMemberObservationV1;
use census_store::StoreResult;

pub(super) fn validate_member<'a>(
    athlete: &CanonicalAthlete,
    note: &RosterMemberObservationV1,
    capture: &'a CapturedRoster,
) -> StoreResult<&'a census_crawl::milesplit::LocatedRosterAthlete> {
    let row = note.row();
    let member = capture
        .members
        .get(&row.ordinal())
        .filter(|member| {
            let located = member.locator();
            located.ordinal == row.ordinal()
                && located.byte_offset == row.byte_offset()
                && located.byte_length == row.byte_length()
        })
        .ok_or_else(|| invalid("retained roster locator does not identify an admitted row"))?;
    let source = member.athlete();
    if capture.hashes.get(&row.ordinal()).map(String::as_str) != Some(note.row_sha256())
        || source.athlete_id != note.provider_athlete_id()
        || source.name != athlete.canonical_name
        || source.grad_year != athlete.grad_year
        || source.gender != athlete.gender
    {
        return Err(invalid(
            "retained roster row does not reproduce its athlete observation",
        ));
    }
    Ok(member)
}

pub(super) fn selected_sport(
    member: &census_crawl::milesplit::LocatedRosterAthlete,
    profile: &QueryProfileV1,
) -> bool {
    let source = member.athlete();
    profile.input().sports.iter().any(|sport| match sport {
        Sport::CrossCountry => source.xc,
        Sport::IndoorTrack => source.indoor,
        Sport::OutdoorTrack => source.outdoor,
        Sport::Unknown => false,
    })
}

pub(super) fn validate_source(
    athlete: &CanonicalAthlete,
    evidence: &Evidence,
    school: &CanonicalSchool,
) -> StoreResult<()> {
    let source = athlete
        .source
        .as_ref()
        .ok_or_else(|| invalid("roster subject has no source"))?;
    let state = school
        .state
        .ok_or_else(|| invalid("roster school has no source jurisdiction"))?;
    if evidence.source.id != census_crawl::milesplit::Site::for_jurisdiction(state).source_id()
        || athlete.id
            != CanonicalAthlete::mint(
                &school.id,
                &athlete.canonical_name,
                athlete.grad_year,
                athlete.gender,
                source,
            )
    {
        return Err(invalid(
            "retained roster source or physical subject identity changed",
        ));
    }
    Ok(())
}

pub(super) fn validate_owner(
    athlete: &CanonicalAthlete,
    note: &RosterMemberObservationV1,
) -> StoreResult<()> {
    if !athlete.source.as_ref().is_some_and(|source| {
        source.namespace == SourceNamespace::MilesplitAthlete
            && source.id == note.provider_athlete_id()
    }) {
        return Err(invalid(
            "retained roster row changes athlete provider ownership",
        ));
    }
    Ok(())
}
