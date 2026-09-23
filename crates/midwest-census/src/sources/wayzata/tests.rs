use super::*;
use crate::school_index::SchoolIndex;
use census_domain::model::{CompetitionLevel, Sport};
use census_domain::UsJurisdiction;
use std::collections::HashMap;

const TRACK_2026: &str = include_str!("../../../tests/fixtures/wayzata/track_2026_schedule.html");
const XC_2026: &str = include_str!("../../../tests/fixtures/wayzata/xc_2026_schedule.html");
const OBSERVED_ON: &str = "2026-09-20";

#[test]
fn schedule_rows_read_the_track_table_with_its_month_heading_and_provider_slug() {
    let rows = schedule_rows(TRACK_2026, 2026).expect("the track schedule parses");
    assert_eq!(rows.len(), 13, "one row per competition day in the excerpt");
    let first = &rows[0];
    assert_eq!(first.date, "2026-01-04");
    assert_eq!(first.name, "USATF Minnesota All-Comers Meet #3");
    assert_eq!(first.location, "University of Minnesota");
    assert_eq!(first.slug.as_deref(), Some("7vqvs7"));
    assert!(
        first
            .aria_label
            .as_deref()
            .is_some_and(|label| label.contains("USATF Minnesota All-Comers Meet #3")),
        "the link's own label repeats the event: {:?}",
        first.aria_label
    );
    assert!(
        rows.windows(2).all(|pair| pair[0].date <= pair[1].date),
        "the schedule is published in date order"
    );
}

#[test]
fn schedule_rows_read_the_cross_country_table() {
    let rows = schedule_rows(XC_2026, 2026).expect("the cross-country schedule parses");
    assert_eq!(rows.len(), 10, "one row per competition day in the excerpt");
    assert_eq!(rows[0].date, "2026-08-27");
    assert_eq!(rows[0].name, "River Falls Extreme Meet");
    assert_eq!(rows[0].location, "UW-River Falls");
    assert_eq!(rows[0].slug.as_deref(), Some("v70kmd"));
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

fn seed_cache(cache_dir: &std::path::Path, url: &str, body: &str) {
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
        "sha256": format!("{:x}", Sha256::digest(body.as_bytes())),
        "bytes": body.len(),
        "fetched_at": "2026-09-20T14:39:00Z",
    });
    std::fs::write(
        cache_dir.join(format!("{key}.meta.json")),
        serde_json::to_string(&meta).expect("meta json"),
    )
    .expect("write meta");
    std::fs::write(cache_dir.join(format!("{key}.body")), body).expect("write body");
}

#[tokio::test]
async fn collect_mints_core_meets_from_the_provider_schedule_without_touching_the_links() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cache = dir.path().join("http");
    std::fs::create_dir_all(&cache).expect("cache dir");
    seed_cache(
        &cache,
        &schedule_url(ScheduleSport::Track, 2026),
        TRACK_2026,
    );
    seed_cache(
        &cache,
        &schedule_url(ScheduleSport::CrossCountry, 2026),
        XC_2026,
    );

    let store = census_store::Store::open(dir.path().join("store")).expect("store");
    let fetcher = crate::net::Fetcher::new(
        &cache,
        None,
        std::time::Duration::from_millis(1),
        std::collections::HashMap::new(),
        Vec::new(),
    )
    .expect("fetcher");
    let ctx = AdapterContext {
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: census_domain::model::SchoolYear::new(2026).expect("2026 is a season"),
        observed_on: OBSERVED_ON.to_string(),
    };
    let options = Options {
        years: vec![2026],
        limit: None,
        refresh: false,
        observed_on: Some(OBSERVED_ON.to_string()),
    };
    let report = collect(&ctx, &options)
        .await
        .expect("collect returns a report");

    assert_eq!(report.rows, 23, "13 track rows plus 10 cross-country rows");
    assert_eq!(report.errors, 0);
    assert_eq!(
        report.requests, 0,
        "both schedules came from the seeded cache"
    );
    assert_eq!(report.from_cache, 2);
    assert!(
        report
            .notes
            .iter()
            .any(|note| note.contains("venue state resolution") && note.contains("sites=12")),
        "the runner states its own venue resolution: {:?}",
        report.notes
    );

    // The meets are in the store with core evidence and the provider's own key.
    let appended: Vec<CanonicalMeet> = store.scan(Table::Meets).expect("scan meets from Fjall");
    assert!(
        appended.len() >= 20,
        "most rows mint a distinct meet; {} were appended",
        appended.len()
    );

    let opener = appended
        .iter()
        .find(|meet| meet.name == "USATF Minnesota All-Comers Meet #3")
        .expect("the first track row is a meet");
    assert_eq!(opener.state, Some(UsJurisdiction::Minnesota));
    assert_eq!(opener.date, "2026-01-04");
    assert_eq!(opener.location.as_deref(), Some("University of Minnesota"));
    assert_eq!(opener.sports, vec![Sport::IndoorTrack]);
    assert_eq!(opener.level, CompetitionLevel::Invitational);
    assert!(
        opener
            .source_urls
            .iter()
            .any(|url| url.ends_with("/links/7vqvs7")),
        "the provider link is retained as a key, not fetched: {:?}",
        opener.source_urls
    );
    assert!(
        opener
            .evidence
            .iter()
            .all(|evidence| evidence.source.id == ADAPTER_ID),
        "core evidence only: {:?}",
        opener.evidence
    );
    assert_eq!(
        opener.source_identities,
        vec![SourceIdentity::new(
            SourceNamespace::TimerMeet {
                provider: PROVIDER.to_string(),
            },
            "7vqvs7",
        )]
    );

    let xc = appended
        .iter()
        .find(|meet| meet.name == "River Falls Extreme Meet")
        .expect("the first cross-country row is a meet");
    assert_eq!(xc.state, Some(UsJurisdiction::Wisconsin));
    assert_eq!(xc.date, "2026-08-27");
    assert_eq!(xc.sports, vec![Sport::CrossCountry]);

    // A venue the table does not claim stays unplaced: the report spells that bucket
    // `MEET_STATE_UNRESOLVED`, so the meet must carry no jurisdiction at all.
    assert!(
        appended.iter().any(|meet| meet.state.is_none()),
        "the fixture carries venues the table does not claim"
    );
    assert!(
        appended
            .iter()
            .filter(|meet| meet.state.is_none())
            .all(|meet| meet.state != Some(UsJurisdiction::Minnesota)),
        "unresolved venues are never filed under the provider's home state"
    );
}

