use super::collect::Options;
use super::map::{map_coach_row, map_org_school, map_primary_contact};
use super::parse::{credentials_in_bundle, parse_coach_rows, parse_org_schools, parse_token};
use super::{covered_states, org_for};
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Gender, SchoolId, SchoolYear, Sport,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::Duration;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
type SchoolFields = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);
type SchoolMap = BTreeMap<String, SchoolFields>;

use crate::net::Fetcher;

const NH_CHILDREN: &str = include_str!("../../tests/fixtures/arbiter/nh_children_p1.body");
const ALVIRNE_COACHES: &str = include_str!("../../tests/fixtures/arbiter/alvirne_coaches_p1.body");
const NH_ALL_COACHES_P11: &str =
    include_str!("../../tests/fixtures/arbiter/nh_all_coaches_p11.body");
const EMPTY_COACHES: &str = include_str!("../../tests/fixtures/arbiter/coaches_empty.body");
const NOT_JSON: &str = include_str!("../../tests/fixtures/arbiter/not_json.body");
const BUNDLE: &str = include_str!("../../tests/fixtures/arbiter/bundle_credentials.derived.js");
const TOKEN: &str = include_str!("../../tests/fixtures/arbiter/token_response.derived.json");
const SYNTHETIC_ENTRY: &str =
    "<!doctype html><script type=\"module\" src=\"assets/index-synthetic-redeploy-20261002.js\"></script>";
const SYNTHETIC_BUNDLE_URL: &str =
    "https://live.arbiter.io/directory/assets/index-synthetic-redeploy-20261002.js";

const GOLDEN_SCHOOLS: &str = include_str!("../../tests/fixtures/arbiter/golden_nh_schools.json");
const GOLDEN_CONTACTS: &str = include_str!("../../tests/fixtures/arbiter/golden_nh_contacts.json");
const GOLDEN_ALVIRNE: &str =
    include_str!("../../tests/fixtures/arbiter/golden_alvirne_coaches.json");
const GOLDEN_P11: &str =
    include_str!("../../tests/fixtures/arbiter/golden_nh_all_coaches_p11.json");

const CHILDREN_URL: &str =
    "https://services.arbitersports.com/api/v2/organization/public/2132/children?&pageSize=200&pageNumber=1";
const ALVIRNE_URL: &str = "https://services.arbitersports.com/api/v2/legacy/public/2132/coaches?filter.EntityId=450&&pageSize=200&pageNumber=1";
const ORGANISATION_URL: &str =
    "https://services.arbitersports.com/api/v2/organization/public/2132/children";
const OBSERVED_ON: &str = "2026-09-27";

fn golden_rows(json: &str) -> TestResult<Vec<serde_json::Value>> {
    Ok(serde_json::from_str(json)?)
}

fn field(row: &serde_json::Value, key: &str) -> Option<String> {
    row.get(key)
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .filter(|value| !value.is_empty())
}

fn alvirne_id() -> SchoolId {
    CanonicalSchool::mint(
        UsJurisdiction::NewHampshire,
        "Alvirne High School",
        &normalize_name("Alvirne High School"),
    )
}

fn mapped_coaches(
    body: &str,
    url: &str,
    school: &str,
) -> TestResult<(Vec<CanonicalCoach>, CanonicalSchool)> {
    let parsed = parse_org_schools(NH_CHILDREN, CHILDREN_URL)?;
    let row = parsed
        .rows
        .iter()
        .find(|row| row.name == school)
        .ok_or("the school is in the children page")?;
    let (school_row, id) = map_org_school(
        row,
        UsJurisdiction::NewHampshire,
        ORGANISATION_URL,
        OBSERVED_ON,
    )
    .ok_or("the school maps")?;
    let coaches = parse_coach_rows(body, url)?
        .rows
        .iter()
        .filter_map(|coach| map_coach_row(coach, &id, url, OBSERVED_ON))
        .collect();
    Ok((coaches, school_row))
}

