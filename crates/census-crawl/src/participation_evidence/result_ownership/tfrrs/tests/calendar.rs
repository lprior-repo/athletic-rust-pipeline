use super::super::calendar::read_checked_tfrrs_row_calendar;
use super::*;
use census_domain::model::ProfileDate;

#[test]
fn genuine_calendar_context_preserves_each_native_meet_and_exact_source_day() -> TestResult {
    let capture = capture(BODY)?;
    let document = read_retained_tfrrs_list_rows(&capture, &locators())?;
    let first = read_checked_tfrrs_row_calendar(&document, 0)?;
    let second = read_checked_tfrrs_row_calendar(&document, 1)?;
    check!(std::ptr::eq(first.capture(), &capture));
    check!(eq; first.capture().body_sha256(), "053ff7832326a6d27e2456155db7a9530682393fae94a9fcb9ccc9e88bfc8407");
    check!(eq; first.list_context_reference(), document.context_reference());
    check!(eq; second.list_context_reference(), document.context_reference());
    check!(eq; first.row().athlete_name(), "Justin Reckers");
    check!(eq; first.row().team_name(), "Columbus North");
    check!(eq; first.row().meet_id(), "95201");
    check!(eq; first.row().meet_name(), "ISU HSR Last Chance #2 and MS Championships");
    check!(eq; first.day(), &ProfileDate::parse("2026-03-21")?);
    check!(eq; second.row().meet_id(), "93917");
    check!(eq; second.row().meet_name(), "Brownsburg HCC Indoor");
    check!(eq; second.day(), &ProfileDate::parse("2026-03-11")?);
    check!(ne; first.day(), second.day());
    check!(ne; first.row().context_reference(), second.row().context_reference());
    check!(ne; first.row().context_reference(), first.list_context_reference());
    for calendar in [&first, &second] {
        check!(eq; calendar.school_year().get(), 2025);
        check!(eq; calendar.sport(), Sport::IndoorTrack);
        check!(eq; calendar.category(), Gender::Boys);
    }
    Ok(())
}

#[test]
fn calendar_proof_spans_are_inside_the_exact_original_selected_row() -> TestResult {
    let capture = capture(BODY)?;
    let document = read_retained_tfrrs_list_rows(&capture, &locators())?;
    for (index, day, native) in [
        (
            0,
            "Mar 21, 2026",
            "/results/95201/ISU_HSR_Last_Chance_2_and_MS_Championships",
        ),
        (1, "Mar 11, 2026", "/results/93917/Brownsburg_HCC_Indoor"),
    ] {
        let calendar = read_checked_tfrrs_row_calendar(&document, index)?;
        let row = calendar.row();
        let locator = row.locator();
        let end = locator
            .byte_offset
            .checked_add(locator.byte_length)
            .ok_or("row end")?;
        for span in [row.meet_span(), row.date_span()] {
            check!(span.start >= locator.byte_offset);
            check!(span.end <= end);
        }
        let raw_date = document
            .original_document()
            .get(row.date_span())
            .ok_or("date source")?;
        check!(eq; raw_date, format!("<div class=\"col-narrow\" data-label=\"Meet Date\">{day}</div>"));
        let meet = document
            .original_document()
            .get(row.meet_span())
            .ok_or("meet source")?;
        check!(meet.contains(&format!("href=\"https://indiana.tfrrs.org{native}\"")));
        check!(meet.contains(row.meet_name()));
        check!(eq; row.reference(), document.row(index).ok_or("selected row")?.reference());
        check!(eq; row.original_row_digest(), document.row(index).ok_or("selected row")?.original_row_digest());
    }
    Ok(())
}

#[test]
fn cold_capture_recomputes_the_same_calendar_context_without_new_preimages() -> TestResult {
    let original = capture(BODY)?;
    let decoded: RetainedHtmlCaptureV1 = serde_json::from_slice(&serde_json::to_vec(&original)?)?;
    let first = read_retained_tfrrs_list_rows(&original, &locators())?;
    let cold = read_retained_tfrrs_list_rows(&decoded, &locators())?;
    check!(eq; first.original_document(), cold.original_document());
    for index in 0..first.selected_count() {
        let first = read_checked_tfrrs_row_calendar(&first, index)?;
        let cold = read_checked_tfrrs_row_calendar(&cold, index)?;
        check!(eq; first.capture().body_sha256(), cold.capture().body_sha256());
        check!(eq; first.list_context_reference(), cold.list_context_reference());
        check!(eq; first.day(), cold.day());
        check!(eq; first.school_year(), cold.school_year());
        check!(eq; first.sport(), cold.sport());
        check!(eq; first.category(), cold.category());
        check!(eq; first.row().reference(), cold.row().reference());
        check!(eq; first.row().context_reference(), cold.row().context_reference());
        check!(eq; first.row().original_row_digest(), cold.row().original_row_digest());
        check!(eq; first.row().meet_span(), cold.row().meet_span());
        check!(eq; first.row().date_span(), cold.row().date_span());
    }
    Ok(())
}

#[test]
fn absent_selected_row_refuses_without_substituting_an_adjacent_calendar() -> TestResult {
    let capture = capture(BODY)?;
    let [first, _] = locators();
    let document = read_retained_tfrrs_list_rows(&capture, &[first])?;
    for index in [1, usize::MAX] {
        check!(matches!(
            read_checked_tfrrs_row_calendar(&document, index),
            Err(RetainedResultOwnershipError::Invalid(
                "TFRRS calendar selected row absent"
            ))
        ));
    }
    check!(eq; read_checked_tfrrs_row_calendar(&document, 0)?.day().as_str(), "2026-03-21");
    Ok(())
}

#[test]
fn wrong_year_hidden_date_or_changed_capture_digest_refuses_before_calendar_projection(
) -> TestResult {
    for (from, to) in [
        ("Mar 21, 2026", "Mar 21, 2025"),
        ("Mar 21, 2026", "Feb 30, 2026"),
        (
            "data-label=\"Meet Date\"",
            "hidden data-label=\"Meet Date\"",
        ),
    ] {
        let (changed, locator) = fixture::first_replaced(from, to)?;
        check!(read_retained_tfrrs_list_rows(&changed, &[locator]).is_err());
    }
    let original = capture(BODY)?;
    let mut encoded = serde_json::to_value(&original)?;
    *encoded.get_mut("body_sha256").ok_or("body digest")? = serde_json::json!("0".repeat(64));
    let changed: RetainedHtmlCaptureV1 = serde_json::from_value(encoded)?;
    check!(matches!(
        read_retained_tfrrs_list_rows(&changed, &locators()),
        Err(RetainedResultOwnershipError::Capture(_))
    ));
    Ok(())
}
