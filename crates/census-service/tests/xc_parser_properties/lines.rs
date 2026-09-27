use super::{parse_lines, rendered_rows, seam_config, ACCURACE, STATE, TABLE};
use census_crawl::hytek::{lines_from_pdf_text, lines_from_text};
use proptest::prelude::*;

const CAPTURES: [(&str, &str); 3] = [
    ("team blocks (state meet)", STATE),
    ("padded grade table (sectional)", TABLE),
    ("rule-lined table (AccuRace sectional)", ACCURACE),
];

const PADDING: [&str; 5] = [" ", "\t", "  ", "&nbsp;", "\u{a0}"];

fn with_form_feed(body: &str, marked: usize) -> String {
    let parts: Vec<&str> = body.split('\n').collect();
    let breaks = parts.len().saturating_sub(1);
    let marked = if breaks == 0 { 0 } else { marked % breaks + 1 };
    let mut out = String::with_capacity(body.len());
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            out.push(if index == marked { '\u{c}' } else { '\n' });
        }
        out.push_str(part);
    }
    out
}

fn with_carriage_returns(body: &str) -> String {
    body.replace('\n', "\r\n")
}

fn with_trailing_padding(body: &str, stride: usize, pad: &str) -> String {
    let stride = stride.max(1);
    let mut out = String::with_capacity(body.len());
    for (index, part) in body.split('\n').enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str(part);
        if index % stride == 0 && !part.trim().is_empty() {
            out.push_str(pad);
        }
    }
    out
}

proptest! {
    #![proptest_config(seam_config())]

    #[test]
    fn a_form_feed_splits_the_page_where_a_newline_does(capture in 0usize..3, marked in 0usize..80) {
        let (name, body) = CAPTURES[capture];
        prop_assert_eq!(
            lines_from_pdf_text(&with_form_feed(body, marked)),
            lines_from_pdf_text(body),
            "{}: a form feed after line {} is a page break",
            name,
            marked
        );
    }

    #[test]
    fn carriage_returns_are_not_part_of_a_line(capture in 0usize..3) {
        let (name, body) = CAPTURES[capture];
        prop_assert_eq!(
            lines_from_pdf_text(&with_carriage_returns(body)),
            lines_from_pdf_text(body),
            "{}: the PDF front end reads a Windows capture as the same lines",
            name
        );
        prop_assert_eq!(
            lines_from_text(&with_carriage_returns(body)),
            lines_from_text(body),
            "{}: and so does the plain-text front end",
            name
        );
    }

    #[test]
    fn trailing_padding_never_reaches_a_record(
        capture in 0usize..3,
        stride in 1usize..8,
        pad in 0usize..5,
    ) {
        let (name, body) = CAPTURES[capture];
        let padded = with_trailing_padding(body, stride, PADDING[pad]);
        let full = lines_from_pdf_text(body);
        let padded_lines = lines_from_pdf_text(&padded);
        prop_assert_eq!(
            padded_lines.len(),
            full.len(),
            "{}: padding neither splits nor joins a line",
            name
        );
        prop_assert_eq!(
            parse_lines(&padded_lines).map(|meet| rendered_rows(&meet)),
            parse_lines(&full).map(|meet| rendered_rows(&meet)),
            "{}: padding with {:?} reaches no row",
            name,
            PADDING[pad]
        );
    }

    #[test]
    fn the_two_front_ends_agree_on_a_body_with_no_page_break(body in super::arbitrary_body()) {
        let body = body.replace('\u{c}', "\n");
        prop_assert_eq!(lines_from_pdf_text(&body), lines_from_text(&body));
    }
}
