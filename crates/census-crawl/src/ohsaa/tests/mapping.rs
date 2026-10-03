use super::{
    capture, fixture_ad_centerville, fixture_ad_dublin, fixture_sports_centerville,
    fixture_sports_dublin, AD_FETCHED, SPORTS_FETCHED,
};
use crate::ohsaa::{school_entities, SearchResult};
use census_domain::model::{CoachRole, Evidence, Gender, SourceNamespace, Sport};

const DUBLIN_FACTS: [(&str, Option<Sport>, Gender, CoachRole); 5] = [
    (
        "Duane Sheldon",
        None,
        Gender::Mixed,
        CoachRole::AthleticDirector,
    ),
    (
        "Joe DePalma",
        Some(Sport::CrossCountry),
        Gender::Boys,
        CoachRole::HeadCoach,
    ),
    (
        "Greg King",
        Some(Sport::CrossCountry),
        Gender::Girls,
        CoachRole::HeadCoach,
    ),
    (
        "James Legins",
        Some(Sport::OutdoorTrack),
        Gender::Boys,
        CoachRole::HeadCoach,
    ),
    (
        "Greg King",
        Some(Sport::OutdoorTrack),
        Gender::Girls,
        CoachRole::HeadCoach,
    ),
];

fn dublin() -> SearchResult {
    SearchResult {
        name: "DUBLIN COFFMAN".to_string(),
        city: "Dublin".to_string(),
        ohsaa_id: "474".to_string(),
    }
}

fn assert_capture(evidence: &Evidence, capture: &crate::net::FetchOutcome) -> anyhow::Result<()> {
    check!(eq; evidence.source.id, "ohsaa_portal");
    check!(eq; evidence.source.url.as_deref(), Some(capture.url.as_str()));
    check!(eq; evidence.observed_on, capture.fetched_at);
    let note: serde_json::Value = serde_json::from_str(
        evidence
            .note
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("capture note absent"))?,
    )?;
    check!(eq; note["capture_url"], capture.url);
    check!(eq; note["sha256"], capture.content_digest);
    check!(eq; note["acquired_at"], capture.fetched_at);
    Ok(())
}

#[test]
fn cached_pages_keep_separate_capture_provenance_and_published_roles() -> anyhow::Result<()> {
    let sr = dublin();
    let sports = capture(sr.sports_url(), fixture_sports_dublin(), SPORTS_FETCHED);
    let ad = capture(sr.ad_url(), fixture_ad_dublin(), AD_FETCHED);
    let extract = school_entities(&sr, &sports, Some(&ad));
    check!(eq; extract.school.name, "DUBLIN COFFMAN");
    check!(eq; extract.school.city.as_deref(), Some("Dublin"));
    check!(eq; extract.school.association.as_deref(), Some("ohsaa"));
    check!(eq;
        extract.school.state,
        Some(census_domain::UsJurisdiction::Ohio)
    );
    check!(eq; extract.school.source_identities.len(), 1);
    let owner = &extract.school.source_identities[0];
    check!(eq;
        owner.namespace,
        SourceNamespace::association_school("ohsaa")
    );
    check!(eq; owner.id, "474");
    check!(eq; owner.url.as_deref(), Some(sr.page_url().as_str()));
    check!(eq; extract.school.evidence.len(), 1);
    assert_capture(&extract.school.evidence[0], &sports)?;
    check!(eq; extract.school.id, extract.school_id);
    let facts: Vec<_> = extract
        .coaches
        .iter()
        .map(|coach| (coach.name.as_str(), coach.sport, coach.gender, coach.role))
        .collect();
    check!(eq; facts, DUBLIN_FACTS);
    extract.coaches.iter().try_for_each(|coach| {
        check!(eq; coach.school, extract.school_id);
        check!(coach.source_identities.is_empty());
        check!(coach.tenure_evidence.is_empty());
        check!(eq; coach.evidence.len(), 1);
        let expected = if coach.role == CoachRole::AthleticDirector {
            &ad
        } else {
            &sports
        };
        assert_capture(&coach.evidence[0], expected)
    })?;
    Ok(())
}

#[test]
fn absent_ad_preserves_sports_without_creating_a_director() {
    let sr = dublin();
    let sports = capture(sr.sports_url(), fixture_sports_dublin(), SPORTS_FETCHED);
    let extract = school_entities(&sr, &sports, None);
    let names: Vec<_> = extract
        .coaches
        .iter()
        .map(|coach| coach.name.as_str())
        .collect();
    assert_eq!(
        names,
        ["Joe DePalma", "Greg King", "James Legins", "Greg King"]
    );
    assert!(extract
        .coaches
        .iter()
        .all(|coach| coach.role == CoachRole::HeadCoach));
}

#[test]
fn tba_coach_does_not_create_an_identity_or_mailbox() {
    let sr = SearchResult {
        name: "CENTERVILLE".to_string(),
        city: "Centerville".to_string(),
        ohsaa_id: "336".to_string(),
    };
    let sports = capture(
        sr.sports_url(),
        fixture_sports_centerville(),
        SPORTS_FETCHED,
    );
    let ad = capture(sr.ad_url(), fixture_ad_centerville(), AD_FETCHED);
    let extract = school_entities(&sr, &sports, Some(&ad));
    assert_eq!(extract.coaches.len(), 4);
    let girls_track = extract
        .coaches
        .iter()
        .find(|coach| coach.sport == Some(Sport::OutdoorTrack) && coach.gender == Gender::Girls);
    assert_eq!(girls_track, None);
}

#[test]
fn publisher_capture_urls_are_not_replaced_by_constructed_request_urls() -> anyhow::Result<()> {
    let sr = dublin();
    let sports = capture(
        format!("{}&view=public", sr.sports_url()),
        fixture_sports_dublin(),
        SPORTS_FETCHED,
    );
    let ad = capture(
        format!("{}&view=public", sr.ad_url()),
        fixture_ad_dublin(),
        AD_FETCHED,
    );
    let extract = school_entities(&sr, &sports, Some(&ad));
    assert_capture(&extract.school.evidence[0], &sports)?;
    extract.coaches.iter().try_for_each(|coach| {
        let expected = if coach.role == CoachRole::AthleticDirector {
            &ad
        } else {
            &sports
        };
        assert_capture(&coach.evidence[0], expected)
    })
}
