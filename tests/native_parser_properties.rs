#![forbid(unsafe_code)]

use anyhow::Result;
use proptest::{
    prelude::*,
    test_runner::{RngAlgorithm, RngSeed},
};
use rust_xlsxwriter::Workbook;
use serde_json::json;
use std::{fs::File, io::Write, path::Path};
use tempfile::tempdir;

use athletic_rust_pipeline::{
    domain::{
        evidence::{BestClaim, ResultAttribution, Sport},
        identity::{AthleteId, EvidenceDigest, SourceRowKey},
        marks::{EventName, MarkValue, Performance, PerformanceState},
    },
    model::SOURCE_HEADERS,
    profile::{parse_bio, parse_profile_html},
    search::{parse_page, SearchQuery},
    workbook_ingest, xlsx,
};

const ATHLETE_ID: u64 = 123;
const DIGEST_TEXT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn athlete() -> AthleteId {
    AthleteId::new(ATHLETE_ID)
        .unwrap_or_else(|error| panic!("fixed test athlete is valid: {error}"))
}

fn digest() -> EvidenceDigest {
    EvidenceDigest::parse(DIGEST_TEXT)
        .unwrap_or_else(|error| panic!("fixed test digest is valid: {error}"))
}

fn ascii_word() -> impl Strategy<Value = String> {
    prop::collection::vec(97_u8..=122_u8, 1..=20)
        .prop_map(|bytes| String::from_utf8(bytes).unwrap_or_else(|_| String::from("synthetic")))
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x4E41_5449_5645_5052),
        .. ProptestConfig::default()
    })]

    #[test]
    fn search_envelope_preserves_numeric_count_and_next_offset(
        name in ascii_word(),
        advertised in 0_u32..=10_000,
        start in 0_u32..=1_000,
        distance in 1_u32..=1_000,
    ) {
        let next = start.saturating_add(distance);
        let query = SearchQuery::new("Ada Lovelace", Sport::TrackField, 0)
            .unwrap_or_else(|error| panic!("fixed query is valid: {error}"));
        let html = format!(
            "<table><tr><td><a href=\"/athlete/123/track-and-field\">{name}</a> synthetic</td></tr></table>"
        );
        let envelope = json!({
            "d": {
                "results": html,
                "count": advertised.to_string(),
                "pager": format!("<a data-start=\"{next}\">next</a><a data-start=\"{start}\">current</a>"),
                "runTime": advertised
            }
        });
        let parsed = parse_page(&query, start, digest(), envelope.to_string().as_bytes())
            .unwrap_or_else(|error| panic!("synthetic search envelope parses: {error}"));
        prop_assert_eq!(parsed.advertised_count(), advertised);
        prop_assert_eq!(parsed.requested_count as usize, parsed.results().len());
        prop_assert_eq!(parsed.next_offset, Some(next));
        prop_assert_eq!(parsed.results()[0].id().get(), ATHLETE_ID);
        prop_assert_eq!(parsed.results()[0].name(), name.as_str());
        prop_assert_eq!(parsed.results()[0].sport, Sport::TrackField);
        prop_assert_eq!(parsed.results()[0].evidence.document.as_str(), DIGEST_TEXT);
    }

    #[test]
    fn profile_html_keeps_only_requested_athlete_cohort(
        year in 1900_u16..=2200,
        name in ascii_word(),
    ) {
        let state = json!({"tree": [{"type": "athlete", "id": ATHLETE_ID, "title": name.clone()}]});
        let body = format!(
            "<link rel=\"canonical\" href=\"https://www.athletic.net/athlete/{ATHLETE_ID}/track-and-field/all\">\
             <script>window.anetSiteAppParams={state};</script>\
             <div data-athlete-id=\"{ATHLETE_ID}\">Class of {year}\
               <div data-athlete-id=\"999\">Class of 2027</div>\
               <span hidden>Class of 2027</span><script>Class of 2027</script>\
             </div>"
        );
        let parsed = parse_profile_html(athlete(), digest(), body.as_bytes())
            .unwrap_or_else(|error| panic!("synthetic profile HTML parses: {error}"));
        prop_assert_eq!(
            parsed.cohort_witnesses.iter().map(|witness| witness.value.get()).collect::<Vec<_>>(),
            vec![year]
        );
        prop_assert_eq!(parsed.identity_hints.len(), 1);
        prop_assert_eq!(parsed.identity_hints[0].value.as_str(), name.as_str());
        prop_assert!(parsed.issues.is_empty());
    }

    #[test]
    fn bio_track_numeric_best_flags_remain_opaque(
        flags in 0_u64..=u64::from(u16::MAX),
        fat in 0_u64..=3,
    ) {
        let body = json!({
            "athlete": {"IDAthlete": ATHLETE_ID, "FirstName": "Synthetic", "LastName": "Runner"},
            "allSeasons": [{"SchoolID": 7, "IDSeason": 2025}],
            "allTeams": {"7": {"SchoolName": "Fictional High", "Level": 4}},
            "meets": {"9": {"MeetName": "Synthetic Meet"}},
            "eventsTF": [{"IDEvent": 1, "Event": "100 Meters", "PersonalEvent": true}],
            "resultsTF": [{
                "IDResult": 11, "AthleteID": ATHLETE_ID, "Result": "10.72a", "SchoolID": 7,
                "MeetID": 9, "SeasonID": 2025, "EventID": 1, "PersonalBest": flags,
                "SeasonBest": flags, "FAT": fat, "shortCode": "synthetic-result"
            }]
        });
        let parsed = parse_bio(athlete(), Sport::TrackField, digest(), body.to_string().as_bytes())
            .unwrap_or_else(|error| panic!("synthetic bio parses: {error}"));
        prop_assert_eq!(parsed.results.len(), 1);
        prop_assert_eq!(&parsed.results[0].personal_best, &BestClaim::OpaqueFlags(flags));
        prop_assert_eq!(&parsed.results[0].season_best, &BestClaim::OpaqueFlags(flags));
        prop_assert_eq!(&parsed.results[0].attribution, &ResultAttribution::Individual);
        let expected_timing = match fat {
            0 => Some(String::from("hand")),
            1 => Some(String::from("FAT")),
            _ => Some(fat.to_string()),
        };
        prop_assert_eq!(&parsed.results[0].timing, &expected_timing);
    }

    #[test]
    fn bio_cross_country_boolean_best_and_foreign_identity_are_not_conflated(
        best in any::<bool>(),
        foreign_id in 1_u64..=10_000,
    ) {
        let requested = athlete();
        let reported = if foreign_id == ATHLETE_ID { ATHLETE_ID + 1 } else { foreign_id };
        let body = json!({
            "athlete": {"IDAthlete": ATHLETE_ID, "FirstName": "Synthetic", "LastName": "Runner"},
            "allTeams": {"7": {"SchoolName": "Fictional High"}},
            "meets": {"9": {"MeetName": "Synthetic Meet"}},
            "distancesXC": [{"Meters": 5000, "Distance": 5, "Units": "m"}],
            "resultsXC": [{
                "IDResult": 11, "AthleteID": reported, "Result": "17:01", "SchoolID": 7,
                "MeetID": 9, "SeasonID": 2025, "Distance": 5000, "PersonalBest": best,
                "SeasonBest": best, "shortCode": "synthetic-xc"
            }]
        });
        let parsed = parse_bio(requested, Sport::CrossCountry, digest(), body.to_string().as_bytes())
            .unwrap_or_else(|error| panic!("synthetic XC bio parses: {error}"));
        let expected = if best { BestClaim::Claimed } else { BestClaim::NotClaimed };
        prop_assert_eq!(&parsed.results[0].personal_best, &expected);
        prop_assert!(matches!(parsed.results[0].attribution, ResultAttribution::Unresolved { reported_athlete_id: Some(id) } if id == reported),
            "foreign result identity must remain unresolved");
    }

    #[test]
    fn performance_deserialization_rejects_invalid_serialized_state(milliseconds in 1_u64..=3_600_000) {
        let raw = format!("{}.{:03}s", milliseconds / 1_000, milliseconds % 1_000);
        let boundary = json!({"event": "100m", "raw": raw, "state": "Dns"});
        prop_assert!(serde_json::from_value::<Performance>(boundary).is_err());
    }

    #[test]
    fn performance_time_and_points_keep_distinct_numeric_domains(
        milliseconds in 1_u64..=3_600_000,
        points in 1_u64..=100_000,
    ) {
        let time_event = EventName::parse("100m").unwrap_or_else(|error| panic!("100m is valid: {error}"));
        let time_raw = format!("{}.{:03}s", milliseconds / 1_000, milliseconds % 1_000);
        let time = Performance::parse(&time_event, &time_raw)
            .unwrap_or_else(|error| panic!("generated time is valid: {error}"));
        prop_assert!(matches!(time.state(), PerformanceState::Recorded(MarkValue::Time(value)) if value.get() == milliseconds));

        let points_event = EventName::parse("decathlon").unwrap_or_else(|error| panic!("decathlon is valid: {error}"));
        let points_value = Performance::parse(&points_event, &points.to_string())
            .unwrap_or_else(|error| panic!("generated points are valid: {error}"));
        prop_assert!(matches!(points_value.state(), PerformanceState::Recorded(MarkValue::Points(value)) if value.get() == points));
    }

    #[test]
    fn source_row_keys_round_trip_without_identity_loss(
        row in 2_u32..=1_000_000,
        suffix in ascii_word(),
    ) {
        let raw = format!("Synthetic {suffix}:{row}");
        let key = SourceRowKey::parse(&raw).unwrap_or_else(|error| panic!("generated source key is valid: {error}"));
        let encoded = serde_json::to_vec(&key).unwrap_or_else(|error| panic!("source key serializes: {error}"));
        let restored = serde_json::from_slice::<SourceRowKey>(&encoded)
            .unwrap_or_else(|error| panic!("source key deserializes: {error}"));
        prop_assert_eq!(restored.as_str(), raw.as_str());
        let expected_sheet = format!("Synthetic {suffix}");
        prop_assert_eq!(restored.sheet(), expected_sheet.as_str());
        prop_assert_eq!(restored.row(), row);
    }
}

