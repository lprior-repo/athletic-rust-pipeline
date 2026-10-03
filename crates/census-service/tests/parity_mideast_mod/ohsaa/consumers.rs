use anyhow::Context;
use census_crawl::ohsaa;
use census_domain::model::{
    CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SourceIdentity, SourceNamespace,
    Sport,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

use super::super::{common, context, seeded, OBSERVED_ON, SEEDED_AT};

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

fn ohsaa_capture(
    url: String,
    file: &str,
    fetched_at: &str,
) -> Result<census_crawl::net::FetchOutcome> {
    use sha2::{Digest, Sha256};
    let body = common::fixture("ohsaa", file)?.into_bytes();
    Ok(census_crawl::net::FetchOutcome {
        url,
        response_url: None,
        method: "GET".to_string(),
        status: 200,
        content_digest: format!("{:x}", Sha256::digest(&body)),
        bytes: body.len(),
        fetched_at: fetched_at.to_string(),
        from_cache: true,
        content_type: Some("text/html".to_string()),
        body,
    })
}

fn assert_ohsaa_capture(
    evidence: &Evidence,
    capture: &census_crawl::net::FetchOutcome,
) -> Result<()> {
    check!(eq; evidence.source.id, "ohsaa_portal");
    check!(eq; evidence.source.url.as_deref(), Some(capture.url.as_str()));
    check!(eq; evidence.observed_on, capture.fetched_at);
    let note: serde_json::Value = serde_json::from_str(
        evidence
            .note
            .as_deref()
            .context("the OHSAA capture receipt")?,
    )?;
    check!(eq; note["capture_url"], capture.url);
    check!(eq; note["sha256"], capture.content_digest);
    check!(eq; note["acquired_at"], capture.fetched_at);
    Ok(())
}

type PublishedPerson<'a> = (
    &'a str,
    CoachRole,
    Option<Sport>,
    Gender,
    Option<&'a str>,
    Option<&'a str>,
);

const COFFMAN_PEOPLE: [PublishedPerson<'static>; 5] = [
    (
        "Duane Sheldon",
        CoachRole::AthleticDirector,
        None,
        Gender::Mixed,
        Some("sheldon_duane@dublinschools.net"),
        None,
    ),
    (
        "Joe DePalma",
        CoachRole::HeadCoach,
        Some(Sport::CrossCountry),
        Gender::Boys,
        Some("depalma_joseph@dublinschools.net"),
        None,
    ),
    (
        "Greg King",
        CoachRole::HeadCoach,
        Some(Sport::CrossCountry),
        Gender::Girls,
        Some("king_greg@dublinschools.net"),
        None,
    ),
    (
        "James Legins",
        CoachRole::HeadCoach,
        Some(Sport::OutdoorTrack),
        Gender::Boys,
        None,
        Some("j.legins106@gmail.com"),
    ),
    (
        "Greg King",
        CoachRole::HeadCoach,
        Some(Sport::OutdoorTrack),
        Gender::Girls,
        Some("king_greg@dublinschools.net"),
        None,
    ),
];

fn assert_coffman_consumers(
    school: &CanonicalSchool,
    coaches: &[CanonicalCoach],
    sports: &census_crawl::net::FetchOutcome,
    ad: &census_crawl::net::FetchOutcome,
) -> Result<()> {
    check!(eq; school.name, "DUBLIN COFFMAN");
    check!(eq; school.city.as_deref(), Some("Dublin"));
    check!(eq; school.state, Some(UsJurisdiction::Ohio));
    check!(eq; school.association.as_deref(), Some("ohsaa"));
    check!(eq; school.source_identities,
    vec![
        SourceIdentity::new(SourceNamespace::association_school("ohsaa"), "474")
            .with_url("https://officials.myohsaa.org/Outside/Schedule?ohsaaId=474")
    ]);
    assert_ohsaa_capture(school.evidence.first().context("school capture")?, sports)?;
    let mut published: Vec<_> = coaches
        .iter()
        .map(|coach| {
            (
                coach.name.as_str(),
                coach.role,
                coach.sport,
                coach.gender,
                coach.professional_email.as_deref(),
                coach.personal_email.as_deref(),
            )
        })
        .collect();
    published.sort_unstable();
    let mut expected = COFFMAN_PEOPLE;
    expected.sort_unstable();
    check!(eq; published, expected);
    coaches.iter().try_for_each(|coach| {
        check!(eq; coach.school, school.id, "owner of {}", coach.name);
        check!(
            coach.source_identities.is_empty(),
            "invented person ID for {}",
            coach.name
        );
        let capture = if coach.role == CoachRole::AthleticDirector {
            ad
        } else {
            sports
        };
        assert_ohsaa_capture(coach.evidence.first().context("person capture")?, capture)
    })
}

