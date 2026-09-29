use super::survey::{
    parse_state_filter, probe_one, report_json, round_half_even, sample_rows,
    selected_associations, table_line, ProbeRecord, ASSOCIATIONS, VERIFIED,
};
use super::API_HOST;
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::Fetcher;
use census_domain::UsJurisdiction;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

fn seed_cache(fetcher: &Fetcher, url: &str, status: u16, body: &[u8]) {
    let key = Fetcher::key_for("GET", url, "");
    let (body_path, meta_path) = fetcher.cache_paths(&key);
    let meta = CacheMeta {
        url: url.to_string(),
        method: "GET".to_string(),
        status,
        content_digest: content_digest(body),
        bytes: body.len(),
        fetched_at: "2026-09-29T00:00:00Z".to_string(),
        etag: None,
        last_modified: None,
        content_type: None,
    };
    write_cache(&body_path, &meta_path, body, &meta).expect("write cache");
}

fn fixture(relative: &str) -> String {
    let path = format!("{}/tests/fixtures/{relative}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(path).expect("fixture body")
}

fn make_offline_fetcher(cache_dir: &std::path::Path) -> Fetcher {
    Fetcher::new(
        cache_dir.join("http"),
        None,
        Duration::from_millis(1),
        std::collections::HashMap::new(),
        vec![],
    )
    .expect("fetcher")
    .with_offline(true)
}

#[tokio::test]
async fn offline_probe_ak_zero_staff() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = make_offline_fetcher(dir.path());

    let body = fixture("coach_directories/probe/AK/directory-1.json");
    let url = format!("{API_HOST}/states/ASAA/directory/1");
    seed_cache(&fetcher, &url, 200, body.as_bytes());

    let summary_files = [
        "coach_directories/probe/AK/summary-DEX2BG.json",
        "coach_directories/probe/AK/summary-HU9FJ6.json",
        "coach_directories/probe/AK/summary-LHB72E.json",
        "coach_directories/probe/AK/summary-WFRCLF.json",
    ];
    let summary_codes = ["DEX2BG", "HU9FJ6", "LHB72E", "WFRCLF"];
    for (path, code) in summary_files.iter().zip(summary_codes.iter()) {
        let body = fixture(path);
        let url = format!("{API_HOST}/schools/{code}/summary");
        seed_cache(&fetcher, &url, 200, body.as_bytes());
    }

    let record = probe_one(&fetcher, UsJurisdiction::Alaska, "ASAA").await;
    assert_eq!(record.status, "ok");
    assert_eq!(record.schools, Some(355));
    assert_eq!(record.with_address, Some(355));
    assert_eq!(record.pages, Some(1));
    assert_eq!(record.directory_total, Some(355));
    assert_eq!(record.sampled, Some(4));
    assert_eq!(record.staff, Some(0));
    assert_eq!(record.coaches, Some(0));
    assert!(record.sports.is_some());
    assert_eq!(record.sports.as_ref().unwrap().len(), 0);
    assert_eq!(record.staff_per_school, Some(0.0));
    assert_eq!(record.coaches_per_school, Some(0.0));
}

