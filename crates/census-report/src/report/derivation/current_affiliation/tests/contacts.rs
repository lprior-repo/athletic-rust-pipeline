use super::*;
use crate::export::selected_test_fixture::{hash, retain, Fixture as SelectedGraphFixture};
use census_crawl::net::FetchOutcome;
use census_domain::model::{
    CanonicalCoach, CanonicalSchool, CanonicalTeam, CoachContactClaim, CoachContactProgram,
    CoachRole, CoachTenure, CoachTenureEvidence, Evidence, Gender, SchoolPostalAddress, SchoolYear,
    SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::school_directory::{PostalAddress, SourceLabel, StreetLine};
use census_store::{Store, Table};
use serde::Deserialize;
use std::path::Path;

fn fixture_with_contacts() -> TestResult<Fixture> {
    let mut fixture = Fixture::new()?;
    fixture
        .dataset
        .athletes
        .first_mut()
        .ok_or("fixture athlete")?
        .sports = vec![Sport::OutdoorTrack];
    for (school, name, mailbox, street, key) in [
        (
            &fixture.previous,
            "Previous coach",
            "previous@school.test",
            "1 Previous Road",
            "a",
        ),
        (
            &fixture.current,
            "Current coach",
            "current@school.test",
            "2 Current Road",
            "b",
        ),
    ] {
        let coach = qualified_coach(school, name, mailbox, key)?;
        fixture.dataset.coaches.push(coach.clone());
        fixture.dataset.coach_observations.push(coach);
        add_address(
            fixture
                .dataset
                .schools
                .get_mut(school)
                .ok_or("fixture school")?,
            street,
            key,
        )?;
    }
    fixture.dataset.bind_selection_for_test(
        ProfileId::parse(&"a".repeat(64))?,
        ProfileDate::parse("2026-10-01")?,
    )?;
    fixture.dataset.bind_participation_rule_for_test(
        census_domain::participation_candidates::participation_policy_v2()?,
    )?;
    Ok(fixture)
}

fn qualified_coach(
    school: &SchoolId,
    name: &str,
    mailbox: &str,
    key: &str,
) -> TestResult<CanonicalCoach> {
    let mut coach = CanonicalCoach::new(
        school,
        name,
        Some(Sport::OutdoorTrack),
        Gender::Girls,
        CoachRole::HeadCoach,
    );
    coach.professional_email = Some(mailbox.into());
    let source = SourceRef::new(
        "fixture_contacts",
        Some(format!("https://school.test/coaches/{key}")),
    );
    coach.evidence = vec![Evidence::parsed(source.clone(), "2026-09-20")];
    coach.tenure_evidence = vec![CoachTenureEvidence {
        tenure: CoachTenure::Current {
            school_year: SchoolYear::new(2026).ok_or("fixture school year")?,
        },
        source,
        source_sha256: key.repeat(64),
        retrieved_at: "2026-09-20T00:00:00Z".into(),
        statement: "2026-27 girls outdoor track head coach".into(),
        claim: Some(CoachContactClaim {
            coach: coach.id.clone(),
            school: school.clone(),
            role: CoachRole::HeadCoach,
            program: CoachContactProgram::Team {
                sport: Sport::OutdoorTrack,
                gender: Gender::Girls,
            },
            mailbox: Some(mailbox.into()),
        }),
    }];
    Ok(coach)
}

fn add_address(school: &mut CanonicalSchool, street: &str, key: &str) -> TestResult {
    let state = school.state.ok_or("fixture school state")?;
    let owner = SourceIdentity::new(SourceNamespace::association_school("fixture"), key);
    school.source_identities.push(owner.clone());
    school.add_postal_address(SchoolPostalAddress::new(
        PostalAddress::line(StreetLine::parse(street)?).with_state(state),
        owner,
        SourceLabel::AthleticAssociation { state },
        Evidence::parsed(
            SourceRef::new(
                "fixture",
                Some(format!("https://school.test/address/{key}")),
            ),
            "2026-09-20",
        ),
        key.repeat(64),
    )?)?;
    Ok(())
}

fn csv_row(directory: &Path, derivation: &Derivation<'_>) -> TestResult<BTreeMap<String, String>> {
    let path = directory.join("contacts.csv");
    crate::workbook::write_recruiting_csv(
        derivation,
        SchoolYear::new(2026).ok_or("fixture school year")?,
        &path,
    )?;
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    let mut rows = reader.records();
    let row = rows.next().ok_or("missing recruiting row")??;
    check!(rows.next().transpose()?.is_none());
    Ok(headers
        .iter()
        .zip(row.iter())
        .map(|(header, value)| (header.to_owned(), value.to_owned()))
        .collect())
}

fn field<'a>(row: &'a BTreeMap<String, String>, key: &str) -> TestResult<&'a str> {
    row.get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("missing recruiting field {key}").into())
}

