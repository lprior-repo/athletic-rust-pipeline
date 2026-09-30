use super::spans;

#[test]
fn plain_text_without_markup_keeps_its_lines() {
    assert_eq!(
        spans("first\n\nsecond\n"),
        (
            vec!["first".to_string(), "second".to_string()],
            String::new()
        )
    );
}

#[test]
fn table_rows_are_records_and_headings_are_read_once() {
    let (records, heading) = spans(
        "<h1>Mosinee High School WI</h1><table><tr><td>Dana Reid</td><td>Head XC Coach</td></tr></table>",
    );
    assert_eq!(records, vec!["Dana Reid Head XC Coach".to_string()]);
    assert_eq!(heading, "mosinee high school wi");
}

#[test]
fn a_record_containing_another_record_is_not_reported() {
    let (records, _) = spans("<article><li>Dana Reid</li>Head XC Coach dana@example.org</article>");
    assert_eq!(records, vec!["Dana Reid".to_string()]);
}

#[test]
fn unclosed_items_split_on_the_next_item() {
    let (records, _) = spans("<ul><li>Dana<li>Other Person dana@example.org</ul>");
    assert_eq!(
        records,
        vec![
            "Dana".to_string(),
            "Other Person dana@example.org".to_string()
        ]
    );
}

#[test]
fn unclosed_rows_split_on_the_next_row() {
    let (records, _) =
        spans("<table><tr><td>Dana Reid<tr><td>Head XC Coach dana@example.org</table>");
    assert_eq!(
        records,
        vec![
            "Dana Reid".to_string(),
            "Head XC Coach dana@example.org".to_string()
        ]
    );
}

#[test]
fn class_tokens_are_records_wherever_they_appear() {
    let (records, _) = spans(
        "<div class='staff-card'>Dana Reid</div><div class=\"coach-card\">Head XC Coach</div>",
    );
    assert_eq!(
        records,
        vec!["Dana Reid".to_string(), "Head XC Coach".to_string()]
    );
}

#[test]
fn paragraphs_are_the_fallback_when_no_record_exists() {
    let (records, _) = spans("<div><p>Dana Reid</p><p>Head XC Coach</p></div>");
    assert_eq!(
        records,
        vec!["Dana Reid".to_string(), "Head XC Coach".to_string()]
    );
}

#[test]
fn entity_references_decode() {
    let (records, _) = spans("<table><tr><td>Dana&nbsp;Reid &amp; Head XC Coach</td></tr></table>");
    assert_eq!(records, vec!["Dana\u{a0}Reid & Head XC Coach".to_string()]);
}

#[test]
fn script_text_is_not_treated_as_markup() {
    let (records, heading) = spans(
        "<html><head><title>Mosinee High School WI</title><script>var x = \"<tr>fake</tr>\";</script></head><body><p>Dana Reid Head XC Coach</p></body></html>",
    );
    assert_eq!(records, vec!["Dana Reid Head XC Coach".to_string()]);
    assert_eq!(heading, "mosinee high school wi");
}