#[tokio::test]
async fn offline_probe_al_classified_path() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = make_offline_fetcher(dir.path());

    let body = fixture("coach_directories/probe/AL/directory-1.json");
    let url = format!("{API_HOST}/states/AHSAA/directory/1");
    seed_cache(&fetcher, &url, 200, body.as_bytes());

    let summary_files = [
        "coach_directories/probe/AL/summary-42XLF4.json",
        "coach_directories/probe/AL/summary-4E52LB.json",
        "coach_directories/probe/AL/summary-SVXJDF.json",
        "coach_directories/probe/AL/summary-VHY8C9.json",
    ];
    let summary_codes = ["42XLF4", "4E52LB", "SVXJDF", "VHY8C9"];
    for (path, code) in summary_files.iter().zip(summary_codes.iter()) {
        let body = fixture(path);
        let url = format!("{API_HOST}/schools/{code}/summary");
        seed_cache(&fetcher, &url, 200, body.as_bytes());
    }

    let record = probe_one(&fetcher, UsJurisdiction::Alabama, "AHSAA").await;
    assert_eq!(record.status, "ok");
    assert_eq!(record.schools, Some(793));
    assert_eq!(record.with_address, Some(730));
    assert_eq!(record.pages, Some(1));
    assert_eq!(record.directory_total, Some(793));
    assert_eq!(record.sampled, Some(4));
    assert_eq!(record.staff, Some(77));
    assert_eq!(record.coaches, Some(20));

    let sports = record.sports.as_ref().unwrap();
    assert_eq!(sports.get("Track"), Some(&6));
    assert_eq!(sports.get("CrossCountry"), Some(&7));
    assert_eq!(sports.get("AthleticDirector"), Some(&7));

    assert_eq!(record.staff_per_school, Some(19.2));
    assert_eq!(record.coaches_per_school, Some(5.0));
}

#[tokio::test]
async fn offline_probe_wy_half_even_ratio() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = make_offline_fetcher(dir.path());

    let body = fixture("coach_directories/probe/WY/directory-1.json");
    let url = format!("{API_HOST}/states/WHSAA/directory/1");
    seed_cache(&fetcher, &url, 200, body.as_bytes());

    let summary_files = [
        "coach_directories/probe/WY/summary-BV8LHG.json",
        "coach_directories/probe/WY/summary-JNXGSZ.json",
        "coach_directories/probe/WY/summary-SS28UB.json",
        "coach_directories/probe/WY/summary-YZQ9H7.json",
    ];
    let summary_codes = ["BV8LHG", "JNXGSZ", "SS28UB", "YZQ9H7"];
    for (path, code) in summary_files.iter().zip(summary_codes.iter()) {
        let body = fixture(path);
        let url = format!("{API_HOST}/schools/{code}/summary");
        seed_cache(&fetcher, &url, 200, body.as_bytes());
    }

    let record = probe_one(&fetcher, UsJurisdiction::Wyoming, "WHSAA").await;
    assert_eq!(record.status, "ok");
    assert_eq!(record.schools, Some(93));
    assert_eq!(record.with_address, Some(88));
    assert_eq!(record.pages, Some(1));
    assert_eq!(record.directory_total, Some(93));
    assert_eq!(record.sampled, Some(4));
    assert_eq!(record.staff, Some(44));
    assert_eq!(record.coaches, Some(13));

    let sports = record.sports.as_ref().unwrap();
    assert_eq!(sports.get("Track"), Some(&8));
    assert_eq!(sports.get("CrossCountry"), Some(&2));
    assert_eq!(sports.get("AthleticDirector"), Some(&3));

    assert_eq!(record.staff_per_school, Some(11.0));
    assert_eq!(record.coaches_per_school, Some(3.2));
}

#[tokio::test]
async fn offline_probe_ga_multi_page() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = make_offline_fetcher(dir.path());

    let body = fixture("coach_directories/probe/GA/directory-1.json");
    let url = format!("{API_HOST}/states/GHSA/directory/1");
    seed_cache(&fetcher, &url, 200, body.as_bytes());

    let summary_files = [
        "coach_directories/probe/GA/summary-4VHDT8.json",
        "coach_directories/probe/GA/summary-5T6GB4.json",
        "coach_directories/probe/GA/summary-UVERCT.json",
        "coach_directories/probe/GA/summary-ZF9KFM.json",
    ];
    let summary_codes = ["4VHDT8", "5T6GB4", "UVERCT", "ZF9KFM"];
    for (path, code) in summary_files.iter().zip(summary_codes.iter()) {
        let body = fixture(path);
        let url = format!("{API_HOST}/schools/{code}/summary");
        seed_cache(&fetcher, &url, 200, body.as_bytes());
    }

    let record = probe_one(&fetcher, UsJurisdiction::Georgia, "GHSA").await;
    assert_eq!(record.status, "ok");
    assert_eq!(record.schools, Some(1000));
    assert_eq!(record.with_address, Some(950));
    assert_eq!(record.pages, Some(3));
    assert_eq!(record.directory_total, Some(2825));
    assert_eq!(record.sampled, Some(4));
    assert_eq!(record.staff, Some(187));
    assert_eq!(record.coaches, Some(30));

    let sports = record.sports.as_ref().unwrap();
    assert_eq!(sports.get("Track"), Some(&11));
    assert_eq!(sports.get("CrossCountry"), Some(&16));
    assert_eq!(sports.get("AthleticDirector"), Some(&3));

    assert_eq!(record.staff_per_school, Some(46.8));
    assert_eq!(record.coaches_per_school, Some(7.5));
}