#[test]
fn ohsaa_coffman_consumers_receive_published_people_with_capture_provenance() -> Result<()> {
    let rows = ohsaa::parse_search(&common::fixture("ohsaa", "search_dublin_coffman.html")?);
    let result = rows
        .first()
        .context("the search fixture carries Dublin Coffman")?;
    let sports = ohsaa_capture(result.sports_url(), "sports_dublin_coffman.html", SEEDED_AT)?;
    let ad = ohsaa_capture(
        result.ad_url(),
        "ad_dublin_coffman.html",
        "2026-09-20T14:40:00Z",
    )?;
    let extract = ohsaa::school_entities(result, &sports, Some(&ad));
    assert_coffman_consumers(&extract.school, &extract.coaches, &sports, &ad)
}

#[test]
fn ohsaa_seeded_collection_persists_published_people_and_school_ownership() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let rows =
                ohsaa::parse_search(&common::fixture("ohsaa", "search_dublin_coffman.html")?);
            let result = rows
                .first()
                .context("the search fixture carries Dublin Coffman")?;
            let search_url = format!("{}/Outside/SearchSchool?Name=Dublin%20Coffman", ohsaa::HOST);

            let dir = tempfile::tempdir().context("temp dir")?;
            let (store, fetcher) = seeded(
                dir.path(),
                &[
                    (
                        &search_url,
                        &common::fixture("ohsaa", "search_dublin_coffman.html")?,
                    ),
                    (
                        &result.sports_url(),
                        &common::fixture("ohsaa", "sports_dublin_coffman.html")?,
                    ),
                    (
                        &result.ad_url(),
                        &common::fixture("ohsaa", "ad_dublin_coffman.html")?,
                    ),
                ],
            )?;
            let fetcher = fetcher.with_offline(true);
            let options = ohsaa::Options {
                limit: None,
                refresh: false,
                observed_on: OBSERVED_ON.to_string(),
                states: vec![UsJurisdiction::Ohio],
                school_names: vec!["Dublin Coffman".to_string()],
            };
            let report = ohsaa::collect(&context(&fetcher, &store), &options).await?;

            check!(eq; (report.requests, report.from_cache), (0, 3));
            check!(eq; (report.rows, report.errors, report.with_email), (1, 0, 5));
            store.flush()?;
            drop(store);
            let store =
                Store::open(dir.path().join("store")).context("reopening persisted OHSAA facts")?;
            check!(eq; store.walk_table(Table::Schools)?.rows, 1);
            check!(eq; store.walk_table(Table::Coaches)?.rows, 5);
            let schools: Vec<CanonicalSchool> = store.scan(Table::Schools)?;
            let coaches: Vec<CanonicalCoach> = store.scan(Table::Coaches)?;
            check!(eq; schools.len(), 1);
            let school = schools.first().context("persisted Dublin Coffman")?;
            let sports =
                ohsaa_capture(result.sports_url(), "sports_dublin_coffman.html", SEEDED_AT)?;
            let ad = ohsaa_capture(result.ad_url(), "ad_dublin_coffman.html", SEEDED_AT)?;
            assert_coffman_consumers(school, &coaches, &sports, &ad)
        })
}
