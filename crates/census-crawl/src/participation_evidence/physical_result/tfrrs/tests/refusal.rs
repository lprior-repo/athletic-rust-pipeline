use super::*;

#[test]
fn caller_cannot_replace_the_checked_locator_subject_or_reference_shapes() -> TestResult {
    let capture = fixture::capture(fixture::BODY)?;
    let list = read_retained_tfrrs_list_rows(&capture, &fixture::locators())?;
    let source = list.row(0).ok_or("Justin source")?;
    let subject = source.subject()?.reference();
    let other_subject = list.row(1).ok_or("Isaac source")?.subject()?.reference();
    let athlete = fixture::athlete();
    let school = fixture::school();
    let target = fixture::target(source.locator(), &subject, &athlete, &school);
    for change in 0..5 {
        let mut changed = target;
        match change {
            0 => changed.locator.ordinal = 119,
            1 => changed.locator.byte_offset += 1,
            2 => changed.subject_reference = &other_subject,
            3 => changed.proposed_attachment_reference = "unbound attachment",
            _ => changed.proposed_school_binding_reference = "AABB",
        }
        check!(project_retained_tfrrs_result_row(&list, 0, &changed).is_err());
    }
    check!(project_retained_tfrrs_result_row(&list, 1, &target).is_err());
    check!(project_retained_tfrrs_result_row(&list, 2, &target).is_err());
    check!(project_retained_tfrrs_result_identity(&list, 2, &athlete).is_err());
    Ok(())
}

#[test]
fn canonical_kind_and_bounded_reference_shapes_refuse_before_projection() -> TestResult {
    let capture = fixture::capture(fixture::BODY)?;
    let list = read_retained_tfrrs_list_rows(&capture, &fixture::locators())?;
    let source = list.row(0).ok_or("Justin source")?;
    let subject = source.subject()?.reference();
    let athlete = fixture::athlete();
    let school = fixture::school();
    let wrong_athlete = AthleteId::mint("ath", &["wrong subject kind"]);
    let wrong_school = SchoolId::mint("school", &["wrong school kind"]);
    let long = "a".repeat(4097);
    for (proposed_athlete, proposed_school, attachment) in [
        (&wrong_athlete, &school, fixture::ATTACHMENT),
        (&athlete, &wrong_school, fixture::ATTACHMENT),
        (&athlete, &school, long.as_str()),
    ] {
        let mut target = fixture::target(
            source.locator(),
            &subject,
            proposed_athlete,
            proposed_school,
        );
        target.proposed_attachment_reference = attachment;
        check!(project_retained_tfrrs_result_row(&list, 0, &target).is_err());
    }
    check!(project_retained_tfrrs_result_identity(&list, 0, &wrong_athlete).is_err());
    Ok(())
}

#[test]
fn source_owned_unsupported_events_refuse_row_and_identity_projections() -> TestResult {
    let athlete = fixture::athlete();
    let school = fixture::school();
    for label in ["4x400 Relay", "Decathlon", "Mystery Sprint", "Long Jump"] {
        let (capture, locator) = fixture::altered_event(label)?;
        let list = read_retained_tfrrs_list_rows(&capture, &[locator])?;
        let row = list.row(0).ok_or("owned changed event")?;
        check!(eq; row.event_label(), label);
        let subject = row.subject()?.reference();
        let target = fixture::target(locator, &subject, &athlete, &school);
        check!(matches!(
            project_retained_tfrrs_result_row(&list, 0, &target),
            Err(Error::Invalid(_))
        ));
        check!(matches!(
            project_retained_tfrrs_result_identity(&list, 0, &athlete),
            Err(Error::Invalid(_))
        ));
    }
    Ok(())
}