#[test]
fn the_schools_page_carries_all_89_nhiaa_schools() -> TestResult {
    let page = parse_org_schools(NH_CHILDREN, CHILDREN_URL)?;
    check!(eq; page.total, 89);
    check!(eq; page.rows.len(), 89);
    let first = &page.rows[0];
    check!(eq; first.name, "Alvirne High School");
    check!(eq; first.public_id, Some(450));
    check!(eq; first.org_id, Some(29072));
    check!(eq; first.phone.as_deref(), Some("6038861260"));
    check!(eq; first.enrollment, Some(955));
    let contact = first
        .primary_contact
        .as_ref()
        .ok_or("Alvirne has a contact")?;
    check!(eq; contact.first_name, "Justin");
    check!(eq; contact.last_name, "Hufft");
    check!(eq; contact.role_name, "Athletic Director");
    Ok(())
}

#[test]
fn every_school_row_matches_the_prototype_golden_field_for_field() -> TestResult {
    let page = parse_org_schools(NH_CHILDREN, CHILDREN_URL)?;
    let golden = golden_rows(GOLDEN_SCHOOLS)?;
    check!(eq; page.rows.len(), golden.len());
    let mut rust: SchoolMap = SchoolMap::new();
    for row in &page.rows {
        let (school, _) = map_org_school(
            row,
            UsJurisdiction::NewHampshire,
            ORGANISATION_URL,
            OBSERVED_ON,
        )
        .ok_or("every golden row has a name")?;
        let association_id = school
            .source_identities
            .first()
            .map(|identity| identity.id.clone());
        rust.insert(
            school.name.clone(),
            (
                association_id,
                row.org_id.map(|id| id.to_string()),
                row.phone.clone(),
                school.enrollment.map(|enrollment| enrollment.to_string()),
            ),
        );
    }
    let mut expected: SchoolMap = SchoolMap::new();
    for row in &golden {
        expected.insert(
            field(row, "name").ok_or("golden rows carry a name")?,
            (
                field(row, "association_id"),
                field(row, "arbiter_org_id"),
                field(row, "phone"),
                field(row, "enrollment"),
            ),
        );
    }
    check!(eq; rust, expected, "school fields differ from the prototype");
    Ok(())
}

#[test]
fn every_primary_contact_becomes_an_athletic_director_row() -> TestResult {
    let page = parse_org_schools(NH_CHILDREN, CHILDREN_URL)?;
    let golden = golden_rows(GOLDEN_CONTACTS)?;
    check!(eq;
        golden.len(),
        89,
        "the prototype emits one contact per school"
    );
    let mut rust: BTreeSet<(String, String)> = BTreeSet::new();
    for row in &page.rows {
        let (school, id) = map_org_school(
            row,
            UsJurisdiction::NewHampshire,
            ORGANISATION_URL,
            OBSERVED_ON,
        )
        .ok_or("every golden row has a name")?;
        let Some(contact) = row.primary_contact.as_ref() else {
            continue;
        };
        let Some(coach) = map_primary_contact(contact, &id, ORGANISATION_URL, OBSERVED_ON) else {
            continue;
        };
        check!(eq; coach.role, CoachRole::AthleticDirector);
        check!(eq; coach.sport, None, "a director has no sporting side");
        check!(eq; coach.gender, Gender::Mixed);
        check!(eq; coach.evidence.len(), 1);
        check!(eq; coach.evidence[0].source.id, super::SOURCE_ID);
        rust.insert((school.name.clone(), coach.name.clone()));
    }
    let expected: BTreeSet<(String, String)> = golden
        .iter()
        .map(|row| {
            (
                match field(row, "school") {
                    Some(value) => value,
                    None => Default::default(),
                },
                match field(row, "person") {
                    Some(value) => value,
                    None => Default::default(),
                },
            )
        })
        .collect();
    check!(eq; rust, expected, "contact rows differ from the prototype");
    Ok(())
}