fn check_contact(row: &BTreeMap<String, String>, coach: &CanonicalCoach) -> TestResult {
    let fact = coach.tenure_evidence.first().ok_or("qualified tenure")?;
    let mailbox = coach
        .professional_email
        .as_deref()
        .ok_or("qualified mailbox")?;
    let url = fact.source.url.as_deref().ok_or("qualified capture")?;
    for (key, expected) in [
        ("head_track_coach", coach.name.as_str()),
        ("head_track_coach_email", mailbox),
        ("coach_id", coach.id.as_str()),
        ("coach_source_url", url),
        ("coach_capture_sha256", fact.source_sha256.as_str()),
        ("coach_acquired_at", fact.retrieved_at.as_str()),
    ] {
        check!(eq; field(row, key)?, expected, "{key}");
    }
    let captures: serde_json::Value = serde_json::from_str(field(row, "contact_provenance")?)?;
    check!(eq; captures, serde_json::json!([{
        "coach_id": coach.id.as_str(),
        "mailbox": mailbox,
        "source_url": url,
        "source_sha256": fact.source_sha256,
        "acquired_at": fact.retrieved_at,
    }]));
    Ok(())
}

#[test]
fn historical_recruiting_projection_keeps_previous_contacts_and_postal_claims() -> TestResult {
    let mut fixture = fixture_with_contacts()?;
    fixture.dataset.bind_historical_policy_for_test(21)?;
    let original = serde_json::to_vec(&fixture.dataset)?;
    let historical = Derivation::of(&fixture.dataset, Scope::AllSources, None, &[])?;
    let row = csv_row(fixture.directory.path(), &historical)?;
    let coach = fixture
        .dataset
        .coaches
        .iter()
        .find(|coach| coach.school == fixture.previous)
        .ok_or("previous coach")?;
    check!(eq; field(&row, "athlete_id")?, fixture.athlete.as_str());
    check!(eq; field(&row, "school")?, "Previous school");
    check!(eq; field(&row, "school_address")?, "1 Previous Road, WI");
    check_contact(&row, coach)?;
    check!(eq; historical.performances().first().ok_or("old result")?.source_key, "old-result");
    check!(eq; serde_json::to_vec(&fixture.dataset)?, original);
    Ok(())
}

#[test]
fn selected_school_projection_keeps_contact_observations_postal_and_results_unchanged() -> TestResult
{
    let mut fixture = fixture_with_contacts()?;
    let affiliations = view(
        &fixture,
        "2026-10-01",
        Some(resolved(&fixture.current, "dated-transfer")),
    )?;
    bind_selected(&mut fixture, &affiliations)?;
    let original = serde_json::to_vec(&fixture.dataset)?;
    let derivation = Derivation::of(&fixture.dataset, Scope::AllSources, None, &[])?;
    let athlete = derivation.athletes().first().ok_or("current athlete")?;
    check!(eq; athlete.school, fixture.current);
    let addresses =
        crate::export::postal::athlete_address_index(&fixture.dataset, derivation.athletes())?;
    check!(eq; addresses.get(athlete.id.as_str()).map(String::as_str), Some("2 Current Road, MI"));
    check!(eq; serde_json::to_vec(derivation.coach_observations())?, serde_json::to_vec(&fixture.dataset.coach_observations)?);
    check!(eq; serde_json::to_vec(derivation.performances())?, serde_json::to_vec(&fixture.dataset.performances)?);
    check!(eq; serde_json::to_vec(&fixture.dataset)?, original);
    Ok(())
}