#[tokio::test]
async fn offline_uncached_association_records_offline() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = make_offline_fetcher(dir.path());

    let record = probe_one(&fetcher, UsJurisdiction::California, "CIF").await;
    assert_eq!(record.status, "offline");
    assert!(record.error.is_some());
}

#[tokio::test]
async fn access_denied_xml_body_surfaces_json_error() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = make_offline_fetcher(dir.path());

    let access_denied = fixture("coach_directories/summary_orgid_200_accessdenied.xml");
    seed_cache(
        &fetcher,
        &format!("{API_HOST}/states/TEST/directory/1"),
        200,
        access_denied.as_bytes(),
    );

    let record = probe_one(&fetcher, UsJurisdiction::Alabama, "TEST").await;
    assert_eq!(record.status, "json");
    assert!(record.error.is_some());
}

#[test]
fn sample_rows_empty() {
    let rows: Vec<super::parse::DirectorySchool> = vec![];
    assert_eq!(sample_rows(&rows).len(), 0);
}

#[test]
fn sample_rows_single() {
    let row = super::parse::DirectorySchool {
        short_code: Some("ABC123".to_string()),
        name: Some("Test High".to_string()),
        ..Default::default()
    };
    let rows = vec![row];
    let sampled = sample_rows(&rows);
    assert_eq!(sampled.len(), 1);
    assert_eq!(sampled[0].short_code.as_deref(), Some("ABC123"));
}

#[test]
fn sample_rows_five_step_one_take_four() {
    let mut rows = Vec::new();
    for i in 0..5 {
        rows.push(super::parse::DirectorySchool {
            short_code: Some(format!("S{i}")),
            name: Some(format!("School {i}")),
            ..Default::default()
        });
    }
    let sampled = sample_rows(&rows);
    assert_eq!(sampled.len(), 4);
    assert_eq!(sampled[0].short_code.as_deref(), Some("S0"));
    assert_eq!(sampled[1].short_code.as_deref(), Some("S1"));
    assert_eq!(sampled[2].short_code.as_deref(), Some("S2"));
    assert_eq!(sampled[3].short_code.as_deref(), Some("S3"));
}

#[test]
fn sample_rows_forty_step_ten() {
    let mut rows = Vec::new();
    for i in 0..40 {
        rows.push(super::parse::DirectorySchool {
            short_code: Some(format!("S{i:02}")),
            name: Some(format!("School {i}")),
            ..Default::default()
        });
    }
    let sampled = sample_rows(&rows);
    assert_eq!(sampled.len(), 4);
    assert_eq!(sampled[0].short_code.as_deref(), Some("S00"));
    assert_eq!(sampled[1].short_code.as_deref(), Some("S10"));
    assert_eq!(sampled[2].short_code.as_deref(), Some("S20"));
    assert_eq!(sampled[3].short_code.as_deref(), Some("S30"));
}

