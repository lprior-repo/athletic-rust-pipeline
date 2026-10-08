#![recursion_limit = "256"]

#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

mod common;
#[path = "parity_pipeline_mod/milesplit_fixtures.rs"]
mod milesplit_fixtures;

use anyhow::{bail, ensure, Context, Result};
use census_crawl::{athleticnet, coach_contacts};
use census_domain::model::{CanonicalCoach, CanonicalSchool, CoachRole, Gender, Sport};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::collections::BTreeSet;

const OBSERVED_ON: &str = "2026-09-20";

#[test]
fn authentic_milesplit_projections_retain_owners_without_inventing_school_bindings() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(milesplit_fixtures::replay_owned_captures())
}

#[test]
fn coach_contacts_csv_preserves_published_people_and_deduplicates_replay() -> Result<()> {
    use CoachRole::{AthleticDirector, HeadCoach};
    use Gender::{Boys, Girls, Mixed};
    use Sport::{CrossCountry, OutdoorTrack};
    let path = common::fixtures_dir()?.join("coach_contacts_sample.csv");
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    let store = Store::open(&root)?;
    coach_contacts::import_csv(&store, &path, OBSERVED_ON)?;
    let schools = store.scan::<CanonicalSchool>(Table::Schools)?;
    let coaches = store.scan::<CanonicalCoach>(Table::Coaches)?;
    let published: BTreeSet<_> = coaches
        .iter()
        .map(|coach| {
            let owner = schools
                .iter()
                .find(|school| school.id == coach.school)
                .context("published school owner")?;
            Ok((
                owner.name.as_str(),
                coach.name.as_str(),
                coach.role,
                coach.sport,
                coach.gender,
                coach.professional_email.as_deref(),
            ))
        })
        .collect::<Result<_>>()?;
    let expected = BTreeSet::from([
        (
            "Abbotsford",
            "JACOB KNAPMILLER",
            HeadCoach,
            Some(OutdoorTrack),
            Boys,
            Some("jknapmiller@abbotsford.k12.wi.us"),
        ),
        (
            "Abbotsford",
            "JACOB KNAPMILLER",
            HeadCoach,
            Some(OutdoorTrack),
            Girls,
            Some("jknapmiller@abbotsford.k12.wi.us"),
        ),
        (
            "Abbotsford",
            "Dillon Novak",
            HeadCoach,
            Some(CrossCountry),
            Girls,
            Some("dnovak@abbotsford.k12.wi.us"),
        ),
        (
            "Abbotsford",
            "Alex Larson",
            AthleticDirector,
            None,
            Mixed,
            Some("alarson@abbotsford.k12.wi.us"),
        ),
        (
            "Abilene HS",
            "Derek Berns",
            AthleticDirector,
            None,
            Mixed,
            Some("dberns@abileneschools.org"),
        ),
        (
            "Adams Central",
            "Toni Fowler",
            HeadCoach,
            Some(CrossCountry),
            Boys,
            None,
        ),
        (
            "Adams Central",
            "Toni Fowler",
            HeadCoach,
            Some(CrossCountry),
            Girls,
            None,
        ),
        (
            "Adams Central",
            "Zeb Noyd",
            HeadCoach,
            Some(OutdoorTrack),
            Boys,
            None,
        ),
        (
            "Adams Central",
            "Alan Frank",
            AthleticDirector,
            None,
            Mixed,
            None,
        ),
        (
            "aberdeencentral",
            "Bo Beck",
            AthleticDirector,
            None,
            Mixed,
            None,
        ),
        (
            "Abingdon-Avon High School",
            "Barry Mink",
            HeadCoach,
            Some(CrossCountry),
            Boys,
            Some("bmink@atown276.net"),
        ),
        (
            "Abingdon-Avon High School",
            "Justin Rakestraw",
            HeadCoach,
            Some(OutdoorTrack),
            Boys,
            Some("jrakestraw@atown276.net"),
        ),
        (
            "Abingdon-Avon High School",
            "Reid Kelso",
            AthleticDirector,
            None,
            Mixed,
            Some("rkelso@atown276.net"),
        ),
    ]);
    ensure!(published == expected && coaches.len() == expected.len());
    ensure!(
        schools
            .iter()
            .any(|school| school.name == "East Kentwood HS"),
        "non-coaching administration still belongs to a retained school"
    );
    let physical = store.stats()?.tables;
    drop(store);
    let reopened = Store::open(&root)?;
    coach_contacts::import_csv(&reopened, &path, OBSERVED_ON)?;
    ensure!(reopened.scan::<CanonicalSchool>(Table::Schools)? == schools);
    ensure!(reopened.scan::<CanonicalCoach>(Table::Coaches)? == coaches);
    ensure!(reopened.stats()?.tables == physical);
    Ok(())
}