#[test]
fn alvirne_coach_rows_match_the_prototype_on_person_sport_and_role() -> TestResult {
    let golden = golden_rows(GOLDEN_ALVIRNE)?;
    check!(eq; golden.len(), 2, "the capture holds two cross-country rows");
    let rows = parse_coach_rows(ALVIRNE_COACHES, ALVIRNE_URL)?.rows;
    let id = alvirne_id();
    let rust: BTreeSet<(String, String, String)> = rows
        .iter()
        .filter_map(|row| map_coach_row(row, &id, ALVIRNE_URL, OBSERVED_ON))
        .map(|coach| -> TestResult<_> {
            Ok((
                coach.name.clone(),
                coach
                    .sport
                    .ok_or("mapped coaches have a sport")?
                    .stable_key()
                    .to_string(),
                coach.role.stable_key().to_string(),
            ))
        })
        .collect::<TestResult<_>>()?;
    let expected: BTreeSet<(String, String, String)> = golden
        .iter()
        .map(|row| {
            (
                match field(row, "person") {
                    Some(value) => value,
                    None => Default::default(),
                },
                match field(row, "sport") {
                    Some(value) => value,
                    None => Default::default(),
                },
                match field(row, "role") {
                    Some(value) => value,
                    None => Default::default(),
                },
            )
        })
        .collect();
    check!(eq; rust, expected);
    Ok(())
}

#[test]
fn the_side_of_a_coach_comes_from_the_level_label() -> TestResult {
    let (coaches, _) = mapped_coaches(ALVIRNE_COACHES, ALVIRNE_URL, "Alvirne High School")?;
    let by_name: BTreeMap<&str, &CanonicalCoach> = coaches
        .iter()
        .map(|coach| (coach.name.as_str(), coach))
        .collect();
    let demers = by_name.get("Phillip Demers").ok_or("Demers is mapped")?;
    check!(eq; demers.gender, Gender::Boys);
    check!(eq; demers.sport, Some(Sport::CrossCountry));
    let wilson = by_name.get("Kaitlyn Wilson").ok_or("Wilson is mapped")?;
    check!(eq; wilson.gender, Gender::Girls);
    Ok(())
}

#[test]
fn the_prototype_drops_arbiter_track_variants_and_this_lane_keeps_them() -> TestResult {
    let golden = golden_rows(GOLDEN_P11)?;
    check!(
        golden.is_empty(),
        "the prototype's exact-key sport table maps 'Track & Field - Indoor' to nothing"
    );
    let page = parse_coach_rows(NH_ALL_COACHES_P11, ORGANISATION_URL)?;
    check!(eq; page.total, 2512);
    check!(eq; page.rows.len(), 200);
    let id = alvirne_id();
    let mapped: Vec<CanonicalCoach> = page
        .rows
        .iter()
        .filter_map(|row| map_coach_row(row, &id, ORGANISATION_URL, OBSERVED_ON))
        .collect();
    check!(eq; mapped.len(), 121, "121 of 200 rows survive the filters");
    let mut indoor = 0;
    let mut outdoor = 0;
    let mut boys = 0;
    let mut girls = 0;
    let mut mixed = 0;
    let mut head = 0;
    let mut assistant = 0;
    for coach in &mapped {
        match coach.sport {
            Some(Sport::IndoorTrack) => indoor += 1,
            Some(Sport::OutdoorTrack) => outdoor += 1,
            other => return Err(format!("unexpected sport {other:?}").into()),
        }
        match coach.gender {
            Gender::Boys => boys += 1,
            Gender::Girls => girls += 1,
            Gender::Mixed => mixed += 1,
            Gender::Unknown => return Err("the level label always states a side".into()),
        }
        match coach.role {
            CoachRole::HeadCoach => head += 1,
            CoachRole::AssistantCoach => assistant += 1,
            other => return Err(format!("unexpected role {other:?}").into()),
        }
    }
    check!(eq; (indoor, outdoor), (78, 43));
    check!(eq; (boys, girls, mixed), (69, 29, 23));
    check!(eq; (head, assistant), (78, 43));
    Ok(())
}

#[test]
fn sub_varsity_rows_are_dropped() -> TestResult {
    let page = parse_coach_rows(NH_ALL_COACHES_P11, ORGANISATION_URL)?;
    let eligible = page
        .rows
        .iter()
        .filter(|row| {
            let position = row.position.to_ascii_lowercase();
            let person = !row.first_name.is_empty() || !row.last_name.is_empty();
            let sport = row.sport.to_ascii_lowercase();
            person && sport.contains("track") && position.contains("coach")
        })
        .count();
    let id = alvirne_id();
    let mapped = page
        .rows
        .iter()
        .filter_map(|row| map_coach_row(row, &id, ORGANISATION_URL, OBSERVED_ON))
        .count();
    check!(eq; eligible, 137, "the page holds 137 eligible track rows");
    check!(eq;
        eligible - mapped,
        16,
        "the 16 sub-varsity rows (Coed Unified, Coed Middle School, Boys JV) are dropped"
    );
    Ok(())
}

