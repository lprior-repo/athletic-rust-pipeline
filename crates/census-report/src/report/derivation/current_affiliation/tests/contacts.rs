use super::*;
use census_domain::model::{
    CanonicalCoach, CanonicalSchool, CoachContactClaim, CoachContactProgram, CoachRole,
    CoachTenure, CoachTenureEvidence, Evidence, Gender, SchoolPostalAddress, SchoolYear,
    SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::school_directory::{PostalAddress, SourceLabel, StreetLine};

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

fn csv_row(fixture: &Fixture, derivation: &Derivation<'_>) -> TestResult<BTreeMap<String, String>> {
    let path = fixture.directory.path().join("contacts.csv");
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
fn selected_recruiting_csv_joins_current_contacts_without_rewriting_history() -> TestResult {
    let mut fixture = fixture_with_contacts()?;
    let selected_policy = fixture.dataset.lineage.policy_revision;
    fixture.dataset.bind_historical_policy_for_test(21)?;
    let old_row = {
        let historical = Derivation::of(&fixture.dataset, Scope::AllSources, None, &[])?;
        csv_row(&fixture, &historical)?
    };
    fixture.dataset.lineage.policy_revision = selected_policy;
    fixture.dataset.bind_selection_for_test(
        ProfileId::parse(&"a".repeat(64))?,
        ProfileDate::parse("2026-10-01")?,
    )?;
    fixture.dataset.bind_participation_rule_for_test(
        census_domain::participation_candidates::participation_policy_v2()?,
    )?;
    let original = serde_json::to_vec(&fixture.dataset)?;
    let performances = serde_json::to_vec(&fixture.dataset.performances)?;
    let affiliations = view(
        &fixture,
        "2026-10-01",
        Some(resolved(&fixture.current, "dated-transfer")),
    )?;
    bind_selected(&mut fixture, &affiliations)?;
    let derivation = Derivation::of(&fixture.dataset, Scope::AllSources, None, &[])?;
    let current_row = csv_row(&fixture, &derivation)?;
    let public_id = fixture
        .dataset
        .verified_public_identities()?
        .public_id(fixture.athlete.as_str())
        .ok_or("accepted public athlete")?;
    check!(eq; field(&old_row, "athlete_id")?, fixture.athlete.as_str());
    check!(eq; field(&current_row, "athlete_id")?, public_id.as_str());
    check!(eq; field(&current_row, "canonical_candidate_id")?, fixture.athlete.as_str());
    for (school, row, name, address) in [
        (
            &fixture.previous,
            &old_row,
            "Previous school",
            "1 Previous Road, WI",
        ),
        (
            &fixture.current,
            &current_row,
            "Current school",
            "2 Current Road, MI",
        ),
    ] {
        let coach = fixture
            .dataset
            .coaches
            .iter()
            .find(|coach| &coach.school == school)
            .ok_or("school coach")?;
        check!(eq; field(row, "school")?, name);
        check!(eq; field(row, "school_address")?, address);
        check_contact(row, coach)?;
    }
    check!(eq; derivation.performances().len(), 1);
    let performance = derivation
        .performances()
        .first()
        .ok_or("historical result")?;
    let team = fixture
        .dataset
        .teams
        .get(&performance.team)
        .ok_or("historical team")?;
    check!(eq; &team.school, &fixture.previous);
    check!(eq; performance.source_key, "old-result");
    check!(eq; serde_json::to_vec(derivation.performances())?, performances);
    check!(eq; serde_json::to_vec(&fixture.dataset)?, original);
    Ok(())
}

#[test]
fn current_school_missing_contacts_never_inherits_previous_school_claims() -> TestResult {
    let mut fixture = fixture_with_contacts()?;
    fixture
        .dataset
        .coaches
        .retain(|coach| coach.school == fixture.previous);
    fixture
        .dataset
        .coach_observations
        .retain(|coach| coach.school == fixture.previous);
    fixture
        .dataset
        .schools
        .get_mut(&fixture.current)
        .ok_or("current school")?
        .postal_addresses
        .clear();
    let selected_policy = fixture.dataset.lineage.policy_revision;
    fixture.dataset.bind_historical_policy_for_test(21)?;
    let old_row = {
        let historical = Derivation::of(&fixture.dataset, Scope::AllSources, None, &[])?;
        csv_row(&fixture, &historical)?
    };
    check_contact(
        &old_row,
        fixture.dataset.coaches.first().ok_or("previous coach")?,
    )?;
    check!(eq; field(&old_row, "school_address")?, "1 Previous Road, WI");
    fixture.dataset.lineage.policy_revision = selected_policy;
    fixture.dataset.bind_selection_for_test(
        ProfileId::parse(&"a".repeat(64))?,
        ProfileDate::parse("2026-10-01")?,
    )?;
    fixture.dataset.bind_participation_rule_for_test(
        census_domain::participation_candidates::participation_policy_v2()?,
    )?;
    let original = serde_json::to_vec(&fixture.dataset)?;
    let affiliations = view(
        &fixture,
        "2026-10-01",
        Some(resolved(&fixture.current, "dated-transfer")),
    )?;
    bind_selected(&mut fixture, &affiliations)?;
    let derivation = Derivation::of(&fixture.dataset, Scope::AllSources, None, &[])?;
    let row = csv_row(&fixture, &derivation)?;
    check!(eq; field(&row, "school")?, "Current school");
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
    check!(eq; serde_json::to_vec(&fixture.dataset)?, original);
    Ok(())
}