#[test]
fn classified_subset_is_preferred_for_sampling() {
    let mut rows = Vec::new();
    for i in 0..10 {
        let mut row = super::parse::DirectorySchool {
            short_code: Some(format!("N{i}")),
            name: Some(format!("No Levels {i}")),
            ..Default::default()
        };
        if i < 4 {
            row.competition_levels = BTreeMap::from([(
                "nchsaaClassification".to_string(),
                serde_json::Value::String("6A".to_string()),
            )]);
            row.short_code = Some(format!("L{i}"));
        }
        rows.push(row);
    }
    let classified: Vec<&super::parse::DirectorySchool> = rows
        .iter()
        .filter(|row| !row.competition_levels.is_empty())
        .collect();
    assert_eq!(classified.len(), 4);
    let sampled = sample_rows(&classified);
    assert_eq!(sampled.len(), 4);
    assert_eq!(sampled[0].short_code.as_deref(), Some("L0"));
    assert_eq!(sampled[1].short_code.as_deref(), Some("L1"));
    assert_eq!(sampled[2].short_code.as_deref(), Some("L2"));
    assert_eq!(sampled[3].short_code.as_deref(), Some("L3"));
}

#[test]
fn rows_without_short_code_are_skipped_in_summary_pass() {
    let row = super::parse::DirectorySchool {
        short_code: None,
        name: Some("No Code".to_string()),
        ..Default::default()
    };
    let rows = vec![row];
    let sampled = sample_rows(&rows);
    assert_eq!(sampled.len(), 1);
    assert!(sampled[0].short_code.is_none());
}

#[test]
fn staff_per_school_rounds_half_even() {
    assert_eq!(round_half_even(77, 4, 10), 19.2);
    assert_eq!(round_half_even(50, 4, 10), 12.5);
    assert_eq!(round_half_even(51, 4, 10), 12.8);
    assert_eq!(round_half_even(52, 4, 10), 13.0);
    assert_eq!(round_half_even(53, 4, 10), 13.2);
    assert_eq!(round_half_even(10, 1, 10), 10.0);
    assert_eq!(round_half_even(13, 4, 10), 3.2);
}

#[test]
fn report_json_sorts_by_state() {
    let records = vec![
        ProbeRecord {
            state: "NC".to_string(),
            ruleset: "NCHSAA".to_string(),
            status: "ok".to_string(),
            error: None,
            schools: Some(452),
            with_address: None,
            pages: Some(1),
            directory_total: None,
            sampled: None,
            staff: None,
            coaches: None,
            sports: None,
            staff_per_school: None,
            coaches_per_school: None,
        },
        ProbeRecord {
            state: "AL".to_string(),
            ruleset: "AHSAA".to_string(),
            status: "ok".to_string(),
            error: None,
            schools: Some(793),
            with_address: None,
            pages: Some(1),
            directory_total: None,
            sampled: None,
            staff: None,
            coaches: None,
            sports: None,
            staff_per_school: None,
            coaches_per_school: None,
        },
    ];
    let json = report_json(&records).expect("probe report");
    assert!(json.contains("AL"));
    assert!(json.contains("NC"));
    let al_pos = json.find("AL").unwrap();
    let nc_pos = json.find("NC").unwrap();
    assert!(al_pos < nc_pos, "AL must sort before NC");
}

#[test]
fn associations_table_has_51_entries() {
    assert_eq!(ASSOCIATIONS.len(), 51);
}

#[test]
fn verified_table_has_15_entries() {
    assert_eq!(VERIFIED.len(), 15);
}

#[test]
fn the_state_filter_selects_the_named_associations() {
    assert!(parse_state_filter("").is_empty());
    assert!(parse_state_filter(" , ,, ").is_empty());

    let wanted = parse_state_filter(" nc , al ,,");
    assert_eq!(
        wanted.iter().cloned().collect::<Vec<String>>(),
        vec!["AL".to_string(), "NC".to_string()]
    );
    assert_eq!(
        selected_associations(&wanted),
        vec![
            (UsJurisdiction::Alabama, "AHSAA"),
            (UsJurisdiction::NorthCarolina, "NCHSAA"),
        ]
    );
    assert_eq!(
        selected_associations(&parse_state_filter("DC")),
        vec![(UsJurisdiction::DistrictOfColumbia, "DCSAA")],
        "DC is filtered by its two-letter key"
    );
    assert_eq!(
        selected_associations(&BTreeSet::new()).len(),
        ASSOCIATIONS.len(),
        "an empty filter is every association"
    );
    assert!(selected_associations(&parse_state_filter("ZZ")).is_empty());
}