#[test]
fn an_empty_coach_page_yields_no_rows() -> TestResult {
    let page = parse_coach_rows(EMPTY_COACHES, ORGANISATION_URL)?;
    check!(eq; page.total, 0);
    check!(page.rows.is_empty());
    let id = alvirne_id();
    check!(page
        .rows
        .iter()
        .filter_map(|row| map_coach_row(row, &id, ORGANISATION_URL, OBSERVED_ON))
        .next()
        .is_none());
    Ok(())
}

#[test]
fn a_non_json_body_is_a_completed_error_outcome() {
    assert!(parse_org_schools(NOT_JSON, CHILDREN_URL).is_err());
    assert!(parse_coach_rows(NOT_JSON, ALVIRNE_URL).is_err());
    assert!(parse_token(NOT_JSON, super::TOKEN_URL).is_err());
}

#[test]
fn the_published_bundle_yields_the_client_credentials() -> TestResult {
    let (client_id, client_secret) =
        credentials_in_bundle(BUNDLE).ok_or("the derived bundle slice carries the pair")?;
    check!(eq; client_id, "CLIENT-ID-REDACTED");
    check!(eq; client_secret, "CLIENT-SECRET-REDACTED");
    check!(credentials_in_bundle("<html>nothing here</html>").is_none());
    check!(credentials_in_bundle("client_id:\"\",client_secret:\"x\"").is_none());
    check!(credentials_in_bundle("client_id:\"x\",client_secret:\"\"").is_none());
    Ok(())
}

#[test]
fn the_token_response_yields_the_access_token() -> TestResult {
    let token = parse_token(TOKEN, super::TOKEN_URL)?;
    check!(eq; token, "ACCESS-TOKEN-REDACTED");
    check!(parse_token("{\"expires_in\":86400}", super::TOKEN_URL).is_err());
    Ok(())
}

#[test]
fn the_lane_covers_the_four_arbiter_association_orgs() {
    assert_eq!(org_for(UsJurisdiction::NewHampshire), Some("2132"));
    assert_eq!(org_for(UsJurisdiction::Kentucky), Some("2507"));
    assert_eq!(org_for(UsJurisdiction::Montana), Some("4497"));
    assert_eq!(org_for(UsJurisdiction::WestVirginia), Some("4223"));
    assert_eq!(org_for(UsJurisdiction::Delaware), None);
    let covered: Vec<UsJurisdiction> = covered_states().collect();
    assert_eq!(covered.len(), 4);
    assert!(covered.contains(&UsJurisdiction::NewHampshire));
}

#[test]
fn only_identifiable_coach_roles_are_mapped() -> TestResult {
    let id = alvirne_id();
    let row = super::parse::CoachRow {
        first_name: "Sam".to_string(),
        last_name: "Lee".to_string(),
        position: "Assistant Coach".to_string(),
        sport: "Cross Country, Boys".to_string(),
        level: "Boys Varsity".to_string(),
    };
    let coach = map_coach_row(&row, &id, ALVIRNE_URL, OBSERVED_ON).ok_or("assistant coach maps")?;
    check!(eq; coach.role, CoachRole::AssistantCoach);
    check!(eq; coach.gender, Gender::Boys);
    for position in ["Volunteer Coach", "Secretary", "", "Team Manager"] {
        let row = super::parse::CoachRow {
            first_name: "Sam".to_string(),
            last_name: "Lee".to_string(),
            position: position.to_string(),
            sport: "Cross Country".to_string(),
            level: "Varsity".to_string(),
        };
        check!(
            map_coach_row(&row, &id, ALVIRNE_URL, OBSERVED_ON).is_none(),
            "{position:?} is not an identifiable coach role"
        );
    }
    Ok(())
}

