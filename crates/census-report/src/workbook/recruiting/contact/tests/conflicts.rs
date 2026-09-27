use super::*;
use census_domain::model::Evidence;

#[test]
fn fetch_recency_and_mailboxes_do_not_resolve_distinct_current_coaches() {
    for with_addresses in [false, true] {
        let mut older = head("Older observation", Sport::OutdoorTrack, Gender::Boys);
        let mut newer = head("Newer observation", Sport::OutdoorTrack, Gender::Boys);
        older.evidence = vec![Evidence::parsed(SourceRef::new("fixture", None), "2026-08-01")];
        newer.evidence = vec![Evidence::parsed(SourceRef::new("fixture", None), "2026-10-01")];
        if with_addresses {
            older.professional_email = Some("older@example.invalid".into());
            newer.professional_email = Some("newer@example.invalid".into());
        }
        for rows in [vec![older.clone(), newer.clone()], vec![newer, older]] {
            let contacts = contacts(&rows, year());
            let athlete = athlete();
            let scope = scoped(contacts.get(school().as_str()), &athlete);
            assert_eq!(scope.preferred().state, ContactState::ContactConflict);
            assert_eq!(scope.preferred().name, "");
            assert_eq!(scope.track_emails(), None);
            assert_eq!(scope.all_emails(), "");
            let findings = super::super::disagreements(&rows, year());
            assert_eq!(findings[0].state, ContactState::ContactConflict);
            assert!(findings[0].rows.iter().any(|row| row.contains("Older observation")));
            assert!(findings[0].rows.iter().any(|row| row.contains("Newer observation")));
        }
    }
}

#[test]
fn contradictory_current_addresses_for_one_owner_are_not_ranked() {
    let mut first = head("Same coach", Sport::OutdoorTrack, Gender::Boys);
    first.professional_email = Some("first@example.invalid".into());
    let mut second = first.clone();
    second.professional_email = Some("second@example.invalid".into());
    let result = selected(&[first, second], &athlete());
    assert_eq!(result.state, ContactState::ContactConflict);
    assert_eq!(result.email, "");
}

#[test]
fn complementary_current_fields_of_one_owner_merge_without_reclassifying_personal_mail() {
    let mut personal = head("Same coach", Sport::OutdoorTrack, Gender::Boys);
    personal.personal_email = Some("coach@outlook.com".into());
    let mut professional = personal.clone();
    professional.personal_email = None;
    professional.professional_email = Some("coach@example.invalid".into());
    let contacts = contacts(&[personal, professional], year());
    let athlete = athlete();
    let scope = scoped(contacts.get(school().as_str()), &athlete);
    assert_eq!(scope.preferred().email, "coach@example.invalid");
    assert_eq!(scope.preferred().state, ContactState::ProfessionalCoachEmail);
    assert_eq!(scope.all_emails(), "coach@example.invalid; coach@outlook.com");
}

#[test]
fn an_unresolved_head_coach_conflict_is_not_hidden_by_director_fallback() {
    let first = head("First coach", Sport::OutdoorTrack, Gender::Boys);
    let second = head("Second coach", Sport::OutdoorTrack, Gender::Boys);
    let mut ad = director();
    ad.professional_email = Some("director@example.invalid".into());
    let result = selected(&[first, second, ad], &athlete());
    assert_eq!(result.state, ContactState::ContactConflict);
    assert_eq!(result.name, "");
    assert_eq!(result.email, "");
}

#[test]
fn an_unrelated_programme_conflict_does_not_poison_a_qualified_contact() {
    let mut relevant = head("Outdoor coach", Sport::OutdoorTrack, Gender::Boys);
    relevant.professional_email = Some("outdoor@example.invalid".into());
    let first = head("First XC coach", Sport::CrossCountry, Gender::Boys);
    let second = head("Second XC coach", Sport::CrossCountry, Gender::Boys);
    let contacts = contacts(&[relevant, first, second], year());
    let athlete = athlete();
    let scope = scoped(contacts.get(school().as_str()), &athlete);
    assert_eq!(scope.preferred().email, "outdoor@example.invalid");
    assert_eq!(scope.all_emails(), "outdoor@example.invalid");
    assert!(scope.cross_country().is_none());
}
