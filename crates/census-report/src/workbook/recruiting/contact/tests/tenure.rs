use super::*;
use census_domain::model::Evidence;

#[test]
fn only_the_queried_academic_year_qualifies_a_current_claim() {
    for declared in [2025, 2026, 2027] {
        let mut coach = head("Declared coach", Sport::OutdoorTrack, Gender::Boys);
        coach.professional_email = Some("coach@example.invalid".into());
        coach.tenure_evidence = vec![claim(CoachTenure::Current {
            school_year: SchoolYear::new(declared).unwrap(),
        })];
        let contacts = contacts(&[coach], year());
        let athlete = athlete();
        let scope = scoped(contacts.get(school().as_str()), &athlete);
        if declared == 2026 {
            assert_eq!(scope.preferred().state, ContactState::ProfessionalCoachEmail);
            assert_eq!(scope.all_emails(), "coach@example.invalid");
        } else {
            assert_eq!(scope.preferred().state, ContactState::ContactResearchUnknown);
            assert_eq!(scope.track_names(), None);
            assert_eq!(scope.all_emails(), "");
        }
    }
}

#[test]
fn a_recently_fetched_legacy_row_without_tenure_is_not_current() {
    let mut coach = head("Undated coach", Sport::OutdoorTrack, Gender::Boys);
    coach.professional_email = Some("undated@example.invalid".into());
    coach.evidence = vec![Evidence::parsed(SourceRef::new("fixture", None), "2026-10-01")];
    let mut encoded = serde_json::to_value(coach).unwrap();
    assert!(encoded.as_object_mut().unwrap().remove("tenure_evidence").is_some());
    let legacy: CanonicalCoach = serde_json::from_value(encoded).unwrap();
    let contacts = contacts(&[legacy], year());
    let athlete = athlete();
    let scope = scoped(contacts.get(school().as_str()), &athlete);
    assert_eq!(scope.preferred().state, ContactState::ContactResearchUnknown);
    assert_eq!(scope.track_emails(), None);
    assert_eq!(scope.all_emails(), "");
}

#[test]
fn an_undated_observation_cannot_lend_an_address_to_a_current_name() {
    let current = head("Same coach", Sport::OutdoorTrack, Gender::Boys);
    let mut undated = current.clone();
    undated.tenure_evidence.clear();
    undated.professional_email = Some("undated@example.invalid".into());
    let contacts = contacts(&[current, undated], year());
    let athlete = athlete();
    let scope = scoped(contacts.get(school().as_str()), &athlete);
    assert_eq!(scope.preferred().name, "Same coach");
    assert_eq!(scope.preferred().state, ContactState::CoachNameOnly);
    assert_eq!(scope.track_emails(), None);
    assert_eq!(scope.all_emails(), "");
}

#[test]
fn conflicting_tenure_across_observations_is_retained_in_either_order() {
    let mut current = head("Same coach", Sport::OutdoorTrack, Gender::Boys);
    current.professional_email = Some("coach@example.invalid".into());
    let mut former = current.clone();
    former.tenure_evidence = vec![claim(CoachTenure::Former {
        last_school_year: SchoolYear::new(2025),
    })];
    for rows in [vec![current.clone(), former.clone()], vec![former, current]] {
        let contacts = contacts(&rows, year());
        let athlete = athlete();
        let scope = scoped(contacts.get(school().as_str()), &athlete);
        assert_eq!(scope.preferred().state, ContactState::ContactTenureConflict);
        assert_eq!(scope.all_emails(), "");
        let findings = super::super::disagreements(&rows, year());
        assert_eq!(findings[0].state, ContactState::ContactTenureConflict);
        assert_eq!(findings[0].rows.len(), 2);
    }
}

#[test]
fn incomplete_tenure_metadata_refuses_selection_but_remains_a_distinct_state() {
    let mut coach = head("Malformed evidence", Sport::OutdoorTrack, Gender::Boys);
    coach.professional_email = Some("coach@example.invalid".into());
    coach.tenure_evidence[0].source_sha256.clear();
    let contacts = contacts(&[coach], year());
    let athlete = athlete();
    let scope = scoped(contacts.get(school().as_str()), &athlete);
    assert_eq!(scope.preferred().state, ContactState::ContactEvidenceInvalid);
    assert_eq!(scope.track_emails(), None);
    assert_eq!(scope.all_emails(), "");
}