#[derive(Deserialize)]
struct CapturedCoach {
    school: SchoolId,
    name: String,
    mailbox: String,
    school_year: i16,
    appointment: String,
}

#[derive(Deserialize)]
struct CapturedAddress {
    school: SchoolId,
    street: String,
    state: UsJurisdiction,
}

fn capture_source(store: &Store, url: String, body: Vec<u8>) -> TestResult<FetchOutcome> {
    let capture = FetchOutcome {
        url,
        response_url: None,
        method: "GET".into(),
        status: 200,
        content_digest: hash(&body),
        bytes: body.len(),
        fetched_at: "2026-09-20T00:00:00Z".into(),
        from_cache: false,
        content_type: Some("application/json".into()),
        body,
    };
    retain(store, &capture)?;
    Ok(capture)
}

fn append_captured_coach(
    store: &Store,
    school: &SchoolId,
    name: &str,
    mailbox: &str,
    key: &str,
) -> TestResult<CanonicalCoach> {
    let capture = capture_source(
        store,
        format!("https://school.test/coaches/{key}"),
        serde_json::to_vec(&serde_json::json!({
            "synthetic_fixture": true, "school": school, "name": name,
            "mailbox": mailbox, "school_year": 2026,
            "appointment": "2026-27 girls outdoor track head coach",
        }))?,
    )?;
    let source: CapturedCoach = serde_json::from_slice(&capture.body)?;
    check!(eq; &source.school, school);
    check!(eq; source.school_year, 2026);
    check!(eq; source.appointment, "2026-27 girls outdoor track head coach");
    let mut coach = qualified_coach(&source.school, &source.name, &source.mailbox, key)?;
    let tenure = coach.tenure_evidence.first_mut().ok_or("current tenure")?;
    tenure.source_sha256 = capture.content_digest;
    store.append(Table::Coaches, &coach)?;
    Ok(coach)
}

fn append_previous_contact(
    fixture: &SelectedGraphFixture,
) -> TestResult<(SchoolId, CanonicalCoach)> {
    let previous = fixture
        .store
        .snapshot()
        .scan::<CanonicalTeam>(Table::Teams)?
        .into_iter()
        .find(|team| team.id == fixture.performance.team)
        .ok_or("historical team")?
        .school;
    let mut school = fixture
        .store
        .snapshot()
        .scan::<CanonicalSchool>(Table::Schools)?
        .into_iter()
        .find(|school| school.id == previous)
        .ok_or("previous school")?;
    let capture = capture_source(
        &fixture.store,
        "https://school.test/address/previous".into(),
        serde_json::to_vec(&serde_json::json!({
            "synthetic_fixture": true, "school": previous,
            "street": "1 Previous Road", "state": school.state.ok_or("previous state")?,
        }))?,
    )?;
    let source: CapturedAddress = serde_json::from_slice(&capture.body)?;
    check!(eq; source.school, previous);
    check!(eq; Some(source.state), school.state);
    let owner = SourceIdentity::new(SourceNamespace::association_school("fixture"), "previous");
    school.source_identities.push(owner.clone());
    school.add_postal_address(SchoolPostalAddress::new(
        PostalAddress::line(StreetLine::parse(&source.street)?).with_state(source.state),
        owner,
        SourceLabel::AthleticAssociation {
            state: source.state,
        },
        Evidence::parsed(
            SourceRef::new("fixture", Some(capture.url)),
            &capture.fetched_at,
        ),
        capture.content_digest,
    )?)?;
    fixture.store.append(Table::Schools, &school)?;
    let coach = append_captured_coach(
        &fixture.store,
        &previous,
        "Previous coach",
        "previous@school.test",
        "previous",
    )?;
    Ok((previous, coach))
}