#[test]
fn only_cross_country_and_track_sports_are_mapped() -> TestResult {
    let id = alvirne_id();
    let cases = [
        (
            "Cross Country, Boys",
            Some((Sport::CrossCountry, Gender::Boys)),
        ),
        ("Cross Country", Some((Sport::CrossCountry, Gender::Mixed))),
        (
            "Track & Field - Indoor, Girls",
            Some((Sport::IndoorTrack, Gender::Girls)),
        ),
        (
            "Track & Field - Outdoor",
            Some((Sport::OutdoorTrack, Gender::Mixed)),
        ),
        (
            "Indoor Track & Field",
            Some((Sport::IndoorTrack, Gender::Mixed)),
        ),
        ("Cross Country Skiing", None),
        ("Tennis", None),
        ("Competitive Cheerleading", None),
        ("", None),
    ];
    for (sport, expected) in cases {
        let row = super::parse::CoachRow {
            first_name: "Sam".to_string(),
            last_name: "Lee".to_string(),
            position: "Head Coach".to_string(),
            sport: sport.to_string(),
            level: "Varsity".to_string(),
        };
        let mapped = map_coach_row(&row, &id, ALVIRNE_URL, OBSERVED_ON)
            .map(|coach| -> TestResult<_> {
                Ok((coach.sport.ok_or("mapped sport")?, coach.gender))
            })
            .transpose()?;
        check!(eq; mapped, expected, "{sport:?}");
    }
    Ok(())
}

#[test]
fn a_coach_row_without_a_name_is_dropped() {
    let id = alvirne_id();
    let row = super::parse::CoachRow {
        first_name: String::new(),
        last_name: String::new(),
        position: "Head Coach".to_string(),
        sport: "Cross Country".to_string(),
        level: "Varsity".to_string(),
    };
    assert!(map_coach_row(&row, &id, ALVIRNE_URL, OBSERVED_ON).is_none());
}

#[test]
fn mapped_coaches_carry_parsed_evidence_with_the_source_url() -> TestResult {
    let (coaches, _) = mapped_coaches(ALVIRNE_COACHES, ALVIRNE_URL, "Alvirne High School")?;
    check!(!coaches.is_empty());
    for coach in &coaches {
        check!(eq; coach.evidence.len(), 1);
        check!(eq; coach.evidence[0].source.id, super::SOURCE_ID);
        check!(eq; coach.evidence[0].source.url.as_deref(), Some(ALVIRNE_URL));
        check!(eq; coach.evidence[0].observed_on, OBSERVED_ON);
    }
    Ok(())
}

#[test]
fn mapped_schools_carry_the_association_identity_and_evidence() -> TestResult {
    let page = parse_org_schools(NH_CHILDREN, CHILDREN_URL)?;
    let (school, id) = map_org_school(
        &page.rows[0],
        UsJurisdiction::NewHampshire,
        ORGANISATION_URL,
        OBSERVED_ON,
    )
    .ok_or("Alvirne maps")?;
    check!(eq; school.name, "Alvirne High School");
    check!(eq; school.state, Some(UsJurisdiction::NewHampshire));
    check!(eq; school.association.as_deref(), Some(super::ASSOCIATION));
    check!(eq; id, alvirne_id());
    check!(eq; school.source_identities.len(), 1);
    check!(eq; school.source_identities[0].id, "450");
    check!(eq; school.evidence.len(), 1);
    check!(eq;
        school.evidence[0].source.url.as_deref(),
        Some(ORGANISATION_URL)
    );
    Ok(())
}

fn seed_cache(cache_dir: &std::path::Path, url: &str, body: &str) -> TestResult {
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
    let meta = serde_json::json!({
        "url": url, "method": "GET", "status": 200,
        "content_digest": format!("{:x}", Sha256::digest(body.as_bytes())),
        "bytes": body.len(), "fetched_at": "2026-09-29T12:00:00Z",
    });
    std::fs::create_dir_all(cache_dir)?;
    std::fs::write(cache_dir.join(format!("{key}.body")), body)?;
    std::fs::write(cache_dir.join(format!("{key}.meta.json")), meta.to_string())?;
    Ok(())
}