#[test]
fn the_prototype_table_lines_are_reproduced() {
    let text = fixture("coach_directories/probe/dragonfly_probe_records.json");
    let records: Vec<ProbeRecord> = serde_json::from_str(&text).expect("prototype probe records");
    assert_eq!(records.len(), 51);

    let table = fixture("coach_directories/probe/dragonfly_probe_table.txt");
    let expected: Vec<&str> = table.lines().collect();
    assert_eq!(expected.len(), records.len());

    let mut sorted = records.clone();
    super::survey::sort_records(&mut sorted);
    for (position, (record, want)) in sorted.iter().zip(expected).enumerate() {
        assert_eq!(table_line(record), want, "table line {position}");
    }
}

#[test]
fn the_probe_report_is_byte_identical_to_the_prototype_artifact() {
    let text = fixture("coach_directories/probe/dragonfly_probe_records.json");
    let records: Vec<ProbeRecord> = serde_json::from_str(&text).expect("prototype probe records");
    assert_eq!(
        report_json(&records).expect("report"),
        text,
        "census-prototype/out/dragonfly_probe.json, reproduced field for field and byte for byte"
    );
}

#[test]
fn every_verified_note_restates_the_committed_probe_artifact() {
    let text = fixture("coach_directories/probe/dragonfly_probe_records.json");
    let records: Vec<ProbeRecord> = serde_json::from_str(&text).expect("prototype probe records");
    for (state, schools, pages, note) in VERIFIED {
        let record = records
            .iter()
            .find(|record| record.state == state.code())
            .expect("a verified association carries a probe record");
        assert_eq!(record.status, "ok", "{}", state.code());
        assert_eq!(
            record.directory_total,
            Some(schools),
            "{} directory rows",
            state.code()
        );
        assert_eq!(record.pages, Some(pages), "{} pages", state.code());
        let staff = record.staff_per_school.expect("staff per school");
        let coaches = record.coaches_per_school.expect("coaches per school");
        assert_eq!(
            note,
            format!("{staff:.1} staff and {coaches:.1} census rows per sampled school"),
            "{}",
            state.code()
        );
    }
}

#[tokio::test]
async fn a_summary_failure_records_only_the_failure_like_the_prototype() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = make_offline_fetcher(dir.path());

    let body = fixture("coach_directories/probe/AK/directory-1.json");
    let url = format!("{API_HOST}/states/ASAA/directory/1");
    seed_cache(&fetcher, &url, 200, body.as_bytes());

    let record = probe_one(&fetcher, UsJurisdiction::Alaska, "ASAA").await;
    assert_eq!(record.status, "offline", "the first summary is not cached");
    assert!(record.error.is_some());
    assert_eq!(
        (record.schools, record.with_address, record.pages, record.directory_total, record.sampled),
        (None, None, None, None, None),
        "a failed association carries only state, ruleset, status and error, as the prototype's except branch writes"
    );
    assert_eq!(record.staff, None);
    assert_eq!(record.coaches, None);
    assert!(record.sports.is_none());
    let rendered = report_json(std::slice::from_ref(&record)).expect("report");
    let shape: serde_json::Value = serde_json::from_str(&rendered).expect("report parses");
    let mut keys: Vec<String> = shape[0]
        .as_object()
        .expect("record object")
        .keys()
        .cloned()
        .collect();
    keys.sort();
    assert_eq!(
        keys,
        ["error", "ruleset", "state", "status"],
        "a failed association carries only state, ruleset, status and error, as the prototype's except branch writes"
    );
}
