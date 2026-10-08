use super::*;
use census_domain::model::{CompetitionLevel, Sport};
use census_domain::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const TRACK_2026: &str = include_str!("../../tests/fixtures/wayzata/track_2026_schedule.html");
const XC_2026: &str = include_str!("../../tests/fixtures/wayzata/xc_2026_schedule.html");
const OBSERVED_ON: &str = "2026-09-20";

mod scenarios;

#[test]
fn schedule_rows_read_the_track_table_with_its_month_heading_and_provider_slug() -> TestResult {
    let rows = schedule_rows(TRACK_2026, 2026)?;
    check!(eq; rows.len(), 13, "one row per competition day in the excerpt");
    let first = &rows[0];
    check!(eq; first.date, "2026-01-04");
    check!(eq; first.name, "USATF Minnesota All-Comers Meet #3");
    check!(eq; first.location, "University of Minnesota");
    check!(eq; first.slug.as_deref(), Some("7vqvs7"));
    Ok(())
}

#[test]
fn schedule_rows_read_the_cross_country_table() -> TestResult {
    let rows = schedule_rows(XC_2026, 2026)?;
    check!(eq; rows.len(), 10, "one row per competition day in the excerpt");
    check!(eq; rows[0].date, "2026-08-27");
    check!(eq; rows[0].name, "River Falls Extreme Meet");
    check!(eq; rows[0].location, "UW-River Falls");
    check!(eq; rows[0].slug.as_deref(), Some("v70kmd"));
    Ok(())
}

#[test]
fn venue_state_only_answers_for_unambiguous_venues() {
    assert_eq!(
        venue_state("University of Minnesota"),
        Some(UsJurisdiction::Minnesota)
    );
    assert_eq!(venue_state("Wartburg College"), Some(UsJurisdiction::Iowa));
    assert_eq!(
        venue_state("UW-River Falls"),
        Some(UsJurisdiction::Wisconsin)
    );
    assert_eq!(venue_state("St. Croix Falls HS"), None);
    assert_eq!(
        venue_state("Augustana College"),
        None,
        "exists in two states"
    );
}

#[test]
fn level_of_reads_the_round_out_of_the_meet_name() {
    assert_eq!(
        level_of("WIAA State Championships"),
        CompetitionLevel::State
    );
    assert_eq!(level_of("D1 Sectional 4"), CompetitionLevel::Sectional);
    assert_eq!(level_of("Regional Final"), CompetitionLevel::Regional);
    assert_eq!(
        level_of("Mississippi Valley Conference"),
        CompetitionLevel::Conference
    );
    assert_eq!(
        level_of("Ron Kretsch Invitational"),
        CompetitionLevel::Invitational
    );
    assert_eq!(
        level_of("Milaca Early Bird Invite"),
        CompetitionLevel::Invitational
    );
    assert_eq!(level_of("Zzz"), CompetitionLevel::Unknown);
}

#[test]
fn a_track_row_lands_in_the_season_its_month_belongs_to() {
    assert_eq!(ScheduleSport::Track.sport_for(1), Sport::IndoorTrack);
    assert_eq!(ScheduleSport::Track.sport_for(3), Sport::IndoorTrack);
    assert_eq!(ScheduleSport::Track.sport_for(12), Sport::IndoorTrack);
    assert_eq!(ScheduleSport::Track.sport_for(5), Sport::OutdoorTrack);
    assert_eq!(
        ScheduleSport::CrossCountry.sport_for(9),
        Sport::CrossCountry
    );
}

fn seed_cache(cache_dir: &std::path::Path, url: &str, body: &str) -> TestResult {
    seed_cache_at(cache_dir, url, body, "2026-09-20T14:39:00Z")
}

fn seed_cache_at(
    cache_dir: &std::path::Path,
    url: &str,
    body: &str,
    fetched_at: &str,
) -> TestResult {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"GET");
    hasher.update([0x1f]);
    hasher.update(url.as_bytes());
    hasher.update([0x1f]);
    let key: String = hasher.finalize()[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let meta = json!({
        "url": url,
        "method": "GET",
        "status": 200,
        "content_digest": format!("{:x}", Sha256::digest(body.as_bytes())),
        "bytes": body.len(),
        "fetched_at": fetched_at,
    });
    std::fs::write(
        cache_dir.join(format!("{key}.meta.json")),
        serde_json::to_string(&meta)?,
    )?;
    std::fs::write(cache_dir.join(format!("{key}.body")), body)?;
    Ok(())
}