fn check_retained_history(
    fixture: &SelectedGraphFixture,
    dataset: &ExportDataset,
    derivation: &Derivation<'_>,
    previous: &SchoolId,
) -> TestResult {
    check!(ne; previous, &fixture.roster_school);
    check!(eq; derivation.performances().len(), 2);
    check!(eq; serde_json::to_vec(derivation.performances())?, serde_json::to_vec(&dataset.performances)?);
    let mut keys = Vec::new();
    for performance in derivation.performances() {
        let team = dataset
            .teams
            .get(&performance.team)
            .ok_or("historical team")?;
        check!(eq; &team.school, previous);
        let participation = dataset
            .verified_participation()?
            .assessment(&performance.id)
            .ok_or("retained participation")?;
        check!(eq; participation.outcome(), crate::export::ParticipationOutcome::Unresolved);
        check!(eq; participation.reasons().iter().collect::<Vec<_>>(), vec![
            census_domain::participation_candidates::ParticipationReason::HighSchoolStatusMissing,
            census_domain::participation_candidates::ParticipationReason::SummerBoundaryUnresolved,
        ]);
        keys.push(performance.source_key.as_str());
    }
    keys.sort_unstable();
    check!(eq; keys, vec!["milesplit_result:201782263", "milesplit_result:201782264"]);
    Ok(())
}

#[tokio::test]
async fn selected_recruiting_csv_joins_current_contacts_without_rewriting_history() -> TestResult {
    let (fixture, public_id) = SelectedGraphFixture::transferred().await?;
    let (previous, previous_coach) = append_previous_contact(&fixture)?;
    let current_coach = append_captured_coach(
        &fixture.store,
        &fixture.roster_school,
        "Current coach",
        "current@school.test",
        "current",
    )?;
    let objects = tempfile::tempdir()?;
    let dataset = fixture.capture(objects.path())?;
    let original = serde_json::to_vec(&dataset)?;
    let derivation = Derivation::of(&dataset, Scope::AllSources, Some(2027), &[])?;
    let row = csv_row(fixture.directory.path(), &derivation)?;
    check!(eq; field(&row, "athlete_id")?, public_id.as_str());
    check!(eq; field(&row, "canonical_candidate_id")?, fixture.performance.athlete.as_str());
    check!(eq; field(&row, "school")?, "Synthetic Transfer High");
    check!(eq; field(&row, "school_address")?, "123 Synthetic Transfer Lane, Madison, WI 53703");
    check_contact(&row, &current_coach)?;
    check!(dataset
        .coaches
        .iter()
        .any(|coach| coach.id == previous_coach.id));
    check!(eq; dataset.verified_public_identities()?.public_id(fixture.roster_athlete.as_str()), Some(&public_id));
    check_retained_history(&fixture, &dataset, &derivation, &previous)?;
    check!(eq; serde_json::to_vec(&dataset)?, original);
    Ok(())
}

#[tokio::test]
async fn current_school_missing_contacts_never_inherits_previous_school_claims() -> TestResult {
    let (fixture, public_id) = SelectedGraphFixture::transferred_without_current_address().await?;
    let (previous, previous_coach) = append_previous_contact(&fixture)?;
    let objects = tempfile::tempdir()?;
    let dataset = fixture.capture(objects.path())?;
    let original = serde_json::to_vec(&dataset)?;
    check!(dataset
        .coaches
        .iter()
        .any(|coach| coach.id == previous_coach.id));
    check!(!dataset
        .schools
        .get(&previous)
        .ok_or("previous school")?
        .postal_addresses
        .is_empty());
    check!(dataset
        .schools
        .get(&fixture.roster_school)
        .ok_or("current school")?
        .postal_addresses
        .is_empty());
    let derivation = Derivation::of(&dataset, Scope::AllSources, Some(2027), &[])?;
    let row = csv_row(fixture.directory.path(), &derivation)?;
    check!(eq; field(&row, "athlete_id")?, public_id.as_str());
    check!(eq; field(&row, "canonical_candidate_id")?, fixture.performance.athlete.as_str());
    check!(eq; field(&row, "school")?, "Synthetic Transfer High");
    for key in [
        "school_address",
        "head_track_coach",
        "head_track_coach_email",
        "coach_id",
        "coach_source_url",
        "coach_capture_sha256",
        "coach_acquired_at",
    ] {
        check!(eq; field(&row, key)?, "", "{key}");
    }
    check!(eq; field(&row, "contact_provenance")?, "[]");
    check_retained_history(&fixture, &dataset, &derivation, &previous)?;
    check!(eq; serde_json::to_vec(&dataset)?, original);
    Ok(())
}
