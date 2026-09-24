use super::*;
use crate::registry::{transport_for_host, TransportKind};
use census_domain::core_scope::is_core_source;
use census_domain::model::{EventKind, Mark, SourceNamespace};
use census_domain::model::CentiPoints;
use census_domain::model::CentiMetres;
use census_domain::model::CentiSeconds;

fn payload(body: &str) -> Bio {
    serde_json::from_str(body).expect("a payload this adapter reads")
}

/// Every endpoint this adapter acquires through is a host the registry routes to the browser lane.
///
/// The transport is the registry's declaration and not the caller's request — `execute` reads
/// `transport_for_host` for every URL, and a host no descriptor claims keeps the plain HTTP path — so
/// this test is what keeps a direct-HTTP request from creeping back into the route. Both the
/// constants and the URLs the route actually builds are checked, because a route that spelled its own
/// URL would route by that spelling's host and not by these constants.
#[test]
fn every_acquisition_endpoint_is_browser_transported() {
    let mut urls = vec![BIO_ENDPOINT.to_string()];
    urls.extend(meet::meet_requests(2_150_205));
    urls.push(meet::METADATA_ENDPOINT.to_string());

    for url in urls {
        let parsed = url::Url::parse(&url).expect("the adapter's own URL parses");
        let host = parsed.host_str().expect("a request URL names a host");
        assert_eq!(
            transport_for_host(host),
            Some(TransportKind::Browser),
            "{url} is acquired through the browser lane"
        );
    }
}

#[test]
fn a_registry_line_carries_an_id_and_optionally_a_state() {
    let targets = parse_targets(
        "# season 2026\n28127170,AK\n\n26631105\n28127170,AK\n",
        &[UsJurisdiction::Wisconsin],
    )
    .expect("registry parses");
    assert_eq!(
        targets,
        vec![
            Target {
                athlete_id: 28127170,
                state: Some(UsJurisdiction::Alaska)
            },
            Target {
                athlete_id: 26631105,
                state: Some(UsJurisdiction::Wisconsin)
            },
        ],
        "the per-line state wins, the default fills a bare id, and a repeat reads once"
    );
}

#[test]
fn a_registry_is_refused_rather_than_guessed_between_states() {
    let error = parse_targets(
        "28127170\n",
        &[UsJurisdiction::Wisconsin, UsJurisdiction::Alaska],
    )
    .expect_err("two candidate states are ambiguous");
    assert!(
        error.to_string().contains("exactly one --states"),
        "the refusal names the fix: {error}"
    );
    assert!(parse_targets("28127170,Alaska\n", &[]).is_err());
    assert!(parse_targets("natalia\n", &[]).is_err());
    assert!(parse_targets("28127170,AK,extra\n", &[]).is_err());
}

#[test]
fn marks_are_read_in_the_form_athleticnet_publishes() {
    let time = parse_mark(&EventKind::Track800m, "1:17.80a").expect("an auto-timed time");
    assert_eq!(time, (Mark::TimeSeconds(CentiSeconds(7780)), true));
    assert_eq!(
        parse_mark(&EventKind::Track3200m, "9:41.23"),
        Some((Mark::TimeSeconds(CentiSeconds(58123)), false)),
        "a bare mark is hand-timed"
    );
    assert_eq!(
        parse_mark(&EventKind::Track100m, "11.32q"),
        Some((Mark::TimeSeconds(CentiSeconds(1132)), false)),
        "a qualifier suffix is not part of the mark"
    );
    assert_eq!(
        parse_mark(&EventKind::LongJump, "5-04.25"),
        Some((
            Mark::FieldImperial {
                feet_mark: "5-04.25".to_string(),
                metres: CentiMetres(163),
            },
            false
        ))
    );
    assert_eq!(
        parse_mark(&EventKind::ShotPut, "12.34m"),
        Some((Mark::DistanceMetres(CentiMetres(1234)), false)),
        "a metric field mark is metres, not a time"
    );
    assert_eq!(
        parse_mark(&EventKind::Decathlon, "3,456"),
        Some((Mark::Points(CentiPoints(345600)), false))
    );
}