#[test]
fn registry_rejects_ambiguous_and_malformed_athlete_ownership() -> Result<()> {
    for (registry, states) in [
        (
            "28127170\n",
            vec![UsJurisdiction::Wisconsin, UsJurisdiction::Alaska],
        ),
        ("28127170,Alaska\n", Vec::new()),
        ("natalia\n", Vec::new()),
        ("28127170,AK,extra\n", Vec::new()),
    ] {
        if let Ok(parsed) = athleticnet::parse_targets(registry, &states) {
            bail!(
                "malformed registry {registry:?} admitted {} targets",
                parsed.len()
            );
        }
    }
    let targets = athleticnet::parse_targets(
        "28127170,AK\n26631105\n28127170,AK\n",
        &[UsJurisdiction::Wisconsin],
    )?;
    let owned: BTreeSet<_> = targets
        .iter()
        .map(|target| (target.athlete_id, target.state))
        .collect();
    ensure!(
        owned
            == BTreeSet::from([
                (28127170, Some(UsJurisdiction::Alaska)),
                (26631105, Some(UsJurisdiction::Wisconsin))
            ])
    );
    ensure!(targets.len() == owned.len());
    Ok(())
}

#[test]
fn synthetic_bio_preserves_string_numeric_empty_places_and_absent_scopes() -> Result<()> {
    let body = serde_json::json!({
        "dataFixture": "synthetic-bio-wire-consumer",
        "athlete": {"IDAthlete": 28127170, "FirstName": "Natalia", "LastName": "Casillas",
            "Gender": "F", "SchoolID": 13850},
        "grades": {}, "allTeams": {}, "allSeasons": [], "eventsTF": [], "meets": {},
        "resultsTF": [{"IDResult": 1, "Result": "26.10a", "Place": "3", "SchoolID": 13850,
            "EventID": 20, "MeetID": 589334, "SeasonID": 2026},
            {"IDResult": 2, "Result": "DNS", "Place": "", "SchoolID": 13850,
                "EventID": 20, "MeetID": 589334, "SeasonID": 2026}],
        "resultsXC": null
    });
    let track: athleticnet::Bio = serde_json::from_value(body.clone())?;
    let track_rows = track.results_tf.as_ref().context("track rows")?;
    ensure!(track_rows.first().and_then(|row| row.place.as_deref()) == Some("3"));
    ensure!(track_rows
        .get(1)
        .and_then(|row| row.place.as_deref())
        .is_none());
    ensure!(track.results_xc.is_none());
    let mut cross = body;
    cross["resultsXC"] = serde_json::json!([{"IDResult": 47122798, "Result": "25:31.2", "Place": 68,
        "SchoolID": 13850, "MeetID": 223703, "SeasonID": 2025, "Distance": 5000}]);
    let xc: athleticnet::Bio = serde_json::from_value(cross)?;
    let row = xc
        .results_xc
        .as_ref()
        .and_then(|rows| rows.first())
        .context("XC row")?;
    ensure!(row.place.as_deref() == Some("68") && row.distance == Some(5000));
    Ok(())
}
