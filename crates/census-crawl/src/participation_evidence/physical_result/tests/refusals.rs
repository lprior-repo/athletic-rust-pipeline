use super::*;

#[test]
fn missing_duplicate_reordered_and_late_conflicting_targets_refuse_the_whole_projection(
) -> TestResult {
    let capture = fixture::capture()?;
    let document = read_retained_html_result_rows(&capture, &fixture::locators())?;
    let subjects = fixture::subjects(&document)?;
    let athlete = fixture::athlete();
    let school = fixture::school();
    let targets = fixture::targets(&subjects, &athlete, &school);
    check!(project_retained_html_results(&document, &[]).is_err());
    check!(project_retained_html_results(&document, targets.get(..2).ok_or("prefix")?).is_err());
    let mut changed = targets.clone();
    let first = *changed.first().ok_or("first target")?;
    *changed.get_mut(2).ok_or("last target")? = first;
    check!(project_retained_html_results(&document, &changed).is_err());
    changed = targets.clone();
    changed.reverse();
    check!(project_retained_html_results(&document, &changed).is_err());
    changed = targets.clone();
    changed.get_mut(2).ok_or("last target")?.subject_reference = fixture::ATTACHMENT;
    check!(matches!(
        project_retained_html_results(&document, &changed),
        Err(HtmlResultProjectionError::Invalid(
            "target differs from exact checked row subject"
        ))
    ));
    changed = targets.clone();
    changed.get_mut(2).ok_or("last target")?.locator.byte_length = usize::MAX;
    check!(project_retained_html_results(&document, &changed).is_err());
    check!(eq; project_retained_html_results(&document, &targets)?.rows().len(), 3);
    Ok(())
}

#[test]
fn invalid_untrusted_join_references_and_id_kinds_refuse() -> TestResult {
    let capture = fixture::capture()?;
    let document = read_retained_html_result_rows(&capture, &fixture::locators())?;
    let subjects = fixture::subjects(&document)?;
    let athlete = fixture::athlete();
    let school = fixture::school();
    let mut targets = fixture::targets(&subjects, &athlete, &school);
    targets
        .get_mut(2)
        .ok_or("target")?
        .proposed_attachment_reference = "caller accepted";
    check!(project_retained_html_results(&document, &targets).is_err());
    let wrong_kind: AthleteId = serde_json::from_str("\"ath_0000000000000000\"")?;
    let targets = fixture::targets(&subjects, &wrong_kind, &school);
    check!(matches!(
        project_retained_html_results(&document, &targets),
        Err(HtmlResultProjectionError::Invalid(
            "proposed canonical ID kind"
        ))
    ));
    Ok(())
}

#[test]
fn exact_count_and_encoded_estimate_caps_refuse_one_over_without_raising_limits() -> TestResult {
    check!(admission::count(64).is_ok());
    check!(matches!(
        admission::count(65),
        Err(HtmlResultProjectionError::Budget {
            resource: "selected rows",
            needed: 65,
            limit: 64
        })
    ));
    let limit = super::super::super::MAX_PACKET;
    check!(admission::bound(limit, limit, "estimated encoded output bytes").is_ok());
    let over = limit.checked_add(1).ok_or("overflow")?;
    check!(matches!(
        admission::bound(over, limit, "estimated encoded output bytes"),
        Err(HtmlResultProjectionError::Budget {
            resource: "estimated encoded output bytes",
            ..
        })
    ));
    Ok(())
}
