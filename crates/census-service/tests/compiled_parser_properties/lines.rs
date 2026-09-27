
use super::{lines, parse_body, parse_lines, rendered_reading, seam_config, LAYOUTS, REGIONAL};
use census_crawl::hytek::{lines_from_pdf_text, lines_from_text};
use proptest::prelude::*;

const CAPTURES: [(&str, &str); 2] = LAYOUTS;

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

fn readings(body: &str) -> Option<Vec<Vec<String>>> {
    parse_body(body).map(|meet| {
        meet.events
            .iter()
            .map(|event| event.rows.iter().map(rendered_reading).collect())
            .collect()
    })
}

proptest! {
    #![proptest_config(seam_config())]

    #[test]
    fn a_form_feed_splits_the_page_where_a_newline_does(marked in 0usize..40) {
        prop_assert_eq!(
            lines_from_pdf_text(&with_form_feed(REGIONAL, marked)),
            lines_from_pdf_text(REGIONAL),
            "a form feed after line {} is a page break",
            marked
        );
    }

    #[test]
    fn carriage_returns_are_not_part_of_a_line(capture in 0usize..2) {
        let (name, body) = CAPTURES[capture];
        let windows = body.replace('\n', "\r\n");
        prop_assert_eq!(
            lines_from_pdf_text(&windows),
            lines_from_pdf_text(body),
            "{}: the PDF front end reads a Windows capture as the same lines",
            name
        );
        prop_assert_eq!(
            lines_from_text(&windows),
            lines_from_text(body),
            "{}: and so does the plain-text front end",
            name
        );
        prop_assert_eq!(
            lines(&windows),
            lines(body),
            "{}: which is the splitter the lane parses through",
            name
        );
    }

    #[test]
    fn trailing_padding_never_reaches_a_reading(
        capture in 0usize..2,
        stride in 1usize..8,
        pad in 0usize..5,
    ) {
        let (name, body) = CAPTURES[capture];
        let padded = with_trailing_padding(body, stride, PADDING[pad]);
        prop_assert_eq!(
            parse_lines(&lines_from_pdf_text(&padded)).map(|meet| meet.rows_parsed),
            parse_lines(&lines_from_pdf_text(body)).map(|meet| meet.rows_parsed),
            "{}: padding with {:?} adds and drops no row",
            name,
            PADDING[pad]
        );
        prop_assert_eq!(
            readings(&padded),
            readings(body),
            "{}: padding with {:?} reaches no reading",
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
