use super::*;

#[test]
fn identity_only_projection_matches_full_proposed_ids_without_a_school_argument() -> TestResult {
    let capture = fixture::capture()?;
    let document = read_retained_html_result_rows(&capture, &fixture::locators())?;
    let subjects = fixture::subjects(&document)?;
    let athlete = fixture::athlete();
    let school = fixture::school();
    let targets = fixture::targets(&subjects, &athlete, &school);
    let full = project_retained_html_results(&document, &targets)?;
    for (index, row) in full.rows().iter().enumerate() {
        let identity = project_retained_html_result_identity(&document, index, &athlete)?;
        let performance = row.proposed_performance();
        check!(eq; identity.performance_id(), &performance.id);
        check!(eq; identity.athlete(), &performance.athlete);
        check!(eq; identity.meet_id(), &performance.meet);
        check!(eq; identity.event_id(), &performance.event);
        check!(eq; identity.date(), performance.date);
        check!(eq; identity.source_key(), performance.source_key);
        check!(performance.source_athlete.is_none());
    }
    Ok(())
}

#[test]
fn identity_only_projection_refuses_missing_rows_and_wrong_athlete_id_kind() -> TestResult {
    let capture = fixture::capture()?;
    let document = read_retained_html_result_rows(&capture, &fixture::locators())?;
    let athlete = fixture::athlete();
    check!(matches!(
        project_retained_html_result_identity(&document, 3, &athlete),
        Err(HtmlResultProjectionError::Invalid(
            "selected original row missing"
        ))
    ));
    let wrong_kind: AthleteId = serde_json::from_str("\"ath_0000000000000000\"")?;
    check!(matches!(
        project_retained_html_result_identity(&document, 0, &wrong_kind),
        Err(HtmlResultProjectionError::Invalid(
            "proposed canonical ID kind"
        ))
    ));
    Ok(())
}
