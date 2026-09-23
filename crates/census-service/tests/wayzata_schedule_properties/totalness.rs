//! Total-ness and determinism: the two laws every consumer of this reader is allowed to assume.
//!
//! A reader that panics on a malformed page takes the whole run down with it — a weekly walk visits
//! every published schedule — and a reader that answers differently on the same bytes makes a
//! journaled rerun meaningless. Both are asserted over arbitrary markup and over markup woven out of
//! the schedule's own tokens.

use super::{arbitrary_body, rows, seam_config, shaped_body};
use proptest::prelude::*;

/// An empty page is an empty schedule, not an error: the collector journals a page that published
/// nothing and keeps its cursor, and a typed error here would make an uneventful season look broken.
#[test]
fn a_page_with_no_competition_rows_is_an_empty_schedule() {
    let empty = rows("<html><body><table></table></body></html>", 2026)
        .expect("a page with no rows is still a page");
    assert!(
        empty.is_empty(),
        "a schedule without competitions yields no rows: {empty:?}"
    );
}

proptest! {
    #![proptest_config(seam_config())]

    #[test]
    fn a_body_of_arbitrary_markup_parses_or_is_refused(body in arbitrary_body()) {
        let _ = format!("{:?}", rows(&body, 2026));
    }

    #[test]
    fn a_body_of_shaped_markup_parses_or_is_refused(body in shaped_body()) {
        let _ = format!("{:?}", rows(&body, 2026));
    }

    #[test]
    fn the_same_body_gives_the_same_answer(body in shaped_body()) {
        prop_assert_eq!(
            format!("{:?}", rows(&body, 2026)),
            format!("{:?}", rows(&body, 2026))
        );
    }
}
