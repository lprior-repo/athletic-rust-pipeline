use super::*;

#[test]
fn cold_capture_replay_reproduces_exact_entities_and_proposed_identities() -> TestResult {
    let original = fixture::capture(fixture::BODY)?;
    let retained = serde_json::to_vec(&original)?;
    let cold: RetainedHtmlCaptureV1 = serde_json::from_slice(&retained)?;
    let first = read_retained_tfrrs_list_rows(&original, &fixture::locators())?;
    let second = read_retained_tfrrs_list_rows(&cold, &fixture::locators())?;
    let athlete = fixture::athlete();
    let school = fixture::school();
    for index in 0..2 {
        let source = first.row(index).ok_or("original source")?;
        let subject = source.subject()?.reference();
        let target = fixture::target(source.locator(), &subject, &athlete, &school);
        let first_output = project_retained_tfrrs_result_row(&first, index, &target)?;
        let second_output = project_retained_tfrrs_result_row(&second, index, &target)?;
        check!(eq; serde_json::to_vec(&first_output)?, serde_json::to_vec(&second_output)?);
        let identity = project_retained_tfrrs_result_identity(&first, index, &athlete)?;
        let replay = project_retained_tfrrs_result_identity(&second, index, &athlete)?;
        check!(eq; identity.performance_id(), replay.performance_id());
        check!(eq; identity.meet_id(), replay.meet_id());
        check!(eq; identity.event_id(), replay.event_id());
        check!(eq; identity.date(), replay.date());
        check!(eq; identity.source_key(), replay.source_key());
        check!(eq; identity.athlete(), &athlete);
        let (meet, row) = first_output;
        check!(eq; identity.performance_id(), &row.proposed_performance().id);
        check!(eq; identity.meet_id(), &meet.id);
        check!(eq; identity.event_id(), &row.proposed_event().id);
        check!(eq; identity.date(), row.proposed_performance().date);
        check!(eq; identity.source_key(), row.proposed_performance().source_key);
    }
    Ok(())
}

#[test]
fn changing_only_proposed_bindings_retains_source_key_and_unadmitted_status() -> TestResult {
    let capture = fixture::capture(fixture::BODY)?;
    let list = read_retained_tfrrs_list_rows(&capture, &fixture::locators())?;
    let source = list.row(0).ok_or("source")?;
    let subject = source.subject()?.reference();
    let first_athlete = fixture::athlete();
    let first_school = fixture::school();
    let second_athlete = AthleteId::mint("ath_subject", &["different proposed identity"]);
    let second_school = SchoolId::mint("sch", &["different proposed school"]);
    let first_target = fixture::target(source.locator(), &subject, &first_athlete, &first_school);
    let second_target =
        fixture::target(source.locator(), &subject, &second_athlete, &second_school);
    let (_, first) = project_retained_tfrrs_result_row(&list, 0, &first_target)?;
    let (_, second) = project_retained_tfrrs_result_row(&list, 0, &second_target)?;
    check!(eq; first.proposed_performance().source_key, second.proposed_performance().source_key);
    check!(first.proposed_performance().id != second.proposed_performance().id);
    for row in [&first, &second] {
        let note = row
            .proposed_performance()
            .evidence
            .first()
            .and_then(|evidence| evidence.note.as_deref())
            .ok_or("unadmitted source note")?;
        let note: serde_json::Value = serde_json::from_str(note)?;
        check!(eq; note["admission"], "unadmitted_projection");
    }
    Ok(())
}