#[test]
fn a_short_member_page_below_the_reported_total_is_a_recorded_failure() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let dir = tempfile::tempdir()?;
    let cache = dir.path().join("http");
    let rows: Vec<serde_json::Value> = (0..100).map(|index| {
        serde_json::json!({"name": format!("Short Page School {index}"), "publicId": index + 1000})
    }).collect();
    let body = serde_json::json!({"data": {"total": 150, "rows": rows}}).to_string();
    seed_cache(&cache, CHILDREN_URL, &body)?;
    (0..100).try_for_each(|index| {
        let url = format!(
            "https://services.arbitersports.com/api/v2/legacy/public/2132/coaches?filter.EntityId={}&&pageSize=200&pageNumber=1", index + 1000,
        );
        seed_cache(&cache, &url, EMPTY_COACHES)
    })?;
    let store = Store::open(dir.path().join("store"))?;
    let fetcher = Fetcher::new(&cache, None, Duration::from_millis(1), HashMap::new(), Vec::new())?.with_offline(true);
    let observed_on = OBSERVED_ON.to_string();
    let context = crate::AdapterContext {
        fetcher: &fetcher, store: &store, refresh: false,
        school_year: SchoolYear::new(2026).ok_or("2026 is a season")?,
        observed_on: observed_on.clone(), recording: None,
    };
    let options = Options { states: vec![UsJurisdiction::NewHampshire], observed_on, limit: None, refresh: false };
    let mut run = super::collect::Run {
        ctx: &context, options: &options,
        fetch: crate::net::FetchOptions { refresh: false, allow_not_found: false, headers: Vec::new() },
        done: std::collections::HashSet::new(), tally: super::collect::Tally::default(),
    };
    run.walk(UsJurisdiction::NewHampshire, "2132").await?;
    let tally = run.tally;
    check!(eq; tally.errors, 1, "100 rows read against a total of 150 is a contradiction, and the walk reports it");
    check!(eq; tally.schools, 100, "the rows that were read are still written");
    Ok(())
    })
}

#[test]
fn a_journaled_school_is_skipped_and_an_unwritten_one_is_written() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let dir = tempfile::tempdir()?;
    let cache = dir.path().join("http");
    seed_cache(&cache, ALVIRNE_URL, ALVIRNE_COACHES)?;
    let store = Store::open(dir.path().join("store"))?;
    let fetcher = Fetcher::new(&cache, None, Duration::from_millis(1), HashMap::new(), Vec::new())?.with_offline(true);
    let page = parse_org_schools(NH_CHILDREN, CHILDREN_URL)?;
    let bedford = page.rows.iter().find(|row| row.public_id == Some(1400)).ok_or("Bedford is in the member page")?;
    let alvirne = page.rows.iter().find(|row| row.public_id == Some(450)).ok_or("Alvirne is in the member page")?;
    let bedford_key = "NH:2132:1400";
    let mut batch = store.write_batch();
    batch.journal_done(super::collect::JOURNAL, bedford_key, &serde_json::json!({"seed": "the first run wrote this school"}))?;
    batch.commit()?;
    let observed_on = OBSERVED_ON.to_string();
    let context = crate::AdapterContext {
        fetcher: &fetcher, store: &store, refresh: false,
        school_year: SchoolYear::new(2026).ok_or("2026 is a season")?,
        observed_on: observed_on.clone(), recording: None,
    };
    let options = Options { states: vec![UsJurisdiction::NewHampshire], observed_on, limit: None, refresh: false };
    let done = store.journal_keys(super::collect::JOURNAL)?;
    check!(done.contains(bedford_key));
    let requests_before = fetcher.stats().await.requests;
    let mut run = super::collect::Run {
        ctx: &context, options: &options,
        fetch: crate::net::FetchOptions { refresh: false, allow_not_found: false, headers: Vec::new() },
        done, tally: super::collect::Tally::default(),
    };
    run.process_school(UsJurisdiction::NewHampshire, "2132", bedford, ORGANISATION_URL).await?;
    check!(eq; fetcher.stats().await.requests, requests_before, "a skipped school fetches nothing");
    let skipped_only: Vec<CanonicalSchool> = store.scan(Table::Schools)?;
    check!(skipped_only.is_empty(), "a skipped school is not rewritten");
    run.process_school(UsJurisdiction::NewHampshire, "2132", alvirne, ORGANISATION_URL).await?;
    let tally = run.tally;
    check!(eq; tally.skipped, 1, "the journaled school is counted as skipped");
    check!(eq; tally.schools, 1);
    check!(eq; tally.coaches, 3, "the written school's coach rows are counted, the skipped school's are not");
    check!(eq; tally.errors, 0);
    let schools: Vec<CanonicalSchool> = store.scan(Table::Schools)?;
    check!(eq; schools.len(), 1);
    check!(eq; schools.first().ok_or("written school")?.name, "Alvirne High School");
    let coaches: Vec<CanonicalCoach> = store.scan(Table::Coaches)?;
    check!(eq; coaches.len(), 3, "the athletic director plus the two captured head coaches");
    let journaled = store.journal_keys(super::collect::JOURNAL)?;
    check!(journaled.contains("NH:2132:450"));
    Ok(())
    })
}

