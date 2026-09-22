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

/// A page whose header the reader cannot place is not claimed: the archive's index pages and the
/// meet's own program are not meets, and an event below a missing header cannot be attributed to a
/// date.
#[test]
fn a_body_without_a_meet_header_is_refused() {
    let block_only = "Girls' 4x800 Relay Division 1                     Finals\n\
                             Team                    Relay        Finals             Pts\n\
                      1      HORTONVILLE             'A'          9:55.11            10";
    assert!(
        parse_body(block_only).is_none(),
        "an event block with no meet header is not a meet"
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
