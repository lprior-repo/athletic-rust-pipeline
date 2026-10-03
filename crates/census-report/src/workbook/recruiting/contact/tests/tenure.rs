use super::*;
use census_domain::model::Evidence;

#[test]
fn only_the_queried_academic_year_qualifies_a_current_claim() -> TestResult {
    for declared in [2025, 2026, 2027] {
        let mut coach = head("Declared coach", Sport::OutdoorTrack, Gender::Boys)?;
        coach.professional_email = Some("coach@example.invalid".into());
        coach.tenure_evidence = vec![claim(CoachTenure::Current {
            school_year: SchoolYear::new(declared).ok_or("invalid declared fixture year")?,
        })];
        let contacts = contacts(&[coach], year()?);
        let athlete = athlete();
        let scope = scoped(contacts.get(school().as_str()), &athlete);
        if declared == 2026 {
            check!(eq; scope.preferred().state,
            ContactState::ProfessionalCoachEmail);
            check!(eq; scope.all_emails(), "coach@example.invalid");
        } else {
            check!(eq; scope.preferred().state,
            ContactState::ContactResearchUnknown);
            check!(eq; scope.track_names(), None);
            check!(eq; scope.all_emails(), "");
        }
    }
    Ok(())
}

#[test]
fn a_recently_fetched_legacy_row_without_tenure_is_not_current() -> TestResult {
    let mut coach = head("Undated coach", Sport::OutdoorTrack, Gender::Boys)?;
    coach.professional_email = Some("undated@example.invalid".into());
    coach.evidence = vec![Evidence::parsed(
        SourceRef::new("fixture", None),
        "2026-10-01",
    )];
    let mut encoded = serde_json::to_value(coach)?;
    check!(encoded
        .as_object_mut()
        .ok_or("serialized coach is not an object")?
        .remove("tenure_evidence")
        .is_some());
    let legacy: CanonicalCoach = serde_json::from_value(encoded)?;
    let contacts = contacts(&[legacy], year()?);
    let athlete = athlete();
    let scope = scoped(contacts.get(school().as_str()), &athlete);
    check!(eq; scope.preferred().state,
    ContactState::ContactResearchUnknown);
    check!(eq; scope.track_emails(), None);
    check!(eq; scope.all_emails(), "");
    Ok(())
}

#[test]
fn an_undated_observation_cannot_lend_an_address_to_a_current_name() -> TestResult {
    let current = head("Same coach", Sport::OutdoorTrack, Gender::Boys)?;
    let mut undated = current.clone();
    undated.tenure_evidence.clear();
    undated.professional_email = Some("undated@example.invalid".into());
    let contacts = contacts(&[current, undated], year()?);
    let athlete = athlete();
    let scope = scoped(contacts.get(school().as_str()), &athlete);
    check!(eq; scope.preferred().name, "Same coach");
    check!(eq; scope.preferred().state, ContactState::CoachNameOnly);
    check!(eq; scope.track_emails(), None);
    check!(eq; scope.all_emails(), "");
    Ok(())
}

#[test]
fn conflicting_tenure_across_observations_is_retained_in_either_order() -> TestResult {
    let mut current = head("Same coach", Sport::OutdoorTrack, Gender::Boys)?;
    current.professional_email = Some("coach@example.invalid".into());
    let mut former = current.clone();
    former.tenure_evidence = vec![claim(CoachTenure::Former {
        last_school_year: SchoolYear::new(2025),
    })];
    for rows in [vec![current.clone(), former.clone()], vec![former, current]] {
        let contacts = contacts(&rows, year()?);
        let athlete = athlete();
        let scope = scoped(contacts.get(school().as_str()), &athlete);
        check!(eq; scope.preferred().state, ContactState::ContactTenureConflict);
        check!(eq; scope.all_emails(), "");
        let findings = super::super::disagreements(&rows, year()?);
        check!(eq; findings[0].state, ContactState::ContactTenureConflict);
        check!(eq; findings[0].rows.len(), 2);
    }
    Ok(())
}

#[test]
fn incomplete_tenure_metadata_refuses_selection_but_remains_a_distinct_state() -> TestResult {
    let mut coach = head("Malformed evidence", Sport::OutdoorTrack, Gender::Boys)?;
    coach.professional_email = Some("coach@example.invalid".into());
    coach.tenure_evidence[0].source_sha256.clear();
    let contacts = contacts(&[coach], year()?);
    let athlete = athlete();
    let scope = scoped(contacts.get(school().as_str()), &athlete);
    check!(eq; scope.preferred().state,
    ContactState::ContactEvidenceInvalid);
    check!(eq; scope.track_emails(), None);
    check!(eq; scope.all_emails(), "");
    Ok(())
}
