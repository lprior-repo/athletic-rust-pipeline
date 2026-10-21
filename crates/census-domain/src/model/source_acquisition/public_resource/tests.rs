use super::*;
use crate::UsJurisdiction;

mod directory;
mod origins;
mod targets;
mod wire;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn year() -> TestResult<SchoolYear> {
    SchoolYear::new(2026).ok_or_else(|| "fixture school year".into())
}

fn resources() -> TestResult<Vec<(PublicSourceResourceV1, &'static str)>> {
    Ok(vec![
        (
            PublicSourceResourceV1::milesplit_athlete_profile("13749840")?,
            "https://mi.milesplit.com/athletes/13749840-brennan-smith",
        ),
        (
            PublicSourceResourceV1::milesplit_team_roster("12439")?,
            "https://mi.milesplit.com/teams/12439-detroit-catholic-central/roster",
        ),
        (
            PublicSourceResourceV1::milesplit_meet_team("760382", "12439")?,
            "https://mi.milesplit.com/meets/760382-chsl-championships-2026/teams/12439",
        ),
        (
            PublicSourceResourceV1::milesplit_raw_results("760382", "1300204")?,
            "https://mi.milesplit.com/meets/760382-chsl-championships-2026/results/1300204/raw",
        ),
        (
            PublicSourceResourceV1::milesplit_team_meet_history("12439")?,
            "https://mi.milesplit.com/teams/12439-detroit-catholic-central/meets",
        ),
        (
            PublicSourceResourceV1::school_calendar(
                "12439",
                "https://www.example.org",
                Sport::OutdoorTrack,
                year()?,
            )?,
            "https://www.example.org/sports/track-and-field/schedule/2026-27",
        ),
        (
            PublicSourceResourceV1::robots("https://www.example.org")?,
            "https://www.example.org/robots.txt",
        ),
    ])
}

#[test]
fn all_closed_resource_kinds_roundtrip_without_target_or_retry_identity() -> TestResult {
    let spellings = [
        "mile_split_athlete_profile",
        "mile_split_team_roster",
        "mile_split_meet_team",
        "mile_split_raw_results",
        "mile_split_team_meet_history",
        "school_calendar",
        "robots",
    ];
    for ((resource, target), spelling) in resources()?.into_iter().zip(spellings) {
        resource.validate_target(UsJurisdiction::Michigan, target)?;
        let bytes = serde_json::to_vec(&resource)?;
        let cold: PublicSourceResourceV1 = serde_json::from_slice(&bytes)?;
        check!(eq; cold, resource);
        check!(eq; serde_json::to_vec(&cold)?, bytes);
        check!(eq; cold.kind(), resource.kind());
        let value = serde_json::to_value(&cold)?;
        check!(eq; value.get("kind"), Some(&serde_json::json!(spelling)));
        for absent in ["target_url", "slug", "query", "plan", "retry", "version"] {
            check!(value.get(absent).is_none());
        }
    }
    Ok(())
}

#[test]
fn constructors_expose_only_owned_identifiers_and_calendar_scope() -> TestResult {
    let profile = PublicSourceResourceV1::milesplit_athlete_profile("13711713")?;
    check!(eq; profile.athlete_id(), Some("13711713"));
    check!(profile.team_id().is_none());
    check!(profile.origin().is_none());
    let meet_team = PublicSourceResourceV1::milesplit_meet_team("760382", "12684")?;
    check!(eq; meet_team.meet_id(), Some("760382"));
    check!(eq; meet_team.team_id(), Some("12684"));
    check!(meet_team.result_id().is_none());
    let raw = PublicSourceResourceV1::milesplit_raw_results("760382", "1300204")?;
    check!(eq; raw.result_id(), Some("1300204"));
    let calendar = PublicSourceResourceV1::school_calendar(
        "12684",
        "https://www.example.org",
        Sport::CrossCountry,
        year()?,
    )?;
    check!(eq; calendar.provider_school_id(), Some("12684"));
    check!(eq; calendar.origin(), Some("https://www.example.org"));
    check!(eq; calendar.sport(), Some(Sport::CrossCountry));
    check!(eq; calendar.school_year(), Some(year()?));
    Ok(())
}
