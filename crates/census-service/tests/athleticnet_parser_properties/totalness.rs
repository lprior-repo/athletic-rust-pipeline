use super::{arbitrary_token, number_of, parse_mark, seam_config, shaped_token};
use census_domain::model::EventKind;
use proptest::prelude::*;

fn kinds() -> [EventKind; 3] {
    [
        EventKind::CrossCountry,
        EventKind::ShotPut,
        EventKind::Decathlon,
    ]
}

proptest! {
    #![proptest_config(seam_config())]

    #[test]
    fn arbitrary_tokens_answer_or_refuse(token in arbitrary_token()) {
        for kind in kinds() {
            let _ = parse_mark(&kind, &token);
        }
    }

    #[test]
    fn mark_shaped_tokens_answer_or_refuse(token in shaped_token()) {
        for kind in kinds() {
            let _ = parse_mark(&kind, &token);
        }
    }

    #[test]
    fn an_accepted_mark_is_finite(token in shaped_token()) {
        for kind in kinds() {
            if let Some((mark, _)) = parse_mark(&kind, &token) {
                if let Some(value) = number_of(&mark) {
                    prop_assert!(
                        value.is_finite() && value >= 0.0,
                        "{kind:?} accepted {token:?} as {value}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_same_token_gives_the_same_mark(token in shaped_token()) {
        for kind in kinds() {
            prop_assert_eq!(parse_mark(&kind, &token), parse_mark(&kind, &token));
        }
    }
}

#[test]
fn figures_that_are_not_finite_are_never_marks() {
    let overlong = "9".repeat(320);
    for token in ["1e400", "1e400m", overlong.as_str()] {
        for kind in kinds() {
            assert!(
                parse_mark(&kind, token).is_none(),
                "{kind:?} read {token:?} as a mark: {:?}",
                parse_mark(&kind, token)
            );
        }
    }
}
