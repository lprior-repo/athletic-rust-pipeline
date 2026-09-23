//! The front end a Compiled export is read through: what a capture's own furniture is, and what it is
//! not.
//!
//! Compiled exports arrive as `pdftotext -layout` captures of the timer's print — one page, two event
//! blocks side by side — and every column this seam reads is an offset into a line of that capture
//! ([`lines_from_pdf_text`]). Two consequences follow, and both are asserted here: a page break the
//! capture wrote as a form feed separates the pages exactly where a newline would (a page footer must
//! not glue itself onto the next page's first block), and the trailing padding a timer prints to widen
//! a column can never be read as part of a school, an athlete or a mark.
//!
//! The reading law is stated over the whole entry point rather than over the splitter alone: the
//! compiled reader slices a line by the anchors of its column header, so "padding changes nothing" is a
//! claim about the columns it anchors, not only about the bytes it holds.

use super::{lines, parse_body, parse_lines, rendered_reading, seam_config, LAYOUTS, REGIONAL};
use census_crawl::hytek::{lines_from_pdf_text, lines_from_text};
use proptest::prelude::*;

/// The two committed layouts of this seam, as `crates/census-crawl/src/compiled/tests.rs` parses them.
const CAPTURES: [(&str, &str); 2] = LAYOUTS;

/// What a timer pads a line's tail with: blanks, a tab, and the no-break space in both the escaped and
/// the decoded form a capture can carry.
const PADDING: [&str; 5] = [" ", "\t", "  ", "&nbsp;", "\u{a0}"];

/// A body whose separator after line `marked` is a form feed, the way a PDF capture carries a page
/// break. `marked` is taken modulo the line count, so every break of a capture is reached.
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

/// A body with `pad` appended to every `stride`th line's tail: the padding sits to the right of every
/// column the page prints, which is where a timer puts it.
///
/// Only a line that carries something is padded: padding the tail of a blank line would manufacture a
/// line the capture never carried, which is not a column a timer printed.
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

/// Every reading of a meet, event by event, so two parses of one page compare as values.
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

    /// A page break is a line break, whichever byte the capture wrote it with.
    #[test]
    fn a_form_feed_splits_the_page_where_a_newline_does(marked in 0usize..40) {
        prop_assert_eq!(
            lines_from_pdf_text(&with_form_feed(REGIONAL, marked)),
            lines_from_pdf_text(REGIONAL),
            "a form feed after line {} is a page break",
            marked
        );
    }

    /// A carriage return is line-end furniture: a capture taken on Windows reads as the same lines as
    /// one taken on Unix, on both front ends.
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

    /// Padding widens a column; it is never part of a reading. Every row of every block is the row the
    /// unpadded page prints, field for field.
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

    /// One reader, two front ends: a body with no page break in it is split into the same lines by the
    /// PDF front end and the plain-text one, so the two entry points cannot drift apart.
    #[test]
    fn the_two_front_ends_agree_on_a_body_with_no_page_break(body in super::arbitrary_body()) {
        let body = body.replace('\u{c}', "\n");
        prop_assert_eq!(lines_from_pdf_text(&body), lines_from_text(&body));
    }
}
