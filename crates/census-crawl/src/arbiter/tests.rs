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

use crate::net::Fetcher;

const NH_CHILDREN: &str = include_str!("../../tests/fixtures/arbiter/nh_children_p1.body");
const ALVIRNE_COACHES: &str = include_str!("../../tests/fixtures/arbiter/alvirne_coaches_p1.body");
const NH_ALL_COACHES_P11: &str =
    include_str!("../../tests/fixtures/arbiter/nh_all_coaches_p11.body");
const EMPTY_COACHES: &str = include_str!("../../tests/fixtures/arbiter/coaches_empty.body");
const NOT_JSON: &str = include_str!("../../tests/fixtures/arbiter/not_json.body");
const BUNDLE: &str = include_str!("../../tests/fixtures/arbiter/bundle_credentials.derived.js");
const TOKEN: &str = include_str!("../../tests/fixtures/arbiter/token_response.derived.json");

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

fn golden_rows(json: &str) -> Vec<serde_json::Value> {
    serde_json::from_str(json).expect("golden is a JSON array")
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

fn mapped_coaches(body: &str, url: &str, school: &str) -> (Vec<CanonicalCoach>, CanonicalSchool) {
    let parsed = parse_org_schools(NH_CHILDREN, CHILDREN_URL).expect("children parse");
    let row = parsed
        .rows
        .iter()
        .find(|row| row.name == school)
        .expect("the school is in the children page");
    let (school_row, id) = map_org_school(
        row,
        UsJurisdiction::NewHampshire,
        ORGANISATION_URL,
        OBSERVED_ON,
    )
    .expect("the school maps");
    let coaches = parse_coach_rows(body, url)
        .expect("coaches parse")
        .rows
        .iter()
        .filter_map(|coach| map_coach_row(coach, &id, url, OBSERVED_ON))
        .collect();
    (coaches, school_row)
}

#[test]
fn the_schools_page_carries_all_89_nhiaa_schools() {
    let page = parse_org_schools(NH_CHILDREN, CHILDREN_URL).expect("children parse");
    assert_eq!(page.total, 89);
    assert_eq!(page.rows.len(), 89);
    let first = &page.rows[0];
    assert_eq!(first.name, "Alvirne High School");
    assert_eq!(first.public_id, Some(450));
    assert_eq!(first.org_id, Some(29072));
    assert_eq!(first.phone.as_deref(), Some("6038861260"));
    assert_eq!(first.enrollment, Some(955));
    let contact = first
        .primary_contact
        .as_ref()
        .expect("Alvirne has a contact");
    assert_eq!(contact.first_name, "Justin");
    assert_eq!(contact.last_name, "Hufft");
    assert_eq!(contact.role_name, "Athletic Director");
}

#[test]
fn every_school_row_matches_the_prototype_golden_field_for_field() {
    let page = parse_org_schools(NH_CHILDREN, CHILDREN_URL).expect("children parse");
    let golden = golden_rows(GOLDEN_SCHOOLS);
    assert_eq!(page.rows.len(), golden.len());

    let mut rust: BTreeMap<
        String,
        (
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        ),
    > = BTreeMap::new();
    for row in &page.rows {
        let (school, _) = map_org_school(
            row,
            UsJurisdiction::NewHampshire,
            ORGANISATION_URL,
            OBSERVED_ON,
        )
        .expect("every golden row has a name");
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
    let mut expected: BTreeMap<
        String,
        (
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        ),
    > = BTreeMap::new();
    for row in &golden {
        expected.insert(
            field(row, "name").expect("golden rows carry a name"),
            (
                field(row, "association_id"),
                field(row, "arbiter_org_id"),
                field(row, "phone"),
                field(row, "enrollment"),
            ),
        );
    }
    assert_eq!(rust, expected, "school fields differ from the prototype");
}

#[test]
fn every_primary_contact_becomes_an_athletic_director_row() {
    let page = parse_org_schools(NH_CHILDREN, CHILDREN_URL).expect("children parse");
    let golden = golden_rows(GOLDEN_CONTACTS);
    assert_eq!(
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
        .expect("every golden row has a name");
        let Some(contact) = row.primary_contact.as_ref() else {
            continue;
        };
        let Some(coach) = map_primary_contact(contact, &id, ORGANISATION_URL, OBSERVED_ON) else {
            continue;
        };
        assert_eq!(coach.role, CoachRole::AthleticDirector);
        assert_eq!(coach.sport, None, "a director has no sporting side");
        assert_eq!(coach.gender, Gender::Mixed);
        assert_eq!(coach.evidence.len(), 1);
        assert_eq!(coach.evidence[0].source.id, super::SOURCE_ID);
        rust.insert((school.name.clone(), coach.name.clone()));
    }

    let expected: BTreeSet<(String, String)> = golden
        .iter()
        .map(|row| {
            (
                field(row, "school").unwrap_or_default(),
                field(row, "person").unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(rust, expected, "contact rows differ from the prototype");
}

#[test]
fn alvirne_coach_rows_match_the_prototype_on_person_sport_and_role() {
    let golden = golden_rows(GOLDEN_ALVIRNE);
    assert_eq!(golden.len(), 2, "the capture holds two cross-country rows");

    let rows = parse_coach_rows(ALVIRNE_COACHES, ALVIRNE_URL)
        .expect("coaches parse")
        .rows;
    let id = alvirne_id();
    let rust: BTreeSet<(String, String, String)> = rows
        .iter()
        .filter_map(|row| map_coach_row(row, &id, ALVIRNE_URL, OBSERVED_ON))
        .map(|coach| {
            (
                coach.name.clone(),
                coach
                    .sport
                    .expect("mapped coaches have a sport")
                    .stable_key()
                    .to_string(),
                coach.role.stable_key().to_string(),
            )
        })
        .collect();
    let expected: BTreeSet<(String, String, String)> = golden
        .iter()
        .map(|row| {
            (
                field(row, "person").unwrap_or_default(),
                field(row, "sport").unwrap_or_default(),
                field(row, "role").unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(rust, expected);
}

#[test]
fn the_side_of_a_coach_comes_from_the_level_label() {
    let (coaches, _) = mapped_coaches(ALVIRNE_COACHES, ALVIRNE_URL, "Alvirne High School");
    let by_name: BTreeMap<&str, &CanonicalCoach> = coaches
        .iter()
        .map(|coach| (coach.name.as_str(), coach))
        .collect();
    let demers = by_name.get("Phillip Demers").expect("Demers is mapped");
    assert_eq!(demers.gender, Gender::Boys);
    assert_eq!(demers.sport, Some(Sport::CrossCountry));
    let wilson = by_name.get("Kaitlyn Wilson").expect("Wilson is mapped");
    assert_eq!(wilson.gender, Gender::Girls);
}

#[test]
fn the_prototype_drops_arbiter_track_variants_and_this_lane_keeps_them() {
    let golden = golden_rows(GOLDEN_P11);
    assert!(
        golden.is_empty(),
        "the prototype's exact-key sport table maps 'Track & Field - Indoor' to nothing"
    );

    let page = parse_coach_rows(NH_ALL_COACHES_P11, ORGANISATION_URL).expect("page 11 parses");
    assert_eq!(page.total, 2512);
    assert_eq!(page.rows.len(), 200);
    let id = alvirne_id();
    let mapped: Vec<CanonicalCoach> = page
        .rows
        .iter()
        .filter_map(|row| map_coach_row(row, &id, ORGANISATION_URL, OBSERVED_ON))
        .collect();
    assert_eq!(mapped.len(), 121, "121 of 200 rows survive the filters");

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
            other => panic!("unexpected sport {other:?}"),
        }
        match coach.gender {
            Gender::Boys => boys += 1,
            Gender::Girls => girls += 1,
            Gender::Mixed => mixed += 1,
            Gender::Unknown => panic!("the level label always states a side"),
        }
        match coach.role {
            CoachRole::HeadCoach => head += 1,
            CoachRole::AssistantCoach => assistant += 1,
            other => panic!("unexpected role {other:?}"),
        }
    }
    assert_eq!((indoor, outdoor), (78, 43));
    assert_eq!((boys, girls, mixed), (69, 29, 23));
    assert_eq!((head, assistant), (78, 43));
}

#[test]
fn sub_varsity_rows_are_dropped() {
    let page = parse_coach_rows(NH_ALL_COACHES_P11, ORGANISATION_URL).expect("page 11 parses");
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
    assert_eq!(eligible, 137, "the page holds 137 eligible track rows");
    assert_eq!(
        eligible - mapped,
        16,
        "the 16 sub-varsity rows (Coed Unified, Coed Middle School, Boys JV) are dropped"
    );
}

#[test]
fn an_empty_coach_page_yields_no_rows() {
    let page = parse_coach_rows(EMPTY_COACHES, ORGANISATION_URL).expect("empty page parses");
    assert_eq!(page.total, 0);
    assert!(page.rows.is_empty());
    let id = alvirne_id();
    assert!(page
        .rows
        .iter()
        .filter_map(|row| map_coach_row(row, &id, ORGANISATION_URL, OBSERVED_ON))
        .next()
        .is_none());
}

#[test]
fn a_non_json_body_is_a_completed_error_outcome() {
    assert!(parse_org_schools(NOT_JSON, CHILDREN_URL).is_err());
    assert!(parse_coach_rows(NOT_JSON, ALVIRNE_URL).is_err());
    assert!(parse_token(NOT_JSON, super::TOKEN_URL).is_err());
}

#[test]
fn the_published_bundle_yields_the_client_credentials() {
    let (client_id, client_secret) =
        credentials_in_bundle(BUNDLE).expect("the derived bundle slice carries the pair");
    assert_eq!(client_id, "CLIENT-ID-REDACTED");
    assert_eq!(client_secret, "CLIENT-SECRET-REDACTED");
    assert!(credentials_in_bundle("<html>nothing here</html>").is_none());
    assert!(credentials_in_bundle("client_id:\"\",client_secret:\"x\"").is_none());
    assert!(credentials_in_bundle("client_id:\"x\",client_secret:\"\"").is_none());
}

#[test]
fn the_token_response_yields_the_access_token() {
    let token = parse_token(TOKEN, super::TOKEN_URL).expect("the derived response parses");
    assert_eq!(token, "ACCESS-TOKEN-REDACTED");
    assert!(parse_token("{\"expires_in\":86400}", super::TOKEN_URL).is_err());
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
fn only_identifiable_coach_roles_are_mapped() {
    let id = alvirne_id();
    let row = super::parse::CoachRow {
        first_name: "Sam".to_string(),
        last_name: "Lee".to_string(),
        position: "Assistant Coach".to_string(),
        sport: "Cross Country, Boys".to_string(),
        level: "Boys Varsity".to_string(),
    };
    let coach = map_coach_row(&row, &id, ALVIRNE_URL, OBSERVED_ON).expect("assistant coach maps");
    assert_eq!(coach.role, CoachRole::AssistantCoach);
    assert_eq!(coach.gender, Gender::Boys);

    for position in ["Volunteer Coach", "Secretary", "", "Team Manager"] {
        let row = super::parse::CoachRow {
            first_name: "Sam".to_string(),
            last_name: "Lee".to_string(),
            position: position.to_string(),
            sport: "Cross Country".to_string(),
            level: "Varsity".to_string(),
        };
        assert!(
            map_coach_row(&row, &id, ALVIRNE_URL, OBSERVED_ON).is_none(),
            "{position:?} is not an identifiable coach role"
        );
    }
}

#[test]
fn only_cross_country_and_track_sports_are_mapped() {
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
        let mapped = map_coach_row(&row, &id, ALVIRNE_URL, OBSERVED_ON);
        assert_eq!(
            mapped.map(|coach| (coach.sport.expect("mapped sport"), coach.gender)),
            expected,
            "{sport:?}"
        );
    }
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
fn mapped_coaches_carry_parsed_evidence_with_the_source_url() {
    let (coaches, _) = mapped_coaches(ALVIRNE_COACHES, ALVIRNE_URL, "Alvirne High School");
    assert!(!coaches.is_empty());
    for coach in &coaches {
        assert_eq!(coach.evidence.len(), 1);
        assert_eq!(coach.evidence[0].source.id, super::SOURCE_ID);
        assert_eq!(coach.evidence[0].source.url.as_deref(), Some(ALVIRNE_URL));
        assert_eq!(coach.evidence[0].observed_on, OBSERVED_ON);
    }
}

#[test]
fn mapped_schools_carry_the_association_identity_and_evidence() {
    let page = parse_org_schools(NH_CHILDREN, CHILDREN_URL).expect("children parse");
    let (school, id) = map_org_school(
        &page.rows[0],
        UsJurisdiction::NewHampshire,
        ORGANISATION_URL,
        OBSERVED_ON,
    )
    .expect("Alvirne maps");
    assert_eq!(school.name, "Alvirne High School");
    assert_eq!(school.state, Some(UsJurisdiction::NewHampshire));
    assert_eq!(school.association.as_deref(), Some(super::ASSOCIATION));
    assert_eq!(id, alvirne_id());
    assert_eq!(school.source_identities.len(), 1);
    assert_eq!(school.source_identities[0].id, "450");
    assert_eq!(school.evidence.len(), 1);
    assert_eq!(
        school.evidence[0].source.url.as_deref(),
        Some(ORGANISATION_URL)
    );
}

#[test]
fn the_descriptor_is_registered_as_a_token_gated_structured_api() {
    let entry = crate::registry::descriptor(super::SOURCE_ID).expect("arbiter_orgs is registered");
    assert_eq!(
        entry.transport,
        crate::registry::TransportKind::StructuredApi
    );
    assert!(entry.capabilities.school_evidence);
    assert!(entry.capabilities.coach_directory);
    assert!(
        !entry.capabilities.public_professional_contact,
        "the API publishes no coach email or phone"
    );
    assert_eq!(entry.admission.origin, "services.arbitersports.com");
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
    let meta = serde_json::json!({
        "url": url,
        "method": "GET",
        "status": 200,
        "content_digest": format!("{:x}", Sha256::digest(body.as_bytes())),
        "bytes": body.len(),
        "fetched_at": "2026-09-29T12:00:00Z",
    });
    std::fs::create_dir_all(cache_dir).expect("cache dir");
    std::fs::write(cache_dir.join(format!("{key}.body")), body).expect("cache body");
    std::fs::write(cache_dir.join(format!("{key}.meta.json")), meta.to_string())
        .expect("cache meta");
}

#[tokio::test]
async fn a_short_member_page_below_the_reported_total_is_a_recorded_failure() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cache = dir.path().join("http");
    let rows: Vec<serde_json::Value> = (0..100)
        .map(|index| serde_json::json!({"name": format!("Short Page School {index}")}))
        .collect();
    let body = serde_json::json!({"data": {"total": 150, "rows": rows}}).to_string();
    seed_cache(&cache, CHILDREN_URL, &body);
    let store = Store::open(dir.path().join("store")).expect("store");
    let fetcher = Fetcher::new(
        &cache,
        None,
        Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )
    .expect("fetcher")
    .with_offline(true);
    let observed_on = OBSERVED_ON.to_string();
    let context = crate::AdapterContext {
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: SchoolYear::new(2026).expect("2026 is a season"),
        observed_on: observed_on.clone(),
        recording: None,
    };
    let options = Options {
        states: vec![UsJurisdiction::NewHampshire],
        observed_on,
        limit: None,
        refresh: false,
    };
    let mut run = super::collect::Run {
        ctx: &context,
        options: &options,
        fetch: crate::net::FetchOptions {
            refresh: false,
            allow_not_found: false,
            headers: Vec::new(),
        },
        done: std::collections::HashSet::new(),
        tally: super::collect::Tally::default(),
    };
    run.walk(UsJurisdiction::NewHampshire, "2132")
        .await
        .expect("a short page is not an error result");
    let tally = run.tally;
    assert_eq!(
        tally.errors, 1,
        "100 rows read against a total of 150 is a contradiction, and the walk reports it"
    );
    assert!(
        tally
            .notes
            .iter()
            .any(|note| note.contains("stopped short")),
        "the note names the shortfall: {:?}",
        tally.notes
    );
    assert_eq!(
        tally.schools, 100,
        "the rows that were read are still written"
    );
}

#[tokio::test]
async fn a_journaled_school_is_skipped_and_an_unwritten_one_is_written() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cache = dir.path().join("http");
    seed_cache(&cache, ALVIRNE_URL, ALVIRNE_COACHES);
    let store = Store::open(dir.path().join("store")).expect("store");
    let fetcher = Fetcher::new(
        &cache,
        None,
        Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )
    .expect("fetcher")
    .with_offline(true);
    let page = parse_org_schools(NH_CHILDREN, CHILDREN_URL).expect("children parse");
    let bedford = page
        .rows
        .iter()
        .find(|row| row.public_id == Some(1400))
        .expect("Bedford is in the member page");
    let alvirne = page
        .rows
        .iter()
        .find(|row| row.public_id == Some(450))
        .expect("Alvirne is in the member page");

    let bedford_id = CanonicalSchool::mint(
        UsJurisdiction::NewHampshire,
        "Bedford High School -NH",
        &normalize_name("Bedford High School -NH"),
    );
    let mut batch = store.write_batch();
    batch
        .journal_done(
            super::collect::JOURNAL,
            bedford_id.as_str(),
            &serde_json::json!({"seed": "the first run wrote this school"}),
        )
        .expect("seed journal");
    batch.commit().expect("commit");

    let observed_on = OBSERVED_ON.to_string();
    let context = crate::AdapterContext {
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: SchoolYear::new(2026).expect("2026 is a season"),
        observed_on: observed_on.clone(),
        recording: None,
    };
    let options = Options {
        states: vec![UsJurisdiction::NewHampshire],
        observed_on,
        limit: None,
        refresh: false,
    };
    let done = store
        .journal_keys(super::collect::JOURNAL)
        .expect("journal keys");
    assert!(done.contains(bedford_id.as_str()));

    let requests_before = fetcher.stats().await.requests;
    let mut run = super::collect::Run {
        ctx: &context,
        options: &options,
        fetch: crate::net::FetchOptions {
            refresh: false,
            allow_not_found: false,
            headers: Vec::new(),
        },
        done,
        tally: super::collect::Tally::default(),
    };
    run.process_school(
        UsJurisdiction::NewHampshire,
        "2132",
        bedford,
        ORGANISATION_URL,
    )
    .await
    .expect("the journaled school is skipped without error");
    assert_eq!(
        fetcher.stats().await.requests,
        requests_before,
        "a skipped school fetches nothing"
    );
    let skipped_only: Vec<CanonicalSchool> = store.scan(Table::Schools).expect("scan schools");
    assert!(skipped_only.is_empty(), "a skipped school is not rewritten");
    run.process_school(
        UsJurisdiction::NewHampshire,
        "2132",
        alvirne,
        ORGANISATION_URL,
    )
    .await
    .expect("the unwritten school is processed from the cached coach page");
    let tally = run.tally;

    assert_eq!(
        tally.skipped, 1,
        "the journaled school is counted as skipped"
    );
    assert_eq!(tally.schools, 1);
    assert_eq!(tally.errors, 0);
    let schools: Vec<CanonicalSchool> = store.scan(Table::Schools).expect("scan schools");
    assert_eq!(schools.len(), 1);
    assert_eq!(schools[0].name, "Alvirne High School");
    let coaches: Vec<CanonicalCoach> = store.scan(Table::Coaches).expect("scan coaches");
    assert_eq!(
        coaches.len(),
        3,
        "the athletic director plus the two captured head coaches"
    );
    let journaled = store
        .journal_keys(super::collect::JOURNAL)
        .expect("journal keys");
    assert!(journaled.contains(alvirne_id().as_str()));
}

#[tokio::test]
async fn a_full_member_page_is_written_before_the_next_page_is_fetched() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cache = dir.path().join("http");
    let rows: Vec<serde_json::Value> = (0..200)
        .map(|index| serde_json::json!({"name": format!("Streamed School {index}")}))
        .collect();
    let body = serde_json::json!({"data": {"total": 250, "rows": rows}}).to_string();
    seed_cache(&cache, CHILDREN_URL, &body);
    let store = Store::open(dir.path().join("store")).expect("store");
    let fetcher = Fetcher::new(
        &cache,
        None,
        Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )
    .expect("fetcher")
    .with_offline(true);
    let observed_on = OBSERVED_ON.to_string();
    let context = crate::AdapterContext {
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: SchoolYear::new(2026).expect("2026 is a season"),
        observed_on: observed_on.clone(),
        recording: None,
    };
    let options = Options {
        states: vec![UsJurisdiction::NewHampshire],
        observed_on,
        limit: None,
        refresh: false,
    };
    let mut run = super::collect::Run {
        ctx: &context,
        options: &options,
        fetch: crate::net::FetchOptions {
            refresh: false,
            allow_not_found: false,
            headers: Vec::new(),
        },
        done: std::collections::HashSet::new(),
        tally: super::collect::Tally::default(),
    };
    run.walk(UsJurisdiction::NewHampshire, "2132")
        .await
        .expect("a missing page is recorded as a failure, not returned");
    let tally = run.tally;
    assert_eq!(
        tally.schools, 200,
        "the full first page is written before the walk asks for page 2"
    );
    assert_eq!(
        tally.errors, 1,
        "the uncached second page is the one recorded failure"
    );
    let schools: Vec<CanonicalSchool> = store.scan(Table::Schools).expect("scan schools");
    assert_eq!(schools.len(), 200);
}

#[tokio::test]
async fn a_coach_walk_stopped_by_the_page_bound_records_a_failure() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cache = dir.path().join("http");
    let rows: Vec<serde_json::Value> = (0..200).map(|_| serde_json::json!({})).collect();
    let body = serde_json::json!({"data": {"total": 12_800, "rows": rows}}).to_string();
    for page in 1..=64u64 {
        let url = format!(
            "https://services.arbitersports.com/api/v2/legacy/public/2132/coaches?filter.EntityId=450&&pageSize=200&pageNumber={page}"
        );
        seed_cache(&cache, &url, &body);
    }
    let store = Store::open(dir.path().join("store")).expect("store");
    let fetcher = Fetcher::new(
        &cache,
        None,
        Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )
    .expect("fetcher")
    .with_offline(true);
    let observed_on = OBSERVED_ON.to_string();
    let context = crate::AdapterContext {
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: SchoolYear::new(2026).expect("2026 is a season"),
        observed_on: observed_on.clone(),
        recording: None,
    };
    let options = Options {
        states: vec![UsJurisdiction::NewHampshire],
        observed_on,
        limit: None,
        refresh: false,
    };
    let page = parse_org_schools(NH_CHILDREN, CHILDREN_URL).expect("children parse");
    let alvirne = page
        .rows
        .iter()
        .find(|row| row.public_id == Some(450))
        .expect("Alvirne is in the member page");
    let mut run = super::collect::Run {
        ctx: &context,
        options: &options,
        fetch: crate::net::FetchOptions {
            refresh: false,
            allow_not_found: false,
            headers: Vec::new(),
        },
        done: std::collections::HashSet::new(),
        tally: super::collect::Tally::default(),
    };
    run.process_school(
        UsJurisdiction::NewHampshire,
        "2132",
        alvirne,
        ORGANISATION_URL,
    )
    .await
    .expect("a page-bound coach walk is recorded as a failure, not returned");
    let tally = run.tally;
    assert_eq!(
        tally.schools, 1,
        "the school is written without its coaches"
    );
    assert_eq!(tally.errors, 1);
    assert!(
        tally
            .notes
            .iter()
            .any(|note| note.contains("more pages than the 64-page walk")),
        "the coach truncation is named rather than silent: {:?}",
        tally.notes
    );
}

#[test]
fn a_state_named_twice_is_walked_once() {
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
    let targets = super::collect::targets(&options).expect("both states are registered");
    assert_eq!(targets.len(), 2, "the repeated state is walked once");
    assert_eq!(targets[0], (UsJurisdiction::NewHampshire, "2132"));
    assert_eq!(targets[1], (UsJurisdiction::Kentucky, "2507"));
}

#[test]
fn a_refreshing_context_refreshes_even_when_the_options_do_not() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cache = dir.path().join("http");
    let store = Store::open(dir.path().join("store")).expect("store");
    let fetcher = Fetcher::new(
        &cache,
        None,
        Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )
    .expect("fetcher");
    let context = crate::AdapterContext {
        fetcher: &fetcher,
        store: &store,
        refresh: true,
        school_year: SchoolYear::new(2026).expect("2026 is a season"),
        observed_on: OBSERVED_ON.to_string(),
        recording: None,
    };
    let fetch = super::collect::fetch_options(&context, &Options::default(), Vec::new());
    assert!(
        fetch.refresh,
        "a refreshing context is not served from the cache"
    );
    assert!(!fetch.allow_not_found);
    assert!(fetch.headers.is_empty());
}
