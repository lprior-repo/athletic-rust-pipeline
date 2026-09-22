//! The front end a cross-country capture is read through: what a page's own furniture is, and what
//! it is not.
//!
//! Every layout this seam reads reaches the census as archived text — a `pdftotext -layout` capture,
//! a plain-text release, a Windows copy of either — and the collector splits it into lines before the
//! parser sees anything ([`lines_from_pdf_text`], [`lines_from_text`]). The lines *are* the record: a
//! page break written as a form feed, a carriage return a capture kept, or the padding a timer prints
//! to widen a column must not become part of a row, because every column this seam reads is measured
//! in offsets of that line.
//!
//! Padding is asserted twice on purpose. The splitting law pins it where it is decided — a padded line
//! is the line it pads — and the reading law pins the consequence the census depends on: a padded
//! capture reads to the rows the unpadded one reads, so no school, grade or time arrives with a
//! column's padding glued to it.

use super::{parse_lines, rendered_rows, seam_config, ACCURACE, STATE, TABLE};
use midwest_census::sources::hytek::{lines_from_pdf_text, lines_from_text};
use proptest::prelude::*;

/// The committed captures of this seam's three layouts, as `src/sources/xc/tests.rs` parses them.
const CAPTURES: [(&str, &str); 3] = [
    ("team blocks (state meet)", STATE),
    ("padded grade table (sectional)", TABLE),
    ("rule-lined table (AccuRace sectional)", ACCURACE),
];

/// What a timer pads a line's tail with: blanks, a tab, and the no-break space in both the escaped
/// and the decoded form a capture can carry.
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

/// A body with every line ending written as `\r\n`, the way a capture taken on Windows arrives.
fn with_carriage_returns(body: &str) -> String {
    body.replace('\n', "\r\n")
}

/// A body with `pad` appended to every `stride`th line's tail.
///
/// Only a line that carries something is padded: a timer widens the column of a row it printed, and
/// padding the tail of a blank line would manufacture a line the capture never carried.
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

    /// A page break is a line break, whichever byte the capture wrote it with: the footer above it
    /// stays on its own line instead of gluing itself onto the next page's first row.
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

    /// A carriage return is line-end furniture, not a character of the line.
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

    /// Trailing padding widens a column; it is never part of the reading. The rows a padded capture
    /// yields are the rows the unpadded one yields, field for field.
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

    /// One reader, two front ends: a body with no page break in it is split into the same lines by
    /// the PDF front end and the plain-text one, so the two entry points cannot drift apart.
    #[test]
    fn the_two_front_ends_agree_on_a_body_with_no_page_break(body in super::arbitrary_body()) {
        let body = body.replace('\u{c}', "\n");
        prop_assert_eq!(lines_from_pdf_text(&body), lines_from_text(&body));
    }
}
