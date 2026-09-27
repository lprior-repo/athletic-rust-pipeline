use super::*;

#[test]
fn opposite_or_unspecified_sides_never_supply_athlete_contact_columns() {
    for side in [Gender::Girls, Gender::Unknown] {
        let mut coach = head("Not the boys coach", Sport::OutdoorTrack, side);
        coach.professional_email = Some("wrong-side@example.invalid".into());
        let contacts = contacts(&[coach], year());
        let athlete = athlete();
        let scope = scoped(contacts.get(school().as_str()), &athlete);
        assert_eq!(scope.preferred().state, ContactState::ContactResearchUnknown);
        assert_eq!(scope.track_names(), None);
        assert_eq!(scope.track_emails(), None);
        assert_eq!(scope.professional_coach_email(), None);
        assert_eq!(scope.all_emails(), "");
    }
}

#[test]
fn indoor_outdoor_and_cross_country_contacts_keep_their_programmes() {
    let rows: Vec<_> = [
        ("Outdoor coach", Sport::OutdoorTrack, "outdoor@example.invalid"),
        ("Indoor coach", Sport::IndoorTrack, "indoor@example.invalid"),
        ("XC coach", Sport::CrossCountry, "xc@example.invalid"),
    ].into_iter().map(|(name, sport, email)| {
        let mut coach = head(name, sport, Gender::Mixed);
        coach.professional_email = Some(email.into());
        coach
    }).collect();
    let contacts = contacts(&rows, year());
    for coach in &rows {
        let mut athlete = athlete();
        athlete.sports = vec![coach.sport.unwrap()];
        let scope = scoped(contacts.get(school().as_str()), &athlete);
        assert_eq!(scope.preferred().name, coach.name);
        assert_eq!(scope.all_emails(), coach.professional_email.as_deref().unwrap());
        if coach.sport == Some(Sport::CrossCountry) {
            assert_eq!(scope.track_emails(), None);
        } else {
            assert!(scope.cross_country().is_none());
        }
    }
}

#[test]
fn unknown_sport_or_role_does_not_mean_school_wide_head_coach() {
    for (sport, role) in [(None, CoachRole::HeadCoach),
        (Some(Sport::OutdoorTrack), CoachRole::Unknown)] {
        let mut coach = person("Unqualified scope", sport, Gender::Mixed, role);
        coach.professional_email = Some("unknown@example.invalid".into());
        let contacts = contacts(&[coach], year());
        let athlete = athlete();
        let scope = scoped(contacts.get(school().as_str()), &athlete);
        assert_eq!(scope.preferred().state, ContactState::ContactResearchUnknown);
        assert_eq!(scope.track_names(), None);
        assert_eq!(scope.all_emails(), "");
    }
}

#[test]
fn the_specific_team_coach_is_selected_before_mailbox_preference() {
    let boys = head("Boys coach", Sport::OutdoorTrack, Gender::Boys);
    let mut mixed = head("General programme coach", Sport::OutdoorTrack, Gender::Mixed);
    mixed.professional_email = Some("general@example.invalid".into());
    let contacts = contacts(&[boys, mixed], year());
    let athlete = athlete();
    let scope = scoped(contacts.get(school().as_str()), &athlete);
    assert_eq!(scope.preferred().name, "Boys coach");
    assert_eq!(scope.preferred().state, ContactState::CoachNameOnly);
    assert_eq!(scope.all_emails(), "");
}

#[test]
fn a_school_index_entry_cannot_be_applied_to_another_school() {
    let coach = head("Current coach", Sport::OutdoorTrack, Gender::Boys);
    let contacts = contacts(&[coach], year());
    let mut athlete = athlete();
    athlete.school = SchoolId::mint("sch", &["different synthetic school"]);
    let scope = scoped(contacts.get(school().as_str()), &athlete);
    assert_eq!(scope.preferred().state, ContactState::ContactResearchUnknown);
    assert_eq!(scope.track_names(), None);
    assert_eq!(scope.all_emails(), "");
}

#[test]
fn assistants_require_current_matching_programme_and_side_without_becoming_head_coaches() {
    let mut rows = Vec::new();
    for (name, sport, side, current) in [
        ("Relevant assistant", Sport::OutdoorTrack, Gender::Boys, true),
        ("Other side", Sport::OutdoorTrack, Gender::Girls, true),
        ("Other programme", Sport::CrossCountry, Gender::Boys, true),
        ("Expired assistant", Sport::OutdoorTrack, Gender::Boys, false),
    ] {
        let mut coach = person(name, Some(sport), side, CoachRole::AssistantCoach);
        coach.professional_email = Some(format!("{}@example.invalid", name.replace(' ', "-")));
        if !current {
            coach.tenure_evidence = vec![claim(CoachTenure::Current {
                school_year: SchoolYear::new(2025).unwrap(),
            })];
        }
        rows.push(coach);
    }
    let contacts = contacts(&rows, year());
    let athlete = athlete();
    let scope = scoped(contacts.get(school().as_str()), &athlete);
    assert_eq!(scope.all_emails(), "Relevant-assistant@example.invalid");
    assert_eq!(scope.professional_coach_email(), None);
    assert_eq!(scope.track_names(), None);
    assert_eq!(scope.preferred().state, ContactState::ContactResearchUnknown);
}
