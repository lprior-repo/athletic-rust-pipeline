use super::*;
use crate::registry::{transport_for_host, TransportKind};
use census_domain::core_scope::is_core_source;
use census_domain::model::CentiMetres;
use census_domain::model::CentiPoints;
use census_domain::model::ExactSeconds;
use census_domain::model::{EventKind, Mark, SourceNamespace};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn payload(body: &str) -> TestResult<Bio> {
    Ok(serde_json::from_str(body)?)
}

#[test]
fn every_acquisition_endpoint_is_browser_transported() -> TestResult {
    let mut urls = vec![BIO_ENDPOINT.to_string()];
    urls.extend(meet::meet_requests(2_150_205));
    urls.push(meet::METADATA_ENDPOINT.to_string());

    for url in urls {
        let parsed = url::Url::parse(&url)?;
        let host = parsed.host_str().ok_or("a request URL names a host")?;
        check!(eq;
            transport_for_host(host),
            Some(TransportKind::Browser),
            "{url} is acquired through the browser lane"
        );
    }
    Ok(())
}

#[test]
fn a_registry_line_carries_an_id_and_optionally_a_state() -> TestResult {
    let targets = parse_targets(
        "# season 2026\n28127170,AK\n\n26631105\n28127170,AK\n",
        &[UsJurisdiction::Wisconsin],
    )?;
    check!(eq;
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
    Ok(())
}

#[test]
fn a_registry_is_refused_rather_than_guessed_between_states() -> TestResult {
    let error = match parse_targets(
        "28127170\n",
        &[UsJurisdiction::Wisconsin, UsJurisdiction::Alaska],
    ) {
        Err(error) => error,
        Ok(_) => return Err("two candidate states are ambiguous".into()),
    };
    check!(
        error.to_string().contains("exactly one --states"),
        "the refusal names the fix: {error}"
    );
    check!(parse_targets("28127170,Alaska\n", &[]).is_err());
    check!(parse_targets("natalia\n", &[]).is_err());
    check!(parse_targets("28127170,AK,extra\n", &[]).is_err());
    Ok(())
}

#[test]
fn marks_are_read_in_the_form_athleticnet_publishes() -> TestResult {
    let time = parse_mark(&EventKind::Track800m, "1:17.80a").ok_or("an auto-timed time")?;
    check!(eq; time, (Mark::TimeSeconds(ExactSeconds::parse("77.80")?), true));
    check!(eq;
        parse_mark(&EventKind::Track3200m, "9:41.23"),
        Some((Mark::TimeSeconds(ExactSeconds::parse("581.23")?), false)),
        "a bare mark is hand-timed"
    );
    check!(eq;
        parse_mark(&EventKind::Track100m, "11.32q"),
        Some((Mark::TimeSeconds(ExactSeconds::parse("11.32")?), false)),
        "a qualifier suffix is not part of the mark"
    );
    check!(eq;
        parse_mark(&EventKind::LongJump, "5-04.25"),
        Some((
            Mark::FieldImperial {
                feet_mark: "5-04.25".to_string(),
                metres: CentiMetres::new(163),
            },
            false
        ))
    );
    check!(eq;
        parse_mark(&EventKind::ShotPut, "12.34m"),
        Some((Mark::DistanceMetres(CentiMetres::new(1234)), false)),
        "a metric field mark is metres, not a time"
    );
    check!(eq;
        parse_mark(&EventKind::Decathlon, "3,456"),
        Some((Mark::Points(CentiPoints::new(345600)), false))
    );
    Ok(())
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
fn cross_country_and_track_rows_deserialize_from_their_published_shapes() -> TestResult {
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
    )?;
    let rows = bio.results_tf.as_ref().ok_or("track rows")?;
    check!(eq; rows[0].place.as_deref(), Some("3"));
    check!(eq; rows[0].date().as_deref(), Some("2025-05-02"));
    check!(eq; rows[1].place, None, "an empty place is no place");
    check!(bio.results_xc.is_none(), "a null payload is no rows");
    check!(eq;
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
    )?;
    let xc_rows = xc.results_xc.as_ref().ok_or("cross-country rows")?;
    check!(eq; xc_rows[0].place.as_deref(), Some("68"));
    check!(eq; xc_rows[0].distance, Some(5000));
    Ok(())
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
