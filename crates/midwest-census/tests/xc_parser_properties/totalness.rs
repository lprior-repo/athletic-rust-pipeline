//! Total-ness and determinism: the two laws every consumer of these parsers is allowed to assume.
//!
//! A parser that panics on a malformed page takes the whole run down with it — a census walk is
//! hundreds of pages nobody has read — and a parser that answers differently on the same lines makes
//! a journaled rerun meaningless. Both are asserted over arbitrary text and over text woven out of
//! the tokens this parser keys on.

use super::{arbitrary_body, parse_body, seam_config, shaped_body};
use proptest::prelude::*;

/// A parse's answer with the meet rendered, so two runs are compared as values.
fn answer(body: &str) -> Option<String> {
    parse_body(body).map(|meet| format!("{meet:?}"))
}

/// The archive's index pages are not result files: a body with no meet header is refused rather than
/// read into an event nobody ran.
#[test]
fn a_body_without_a_meet_header_is_refused() {
    let index_page = "<html><body><h3>2025 Cross Country State Results</h3>\
                      <a href=\"/results/d1.htm\">Division 1</a></body></html>";
    assert!(
        parse_body(index_page).is_none(),
        "an index page with no meet header is not a meet"
    );
}

proptest! {
    #![proptest_config(seam_config())]

    #[test]
    fn a_body_of_arbitrary_text_parses_or_is_refused(body in arbitrary_body()) {
        let _ = answer(&body);
    }

    #[test]
    fn a_body_of_shaped_text_parses_or_is_refused(body in shaped_body()) {
        let _ = answer(&body);
    }

    #[test]
    fn the_same_lines_give_the_same_answer(body in shaped_body()) {
        prop_assert_eq!(answer(&body), answer(&body));
    }
}
