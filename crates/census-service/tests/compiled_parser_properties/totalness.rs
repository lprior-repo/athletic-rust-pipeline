use super::{arbitrary_body, parse_body, seam_config, shaped_body};
use proptest::prelude::*;

fn answer(body: &str) -> Option<String> {
    parse_body(body).map(|meet| format!("{meet:?}"))
}

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