#[test]
fn malformed_and_sparse_ooxml_are_rejected_by_both_ingestion_boundaries() -> Result<()> {
    let directory = tempdir()?;
    [
        ("malformed.xlsx", b"not an OOXML package".as_slice()),
        (
            "sparse.xlsx",
            b"<worksheet><sheetData><row r=\"1\"/></sheetData></worksheet>".as_slice(),
        ),
    ]
    .into_iter()
    .try_for_each(|(name, bytes)| {
        let path = directory.path().join(name);
        File::create(&path)?.write_all(bytes)?;
        assert!(xlsx::visit_records(&path, |_record| Ok::<(), anyhow::Error>(())).is_err());
        assert!(
            workbook_ingest::visit_records(&path, |_record| Ok::<(), anyhow::Error>(())).is_err()
        );
        Ok::<(), anyhow::Error>(())
    })
}

#[test]
fn synthetic_workbook_ingestion_preserves_source_row_identity_and_fields() -> Result<()> {
    let directory = tempdir()?;
    let path = directory.path().join("synthetic.xlsx");
    write_source_workbook(&path, "Generated Runner")?;

    let mut xlsx_records = Vec::new();
    let xlsx_stats = xlsx::visit_records(&path, |record| {
        xlsx_records.push(record);
        Ok::<(), anyhow::Error>(())
    })?;
    assert_eq!(xlsx_stats.actual_data_rows, 1);
    assert_eq!(xlsx_records.len(), 1);
    assert_eq!(xlsx_records[0].source_key, "Synthetic:2");
    assert_eq!(xlsx_records[0].sheet, "Synthetic");
    assert_eq!(xlsx_records[0].excel_row, 2);
    assert_eq!(
        xlsx_records[0]
            .fields
            .get("Person First")
            .map(String::as_str),
        Some("Generated Runner")
    );

    let mut stream_records = Vec::new();
    let stream_stats = workbook_ingest::visit_records(&path, |record| {
        stream_records.push(record);
        Ok::<(), anyhow::Error>(())
    })?;
    assert_eq!(stream_stats.actual_data_rows, 1);
    assert_eq!(stream_records.len(), 1);
    assert_eq!(stream_records[0].source_key, "Synthetic:2");
    assert_eq!(
        stream_records[0]
            .fields
            .get("Person Email")
            .map(String::as_str),
        Some("synthetic@example.invalid")
    );
    Ok(())
}

fn write_source_workbook(path: &Path, first: &str) -> Result<()> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet().set_name("Synthetic")?;
    SOURCE_HEADERS
        .iter()
        .enumerate()
        .try_for_each(|(column, header)| {
            let column = u16::try_from(column)?;
            worksheet
                .write_string(0, column, *header)
                .map(|_| ())
                .map_err(anyhow::Error::from)
        })?;
    let values = [
        first,
        "Runner",
        "synthetic@example.invalid",
        "100 Fictional Way",
        "Fictional City",
        "ZZ",
        "00000",
        "2026-01-01",
        "Track",
        "0",
        "2026-01-01",
        "synthetic-fixture",
        "Fictional High",
    ];
    values.iter().enumerate().try_for_each(|(column, value)| {
        let column = u16::try_from(column)?;
        worksheet
            .write_string(1, column, *value)
            .map(|_| ())
            .map_err(anyhow::Error::from)
    })?;
    workbook.save(path).map_err(anyhow::Error::from)
}