#[test]
fn a_full_member_page_is_written_before_the_next_page_is_fetched() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let dir = tempfile::tempdir()?;
    let cache = dir.path().join("http");
    let rows: Vec<serde_json::Value> = (0..200).map(|index| {
        serde_json::json!({"name": format!("Streamed School {index}"), "publicId": index + 1000})
    }).collect();
    let body = serde_json::json!({"data": {"total": 250, "rows": rows}}).to_string();
    seed_cache(&cache, CHILDREN_URL, &body)?;
    (0..200).try_for_each(|index| {
        let url = format!(
            "https://services.arbitersports.com/api/v2/legacy/public/2132/coaches?filter.EntityId={}&&pageSize=200&pageNumber=1", index + 1000,
        );
        seed_cache(&cache, &url, EMPTY_COACHES)
    })?;
    let store = Store::open(dir.path().join("store"))?;
    let fetcher = Fetcher::new(&cache, None, Duration::from_millis(1), HashMap::new(), Vec::new())?.with_offline(true);
    let observed_on = OBSERVED_ON.to_string();
    let context = crate::AdapterContext {
        fetcher: &fetcher, store: &store, refresh: false,
        school_year: SchoolYear::new(2026).ok_or("2026 is a season")?,
        observed_on: observed_on.clone(), recording: None,
    };
    let options = Options { states: vec![UsJurisdiction::NewHampshire], observed_on, limit: None, refresh: false };
    let mut run = super::collect::Run {
        ctx: &context, options: &options,
        fetch: crate::net::FetchOptions { refresh: false, allow_not_found: false, headers: Vec::new() },
        done: std::collections::HashSet::new(), tally: super::collect::Tally::default(),
    };
    run.walk(UsJurisdiction::NewHampshire, "2132").await?;
    let tally = run.tally;
    check!(eq; tally.schools, 200, "the full first page is written before the walk asks for page 2");
    check!(eq; tally.errors, 1, "the uncached second page is the one recorded failure");
    let schools: Vec<CanonicalSchool> = store.scan(Table::Schools)?;
    check!(eq; schools.len(), 200);
    Ok(())
    })
}

#[test]
fn a_coach_walk_stopped_by_the_page_bound_records_a_failure() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let dir = tempfile::tempdir()?;
    let cache = dir.path().join("http");
    let rows: Vec<serde_json::Value> = (0..200).map(|_| serde_json::json!({})).collect();
    let body = serde_json::json!({"data": {"total": 12_800, "rows": rows}}).to_string();
    for page in 1..=64u64 {
        let url = format!("https://services.arbitersports.com/api/v2/legacy/public/2132/coaches?filter.EntityId=450&&pageSize=200&pageNumber={page}");
        seed_cache(&cache, &url, &body)?;
    }
    let store = Store::open(dir.path().join("store"))?;
    let fetcher = Fetcher::new(&cache, None, Duration::from_millis(1), HashMap::new(), Vec::new())?.with_offline(true);
    let observed_on = OBSERVED_ON.to_string();
    let context = crate::AdapterContext {
        fetcher: &fetcher, store: &store, refresh: false,
        school_year: SchoolYear::new(2026).ok_or("2026 is a season")?,
        observed_on: observed_on.clone(), recording: None,
    };
    let options = Options { states: vec![UsJurisdiction::NewHampshire], observed_on, limit: None, refresh: false };
    let page = parse_org_schools(NH_CHILDREN, CHILDREN_URL)?;
    let alvirne = page.rows.iter().find(|row| row.public_id == Some(450)).ok_or("Alvirne is in the member page")?;
    let mut run = super::collect::Run {
        ctx: &context, options: &options,
        fetch: crate::net::FetchOptions { refresh: false, allow_not_found: false, headers: Vec::new() },
        done: std::collections::HashSet::new(), tally: super::collect::Tally::default(),
    };
    run.process_school(UsJurisdiction::NewHampshire, "2132", alvirne, ORGANISATION_URL).await?;
    let tally = run.tally;
    check!(eq; tally.schools, 1, "the school is written without its coaches");
    check!(eq; tally.errors, 1);
    check!(tally.notes.iter().any(|note| note.contains("more pages than the 64-page walk")),
        "the coach truncation is named rather than silent: {:?}", tally.notes);
    Ok(())
    })
}