#[test]
fn a_no_mark_row_yields_no_performance() {
    for published in ["DNS", "ND", "FOUL", "X", ""] {
        assert_eq!(
            parse_mark(&EventKind::Track1600m, published),
            None,
            "`{published}` is not a mark"
        );
    }
}

#[test]
fn cross_country_and_track_rows_deserialize_from_their_published_shapes() {
    // One TF row (string place, event id, result date), one XC row (numeric place, distance,
    // no result date), plus a null `resultsXC` — the three variations observed live.
    let bio = payload(
        r#"{
              "athlete": {"IDAthlete": 28127170, "FirstName": "Natalia", "LastName": "Casillas",
                          "Gender": "F", "SchoolID": 13850},
              "grades": {"13850_2026": 11},
              "allTeams": {"13850": {"IDSchool": 13850, "SchoolName": "Seton Catholic"}},
              "allSeasons": [
                 {"SchoolID": 13850, "IDSeason": 2026, "Display": "2026 Indoor"},
                 {"SchoolID": 13850, "IDSeason": 2026, "Display": "2026 Outdoor"}
              ],
              "eventsTF": [{"IDEvent": 20, "Event": "200 Meters"}],
              "meets": {"589334": {"MeetName": "SOHI Invite", "EndDate": "2025-05-03T00:00:00"}},
              "resultsTF": [
                {"IDResult": 1, "Result": "26.10a", "FAT": 1, "Place": "3", "Round": "F",
                 "Wind": 1.2, "Division": "Varsity", "SchoolID": 13850, "EventID": 20,
                 "MeetID": 589334, "SeasonID": 2026, "ResultDate": "2025-05-02T00:00:00"},
                {"IDResult": 2, "Result": "DNS", "FAT": 0, "Place": "", "Round": "P",
                 "Division": "Varsity", "SchoolID": 13850, "EventID": 20,
                 "MeetID": 589334, "SeasonID": 2026, "ResultDate": "2025-05-02T00:00:00"}
              ],
              "resultsXC": null
            }"#,
    );
    let rows = bio.results_tf.as_ref().expect("track rows");
    assert_eq!(rows[0].place.as_deref(), Some("3"));
    assert_eq!(rows[0].date().as_deref(), Some("2025-05-02"));
    assert_eq!(rows[1].place, None, "an empty place is no place");
    assert!(bio.results_xc.is_none(), "a null payload is no rows");
    assert_eq!(
        bio.athlete.name(),
        "Natalia Casillas",
        "the canonical name is the published one"
    );

    let xc = payload(
        r#"{
              "athlete": {"IDAthlete": 28127170, "FirstName": "Natalia", "LastName": "Casillas",
                          "Gender": "F", "SchoolID": 13850},
              "grades": {"13850_2026": 11},
              "allTeams": {"13850": {"IDSchool": 13850, "SchoolName": "Seton Catholic"}},
              "allSeasons": [],
              "meets": {"223703": {"MeetName": "Chandler Invitational", "EndDate": "2025-09-02T00:00:00"}},
              "resultsXC": [
                {"IDResult": 47122798, "Result": "25:31.2", "Place": 68, "Division": "Varsity",
                 "SchoolID": 13850, "MeetID": 223703, "SeasonID": 2025, "Distance": 5000}
              ]
            }"#,
    );
    let xc_rows = xc.results_xc.as_ref().expect("cross-country rows");
    assert_eq!(xc_rows[0].place.as_deref(), Some("68"));
    assert_eq!(xc_rows[0].distance, Some(5000));
}

#[test]
fn the_athleticnet_namespace_is_outside_the_core_scope() {
    let namespace = SourceNamespace::AthleticNet {
        kind: "athlete".to_string(),
    };
    assert!(
        !namespace.is_core(),
        "the host is not the platform's own source"
    );
    assert!(!is_core_source("athleticnet"));
    assert_eq!(namespace.to_string(), "athleticnet:athlete");
}
