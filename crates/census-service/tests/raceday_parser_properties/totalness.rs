use super::{
    arbitrary_body, parse_body, seam_config, shaped_body, ARCHIVE_YEAR, DECLINED_ROW, NO_GRID,
};
use census_crawl::CrawlError;
use proptest::prelude::*;

#[test]
fn a_titled_page_with_no_result_table_is_refused() {
    let error = parse_body(NO_GRID).expect_err("a page with no grid is not a meet");
    assert!(
        matches!(
            &error,
            CrawlError::Schema { detail, .. } if detail.contains("no events")
        ),
        "the refusal names what was missing: {error:?}"
    );
}

#[test]
fn the_archived_season_is_the_meets_year() {
    let parsed = parse_body(DECLINED_ROW).expect("the grid page is a meet");
    assert_eq!(
        parsed.date,
        format!("{ARCHIVE_YEAR:04}"),
        "the reader publishes the archived season at year precision"
    );
}

proptest! {
    #![proptest_config(seam_config())]

    #[test]
    fn a_body_of_arbitrary_markup_parses_or_is_refused(body in arbitrary_body()) {
        let _ = format!("{:?}", parse_body(&body));
    }

    #[test]
    fn a_body_of_shaped_markup_parses_or_is_refused(body in shaped_body()) {
        let _ = format!("{:?}", parse_body(&body));
    }

    #[test]
    fn the_same_body_gives_the_same_answer(body in shaped_body()) {
        prop_assert_eq!(
            format!("{:?}", parse_body(&body)),
            format!("{:?}", parse_body(&body))
        );
    }
}
