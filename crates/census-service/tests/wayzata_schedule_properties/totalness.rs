use super::{arbitrary_body, rows, seam_config, shaped_body};
use proptest::prelude::*;

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
