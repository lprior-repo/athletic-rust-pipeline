use super::{arbitrary_token, metres_of, parse_field_mark, parse_time, seam_config, shaped_token};
use proptest::prelude::*;

proptest! {
    #![proptest_config(seam_config())]

    #[test]
    fn arbitrary_tokens_answer_or_refuse(token in arbitrary_token()) {
        let _ = parse_time(&token);
        let _ = parse_field_mark(&token);
    }

    #[test]
    fn mark_shaped_tokens_answer_or_refuse(token in shaped_token()) {
        let _ = parse_time(&token);
        let _ = parse_field_mark(&token);
    }

    #[test]
    fn an_accepted_time_is_finite_and_non_negative(token in shaped_token()) {
        if let Some(seconds) = parse_time(&token) {
            prop_assert!(
                seconds.value() >= 0,
                "{token:?} was accepted as {seconds}"
            );
        }
    }

    #[test]
    fn an_accepted_mark_is_finite_and_positive(token in shaped_token()) {
        if let Some(mark) = parse_field_mark(&token) {
            let metres = metres_of(&mark);
            prop_assert!(
                metres.is_finite() && metres > 0.0,
                "{token:?} was accepted as {metres} m"
            );
        }
    }

    #[test]
    fn the_same_token_gives_the_same_mark(token in shaped_token()) {
        prop_assert_eq!(parse_time(&token), parse_time(&token));
        prop_assert_eq!(parse_field_mark(&token), parse_field_mark(&token));
    }
}

#[test]
fn figures_that_are_not_finite_are_never_marks() {
    for token in [
        "inf", "-inf", "NaN", "infinity", "1e400", "inf-0", "inf'6\"", "inf:30", "1:inf:00",
        "1:2:inf",
    ] {
        assert!(
            parse_time(token).is_none(),
            "{token:?} was read as a time: {:?}",
            parse_time(token)
        );
        assert!(
            parse_field_mark(token).is_none(),
            "{token:?} was read as a mark: {:?}",
            parse_field_mark(token)
        );
    }
}

#[test]
fn a_negative_or_overlong_field_figure_is_not_a_mark() {
    for token in ["5-12", "5-13", "0-12", "-1-0", "5- 1-", "1e400-1"] {
        assert!(
            parse_field_mark(token).is_none(),
            "{token:?} was read as a mark: {:?}",
            parse_field_mark(token)
        );
    }
}
