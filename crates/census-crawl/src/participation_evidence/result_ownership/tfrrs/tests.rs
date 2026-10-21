use super::*;
use fixture::{capture, locators, TestResult, BODY};

mod adverse;
mod calendar;
mod fixture;

#[test]
fn genuine_full_capture_retains_native_justin_conversion_and_exact_original_row() -> TestResult {
    let capture = capture(BODY)?;
    check!(eq; capture.body_sha256(), "053ff7832326a6d27e2456155db7a9530682393fae94a9fcb9ccc9e88bfc8407");
    let document = read_retained_tfrrs_list_rows(&capture, &locators())?;
    check!(eq; document.original_document().as_bytes(), BODY);
    check!(eq; document.list_id(), "5489");
    check!(eq; document.year(), 2026);
    check!(eq; document.sport(), Sport::IndoorTrack);
    check!(eq; document.selected_count(), 2);
    let row = document.row(0).ok_or("checked Justin row")?;
    check!(eq; row.locator(), locators()[0]);
    check!(eq; row.original_row_digest(), "c6809a7ceb8c2c8d70c5c8158b96eb13e3aaa881be19d866db75539cd2f5caf4");
    check!(eq; row.athlete_id(), "8650211");
    check!(eq; row.athlete_name(), "Justin Reckers");
    check!(eq; row.team_path(), "/teams/tf/Columbus_North_m.html");
    check!(eq; row.team_name(), "Columbus North");
    check!(eq; row.grade().get(), 11);
    check!(eq; row.published_year(), "JR");
    check!(eq; row.meet_id(), "95201");
    check!(eq; row.meet_name(), "ISU HSR Last Chance #2 and MS Championships");
    check!(eq; row.published_date(), "2026-03-21");
    check!(eq; (row.year(), row.month(), row.day()), (2026, 3, 21));
    check!(eq; row.event_label(), "3200 Meters");
    check!(eq; row.event_handle(), 61);
    check!(eq; row.gender(), Gender::Boys);
    check!(eq; row.performance_id(), "5862520");
    check!(eq; row.published_mark(), "9:15.75");
    check!(eq; row.original_mark(), Some("9:22.21"));
    check!(eq; row.conversion_note(), Some("Converted from 9:22.21 for Track Size."));
    check!(row.original_row()?.contains("_Justin_Reckers"));
    let mark = document
        .original_document()
        .get(row.published_mark_span())
        .ok_or("mark span")?;
    check!(mark.contains(">9:15.75</a>"));
    let note = document
        .original_document()
        .get(row.conversion_note_span().ok_or("note span")?)
        .ok_or("note source")?;
    check!(note.contains("Converted from 9:22.21 for Track Size."));
    let section = row.section_span();
    check!(section.start < row.event_header_span().start);
    check!(section.end > row.locator().byte_offset + row.locator().byte_length);
    Ok(())
}

#[test]
fn list_rows_keep_their_own_meet_and_date_instead_of_one_page_meet() -> TestResult {
    let capture = capture(BODY)?;
    let document = read_retained_tfrrs_list_rows(&capture, &locators())?;
    let first = document.row(0).ok_or("Justin")?;
    let second = document.row(1).ok_or("Isaac")?;
    check!(eq; second.athlete_id(), "8792017");
    check!(eq; second.athlete_name(), "Isaac Smith");
    check!(eq; second.team_name(), "Hamilton Southeastern");
    check!(eq; second.meet_id(), "93917");
    check!(eq; second.meet_name(), "Brownsburg HCC Indoor");
    check!(eq; second.published_date(), "2026-03-11");
    check!(eq; second.published_mark(), "9:16.30");
    check!(eq; second.original_mark(), Some("9:22.77"));
    check!(eq; second.original_row_digest(), "595aa36734f340a2a4becf951002d8a5a2984adef7204d5f49abf52fd9e78ad4");
    check!(eq; first.section_span(), second.section_span());
    check!(first.meet_id() != second.meet_id());
    check!(first.context_reference() != second.context_reference());
    check!(first.reference() != second.reference());
    Ok(())
}

#[test]
fn retained_capture_cold_roundtrip_recomputes_identical_native_ownership() -> TestResult {
    let original = capture(BODY)?;
    let encoded = serde_json::to_vec(&original)?;
    let decoded: RetainedHtmlCaptureV1 = serde_json::from_slice(&encoded)?;
    let first = read_retained_tfrrs_list_rows(&original, &locators())?;
    let cold = read_retained_tfrrs_list_rows(&decoded, &locators())?;
    check!(eq; first.context_reference(), cold.context_reference());
    check!(eq; first.original_document(), cold.original_document());
    for index in 0..2 {
        let first = first.row(index).ok_or("original row")?;
        let cold = cold.row(index).ok_or("cold row")?;
        check!(eq; first.reference(), cold.reference());
        check!(eq; first.context_reference(), cold.context_reference());
        check!(eq; first.original_row_digest(), cold.original_row_digest());
        check!(eq; first.conversion_note_span(), cold.conversion_note_span());
        check!(eq; first.original_mark(), cold.original_mark());
    }
    Ok(())
}

#[test]
fn changed_retained_capture_digest_is_rejected_before_interpretation() -> TestResult {
    let original = capture(BODY)?;
    let mut encoded = serde_json::to_value(&original)?;
    let field = encoded
        .get_mut("body_sha256")
        .ok_or("capture digest field")?;
    *field = serde_json::Value::String("0".repeat(64));
    let changed: RetainedHtmlCaptureV1 = serde_json::from_value(encoded)?;
    check!(matches!(
        read_retained_tfrrs_list_rows(&changed, &locators()),
        Err(RetainedResultOwnershipError::Capture(_))
    ));
    Ok(())
}

#[test]
fn selected_locators_reject_duplicate_unordered_overlapping_subspan_and_capacity() -> TestResult {
    let capture = capture(BODY)?;
    let [first, second] = locators();
    for requested in [vec![first, first], vec![second, first], vec![]] {
        check!(read_retained_tfrrs_list_rows(&capture, &requested).is_err());
    }
    let mut subspan = first;
    subspan.byte_offset += 1;
    subspan.byte_length -= 1;
    check!(read_retained_tfrrs_list_rows(&capture, &[subspan]).is_err());
    let mut ordinal = first;
    ordinal.ordinal += 1;
    check!(read_retained_tfrrs_list_rows(&capture, &[ordinal]).is_err());
    check!(matches!(
        read_retained_tfrrs_list_rows(&capture, &vec![first; 65]),
        Err(RetainedResultOwnershipError::Capacity { .. })
    ));
    Ok(())
}
