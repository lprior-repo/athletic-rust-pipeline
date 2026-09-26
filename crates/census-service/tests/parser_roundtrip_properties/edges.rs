//! Edges: empty, malformed, truncated-by-content and arbitrary bytes.

use super::*;

#[test]
fn no_format_turns_an_empty_or_malformed_body_into_a_meet() {
    for format in FORMATS {
        assert_eq!(parse("", format, ARCHIVE_YEAR), None, "empty as {format:?}");
        assert_eq!(
            parse(ARCHIVE_HTML, format, ARCHIVE_YEAR),
            None,
            "an archive page is not a result file as {format:?}"
        );
    }
}

proptest! {
    #![proptest_config(seam_config())]

    #[test]
    fn arbitrary_bytes_are_never_a_panic_and_always_the_same_answer(
        body in prop::collection::vec(any::<u8>(), 0..512),
        format_index in 0usize..FORMATS.len(),
    ) {
        let format = FORMATS[format_index];
        let attempt = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            parse_result_body(&body, format, source(), ARCHIVE_YEAR)
        }));
        let parsed = attempt.unwrap_or_else(|_| {
            panic!("{} bytes of arbitrary input panicked {format:?}", body.len())
        });
        prop_assert_eq!(
            &parsed,
            &parse_result_body(&body, format, source(), ARCHIVE_YEAR),
            "the seam is a pure function of its inputs"
        );
        if let Some(meet) = parsed {
            prop_assert!(!meet.name.is_empty(), "a parsed meet is named");
            prop_assert!(meet.events.len() <= body.len() + 1, "events come from the bytes");
        }
    }

    #[test]
    fn an_unreadable_or_unsupported_body_is_never_a_meet(
        body in prop::collection::vec(any::<u8>(), 0..512),
        format_index in 0usize..2,
    ) {
        let format = [ArtifactFormat::Pdf, ArtifactFormat::Unparsed][format_index];
        prop_assert_eq!(parse_result_body(&body, format, source(), ARCHIVE_YEAR), None);
    }
}
