use super::super::super::{codec, RetainedHtmlCaptureV1, MAX_PACKET};
use super::*;

#[test]
fn original_capture_replay_preserves_all_proposed_rows_and_provenance() -> TestResult {
    let capture = fixture::capture()?;
    let encoded = codec::encode(&capture, MAX_PACKET)?;
    let reopened: RetainedHtmlCaptureV1 = serde_json::from_slice(&encoded)?;
    let before = read_retained_html_result_rows(&capture, &fixture::locators())?;
    let after = read_retained_html_result_rows(&reopened, &fixture::locators())?;
    let subjects = fixture::subjects(&before)?;
    let athlete = fixture::athlete();
    let school = fixture::school();
    let targets = fixture::targets(&subjects, &athlete, &school);
    let first = project_retained_html_results(&before, &targets)?;
    let second = project_retained_html_results(&after, &targets)?;
    check!(eq; first.proposed_meet(), second.proposed_meet());
    check!(eq; codec::encode(&first.rows(), MAX_PACKET)?, codec::encode(&second.rows(), MAX_PACKET)?);
    check!(eq; first.document().parsed_page(), second.document().parsed_page());
    check!(eq; first.document().context_reference(), second.document().context_reference());
    Ok(())
}

#[test]
fn changed_proposed_attachment_remains_untrusted_and_never_changes_source_content_identity(
) -> TestResult {
    let capture = fixture::capture()?;
    let document = read_retained_html_result_rows(&capture, &fixture::locators())?;
    let subjects = fixture::subjects(&document)?;
    let athlete = fixture::athlete();
    let school = fixture::school();
    let mut targets = fixture::targets(&subjects, &athlete, &school);
    let before = project_retained_html_results(&document, &targets)?;
    targets
        .get_mut(2)
        .ok_or("last target")?
        .proposed_attachment_reference = fixture::SCHOOL;
    let after = project_retained_html_results(&document, &targets)?;
    let before = before.rows().last().ok_or("before row")?;
    let after = after.rows().last().ok_or("after row")?;
    check!(eq; before.proposed_performance().id, after.proposed_performance().id);
    check!(eq; before.proposed_performance().source_key, after.proposed_performance().source_key);
    check!(ne; before.proposed_attachment_reference(), after.proposed_attachment_reference());
    check!(ne; before.proposed_performance().evidence, after.proposed_performance().evidence);
    Ok(())
}

#[test]
fn changed_transport_changes_context_but_preserves_content_key() -> TestResult {
    let original = fixture::capture()?;
    let changed = fixture::capture_with(fixture::BODY, "synthetic-provenance-control/1")?;
    let before = read_retained_html_result_rows(&original, &fixture::locators())?;
    let after = read_retained_html_result_rows(&changed, &fixture::locators())?;
    check!(ne; before.context_reference(), after.context_reference());
    check!(eq; before.capture().body_sha256(), after.capture().body_sha256());
    let subjects = fixture::subjects(&before)?;
    let athlete = fixture::athlete();
    let school = fixture::school();
    let targets = fixture::targets(&subjects, &athlete, &school);
    let first = project_retained_html_results(&before, &targets)?;
    let second = project_retained_html_results(&after, &targets)?;
    for (left, right) in first.rows().iter().zip(second.rows()) {
        check!(eq; left.proposed_performance().source_key, right.proposed_performance().source_key);
        check!(ne; left.proposed_performance().evidence, right.proposed_performance().evidence);
    }
    Ok(())
}

#[test]
fn changed_original_bytes_cannot_reuse_an_old_exact_subject_target() -> TestResult {
    let original = fixture::capture()?;
    let before = read_retained_html_result_rows(&original, &fixture::locators())?;
    let mut body = fixture::BODY.to_vec();
    body.extend_from_slice(b"<!-- synthetic late content control -->");
    let changed = fixture::capture_with(&body, "OpenAI-research/0.1")?;
    let after = read_retained_html_result_rows(&changed, &fixture::locators())?;
    let subjects = fixture::subjects(&before)?;
    let athlete = fixture::athlete();
    let school = fixture::school();
    let targets = fixture::targets(&subjects, &athlete, &school);
    check!(matches!(
        project_retained_html_results(&after, &targets),
        Err(HtmlResultProjectionError::Invalid(
            "target differs from exact checked row subject"
        ))
    ));
    let current = fixture::subjects(&after)?;
    let targets = fixture::targets(&current, &athlete, &school);
    let output = project_retained_html_results(&after, &targets)?;
    check!(ne; output.rows().first().ok_or("row")?.proposed_performance().source_key, "milesplit_html_row_v1:947d8be3d1cc9625154b5ebf345c4e4a578dee9640a4852ea26ccbad80a34849");
    Ok(())
}