#[test]
fn a_state_named_twice_is_walked_once() -> TestResult {
    let options = Options {
        states: vec![
            UsJurisdiction::NewHampshire,
            UsJurisdiction::NewHampshire,
            UsJurisdiction::Kentucky,
        ],
        observed_on: OBSERVED_ON.to_string(),
        limit: None,
        refresh: false,
    };
    let targets = super::collect::targets(&options)?;
    check!(eq; targets.len(), 2, "the repeated state is walked once");
    check!(eq; targets[0], (UsJurisdiction::NewHampshire, "2132"));
    check!(eq; targets[1], (UsJurisdiction::Kentucky, "2507"));
    Ok(())
}

#[test]
fn the_token_is_never_replayed_from_the_cache() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let cache = dir.path().join("http");
            seed_cache(&cache, super::DIRECTORY_URL, SYNTHETIC_ENTRY)?;
            seed_cache(&cache, SYNTHETIC_BUNDLE_URL, BUNDLE)?;
            seed_cache(&cache, CHILDREN_URL, NH_CHILDREN)?;
            let (client_id, client_secret) =
                credentials_in_bundle(BUNDLE).ok_or("the derived bundle carries the pair")?;
            let encoded = url::form_urlencoded::Serializer::new(String::new())
                .extend_pairs([
                    ("client_id", client_id.as_str()),
                    ("client_secret", client_secret.as_str()),
                    ("grant_type", "client_credentials"),
                    ("scope", super::TOKEN_SCOPE),
                ])
                .finish();
            let key = Fetcher::key_for("POST", super::TOKEN_URL, &encoded);
            std::fs::write(cache.join(format!("{key}.body")), TOKEN)?;
            std::fs::write(
                cache.join(format!("{key}.meta.json")),
                serde_json::json!({
                    "url": super::TOKEN_URL, "method": "POST", "status": 200,
                    "content_digest": crate::net::cache::content_digest(TOKEN.as_bytes()),
                    "bytes": TOKEN.len(), "fetched_at": "2026-09-29T12:00:00Z",
                })
                .to_string(),
            )?;
            let store = Store::open(dir.path().join("store"))?;
            let fetcher = Fetcher::new(
                &cache,
                None,
                Duration::from_millis(1),
                HashMap::new(),
                Vec::new(),
            )?
            .with_offline(true);
            let observed_on = OBSERVED_ON.to_string();
            let context = crate::AdapterContext {
                fetcher: &fetcher,
                store: &store,
                refresh: false,
                school_year: SchoolYear::new(2026).ok_or("2026 is a season")?,
                observed_on: observed_on.clone(),
                recording: None,
            };
            let options = Options {
                states: vec![UsJurisdiction::NewHampshire],
                observed_on,
                limit: Some(1),
                refresh: false,
            };
            match super::collect(&context, &options).await {
                Err(crate::CrawlError::Fetch(crate::net::FetchError::Offline { url })) => {
                    check!(eq; url, super::TOKEN_URL)
                }
                Err(error) => return Err(error.into()),
                Ok(_) => {
                    return Err(
                        "a cached token body is not replayed: the POST is issued and fails offline"
                            .into(),
                    )
                }
            }
            let schools: Vec<CanonicalSchool> = store.scan(Table::Schools)?;
            check!(
                schools.is_empty(),
                "no school is written without a freshly minted token"
            );
            Ok(())
        })
}