#[tokio::test]
async fn a_journaled_schedule_is_skipped_on_the_next_run() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cache = dir.path().join("http");
    std::fs::create_dir_all(&cache).expect("cache dir");
    seed_cache(
        &cache,
        &schedule_url(ScheduleSport::Track, 2026),
        TRACK_2026,
    );
    seed_cache(
        &cache,
        &schedule_url(ScheduleSport::CrossCountry, 2026),
        XC_2026,
    );
    let store = census_store::Store::open(dir.path().join("store")).expect("store");
    let fetcher = crate::net::Fetcher::new(
        &cache,
        None,
        std::time::Duration::from_millis(1),
        std::collections::HashMap::new(),
        Vec::new(),
    )
    .expect("fetcher");
    let ctx = AdapterContext {
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: census_domain::model::SchoolYear::new(2026).expect("2026 is a season"),
        observed_on: OBSERVED_ON.to_string(),
    };
    let options = Options {
        years: vec![2026],
        limit: None,
        refresh: false,
        observed_on: Some(OBSERVED_ON.to_string()),
    };
    collect(&ctx, &options).await.expect("first run");
    let second = collect(&ctx, &options).await.expect("second run");
    assert_eq!(second.rows, 0, "both schedules are already journaled");
    assert_eq!(second.from_cache, 0, "and are not even read again");
}

#[test]
fn a_school_shaped_venue_is_read_with_its_suffix_written_out() {
    assert_eq!(
        venue_candidates("Albany HS"),
        vec!["Albany HS", "Albany High School"]
    );
    assert_eq!(
        venue_candidates("St. Croix Falls H.S."),
        vec!["St. Croix Falls H.S.", "St. Croix Falls High School"]
    );
    assert_eq!(venue_candidates("Blake School"), vec!["Blake School"]);
    assert_eq!(
        venue_candidates("Bassett Creek Park"),
        vec!["Bassett Creek Park"]
    );
    assert_eq!(venue_candidates(" HS"), vec!["HS"]);
}

#[test]
fn a_school_venue_resolves_only_where_exactly_one_state_owns_it() {
    // Canonical schools carry a normalized name, exactly as the store writes them.
    let school = |state: UsJurisdiction, name: &str| {
        census_domain::model::CanonicalSchool::new(
            state,
            name,
            census_domain::model::normalize_name(name),
        )
        .0
    };
    let mut cache = HashMap::new();

    // The same school name in two states is never guessed at.
    let both = SchoolIndex::from_schools(&[
        school(UsJurisdiction::Minnesota, "Albany High School"),
        school(UsJurisdiction::Wisconsin, "Albany High School"),
    ]);
    assert_eq!(
        resolve_venue(&both, &mut cache, "Albany HS"),
        VenueResolution::Unknown
    );

    // With one owner it resolves, including for the punctuated spelling a timer may print.
    let one = SchoolIndex::from_schools(&[
        school(UsJurisdiction::Minnesota, "Albany High School"),
        school(UsJurisdiction::Wisconsin, "River Falls High School"),
    ]);
    let mut cache = HashMap::new();
    assert_eq!(
        resolve_venue(&one, &mut cache, "Albany HS"),
        VenueResolution::School(UsJurisdiction::Minnesota)
    );
    assert_eq!(
        resolve_venue(&one, &mut cache, "Albany H.S."),
        VenueResolution::School(UsJurisdiction::Minnesota)
    );
    assert_eq!(
        resolve_venue(&one, &mut cache, "River Falls HS"),
        VenueResolution::School(UsJurisdiction::Wisconsin)
    );
    assert_eq!(
        resolve_venue(&one, &mut cache, "Bassett Creek Park"),
        VenueResolution::Unknown
    );
    assert_eq!(
        resolve_venue(&one, &mut cache, "University of Minnesota"),
        VenueResolution::Site(UsJurisdiction::Minnesota)
    )
}
