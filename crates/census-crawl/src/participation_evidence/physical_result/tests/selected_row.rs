use super::*;

#[test]
fn selected_second_row_matches_same_row_from_full_projection() -> TestResult {
    let capture = fixture::capture()?;
    let document = read_retained_html_result_rows(&capture, &fixture::locators())?;
    let subjects = fixture::subjects(&document)?;
    let athlete = fixture::athlete();
    let school = fixture::school();
    let targets = fixture::targets(&subjects, &athlete, &school);
    let full = project_retained_html_results(&document, &targets)?;
    let row_target = *targets.get(1).ok_or("second target")?;
    let row_only = project_retained_html_result_row(&document, 1, &row_target)?;
    check!(eq; row_only.rows().len(), 1);
    let full_row = full.rows().get(1).ok_or("full row 1")?;
    let row_only_row = row_only.rows().first().ok_or("row only row")?;
    check!(eq; row_only_row.proposed_performance(), full_row.proposed_performance());
    check!(eq; row_only_row.proposed_event(), full_row.proposed_event());
    check!(eq; row_only_row.proposed_team(), full_row.proposed_team());
    check!(eq; row_only_row.locator(), full_row.locator());
    check!(eq; row_only_row.subject_reference(), full_row.subject_reference());
    let (meet, rows) = row_only.into_proposed_entities();
    check!(eq; rows.len(), 1);
    let (event, team, performance) = rows
        .into_iter()
        .next()
        .ok_or("consumed row")?
        .into_proposed_entities();
    check!(eq; performance.athlete, athlete);
    check!(eq; event.kind, EventKind::Track110mHurdles);
    check!(eq; team.school, school);
    check!(eq; meet.date, "2026-05-15");
    Ok(())
}

#[test]
fn wrong_row_target_subject_or_locator_refuses_the_single_projection() -> TestResult {
    let capture = fixture::capture()?;
    let document = read_retained_html_result_rows(&capture, &fixture::locators())?;
    let subjects = fixture::subjects(&document)?;
    let athlete = fixture::athlete();
    let school = fixture::school();
    let targets = fixture::targets(&subjects, &athlete, &school);
    let row_target = *targets.get(1).ok_or("second target")?;
    let mut wrong_locator = row_target;
    wrong_locator.locator = *fixture::locators().first().ok_or("first locator")?;
    check!(matches!(
        project_retained_html_result_row(&document, 1, &wrong_locator),
        Err(HtmlResultProjectionError::Invalid(_))
    ));
    let mut wrong_subject = row_target;
    wrong_subject.subject_reference = fixture::ATTACHMENT;
    check!(matches!(
        project_retained_html_result_row(&document, 1, &wrong_subject),
        Err(HtmlResultProjectionError::Invalid(_))
    ));
    let out_of_range_target = *targets.last().ok_or("last target")?;
    check!(matches!(
        project_retained_html_result_row(&document, 3, &out_of_range_target),
        Err(HtmlResultProjectionError::Invalid(_))
    ));
    Ok(())
}
