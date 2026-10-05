use super::*;

#[test]
fn opposite_or_unspecified_sides_never_supply_athlete_contact_columns() -> TestResult {
    for side in [Gender::Girls, Gender::Unknown] {
        let mut coach = head("Not the boys coach", Sport::OutdoorTrack, side)?;
        coach.professional_email = Some("wrong-side@example.invalid".into());
        bind(&mut coach);
        let contacts = contacts(&[coach], year()?);
        let athlete = athlete();
        let scope = scoped(contacts.get(school().as_str()), &athlete);
        check!(eq; scope.preferred().state,
        ContactState::ContactResearchUnknown);
        check!(eq; scope.track_names(), None);
        check!(eq; scope.track_emails(), None);
        check!(eq; scope.professional_coach_email(), None);
        check!(eq; scope.all_emails(), "");
    }
    Ok(())
}

#[test]
fn indoor_outdoor_and_cross_country_contacts_keep_their_programmes() -> TestResult {
    let rows: Vec<_> = [
        (
            "Outdoor coach",
            Sport::OutdoorTrack,
            "outdoor@example.invalid",
        ),
        ("Indoor coach", Sport::IndoorTrack, "indoor@example.invalid"),
        ("XC coach", Sport::CrossCountry, "xc@example.invalid"),
    ]
    .into_iter()
    .map(|(name, sport, email)| {
        let mut coach = head(name, sport, Gender::Mixed)?;
        coach.professional_email = Some(email.into());
        bind(&mut coach);
        Ok(coach)
    })
    .collect::<TestResult<Vec<_>>>()?;
    let contacts = contacts(&rows, year()?);
    for coach in &rows {
        let mut athlete = athlete();
        athlete.sports = vec![coach.sport.ok_or("fixture coach has no sport")?];
        let scope = scoped(contacts.get(school().as_str()), &athlete);
        check!(eq; scope.preferred().name, coach.name);
        check!(eq; scope.all_emails(),
        coach
            .professional_email
            .as_deref()
            .ok_or("fixture coach has no professional email")?);
        if coach.sport == Some(Sport::CrossCountry) {
            check!(eq; scope.track_emails(), None);
        } else {
            check!(scope.cross_country().is_none());
        }
    }
    Ok(())
}

#[test]
fn unknown_sport_or_role_does_not_mean_school_wide_head_coach() -> TestResult {
    for (sport, role) in [
        (None, CoachRole::HeadCoach),
        (Some(Sport::OutdoorTrack), CoachRole::Unknown),
    ] {
        let mut coach = person("Unqualified scope", sport, Gender::Mixed, role)?;
        coach.professional_email = Some("unknown@example.invalid".into());
        let contacts = contacts(&[coach], year()?);
        let athlete = athlete();
        let scope = scoped(contacts.get(school().as_str()), &athlete);
        check!(eq; scope.preferred().state,
        ContactState::ContactResearchUnknown);
        check!(eq; scope.track_names(), None);
        check!(eq; scope.all_emails(), "");
    }
    Ok(())
}

#[test]
fn the_specific_team_coach_is_selected_before_mailbox_preference() -> TestResult {
    let boys = head("Boys coach", Sport::OutdoorTrack, Gender::Boys)?;
    let mut mixed = head(
        "General programme coach",
        Sport::OutdoorTrack,
        Gender::Mixed,
    )?;
    mixed.professional_email = Some("general@example.invalid".into());
    bind(&mut mixed);
    let contacts = contacts(&[boys, mixed], year()?);
    let athlete = athlete();
    let scope = scoped(contacts.get(school().as_str()), &athlete);
    check!(eq; scope.preferred().name, "Boys coach");
    check!(eq; scope.preferred().state, ContactState::CoachNameOnly);
    check!(eq; scope.all_emails(), "");
    Ok(())
}

#[test]
fn a_school_index_entry_cannot_be_applied_to_another_school() -> TestResult {
    let coach = head("Current coach", Sport::OutdoorTrack, Gender::Boys)?;
    let contacts = contacts(&[coach], year()?);
    let mut athlete = athlete();
    athlete.school = SchoolId::mint("sch", &["different synthetic school"]);
    let scope = scoped(contacts.get(school().as_str()), &athlete);
    check!(eq; scope.preferred().state,
    ContactState::ContactResearchUnknown);
    check!(eq; scope.track_names(), None);
    check!(eq; scope.all_emails(), "");
    Ok(())
}

#[test]
fn assistants_require_current_matching_programme_and_side_without_becoming_head_coaches(
) -> TestResult {
    let mut rows = Vec::new();
    for (name, sport, side, current) in [
        (
            "Relevant assistant",
            Sport::OutdoorTrack,
            Gender::Boys,
            true,
        ),
        ("Other side", Sport::OutdoorTrack, Gender::Girls, true),
        ("Other programme", Sport::CrossCountry, Gender::Boys, true),
        (
            "Expired assistant",
            Sport::OutdoorTrack,
            Gender::Boys,
            false,
        ),
    ] {
        let mut coach = person(name, Some(sport), side, CoachRole::AssistantCoach)?;
        coach.professional_email = Some(format!("{}@example.invalid", name.replace(' ', "-")));
        if !current {
            coach.tenure_evidence = vec![claim(CoachTenure::Current {
                school_year: SchoolYear::new(2025).ok_or("invalid fixture season")?,
            })];
        }
        bind(&mut coach);
        rows.push(coach);
    }
    let contacts = contacts(&rows, year()?);
    let athlete = athlete();
    let scope = scoped(contacts.get(school().as_str()), &athlete);
    check!(eq; scope.all_emails(), "Relevant-assistant@example.invalid");
    check!(eq; scope.professional_coach_email(), None);
    check!(eq; scope.track_names(), None);
    check!(eq; scope.preferred().state,
    ContactState::ContactResearchUnknown);
    Ok(())
}
