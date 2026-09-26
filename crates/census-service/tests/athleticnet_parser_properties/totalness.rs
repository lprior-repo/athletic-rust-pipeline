//! Total-ness: what the mark column does with text no page would publish.
//!
//! The column is scraped, so the input space is whatever bytes are in the cell — including figures
//! that parse as floating point but are not marks. A mark of `inf` points or metres compares as
//! better than every real one, so one such cell would silently take every best-mark slot for that
//! event from then on. That is the failure this law exists to prevent, and it is the reason the
//! multi-event points arm carries an explicit finiteness check.

use super::{arbitrary_token, number_of, parse_mark, seam_config, shaped_token};
use census_domain::model::EventKind;
use proptest::prelude::*;

/// One kind of each mark route: a running event, a field event, a multi-event.
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

    /// A mark that is accepted is a figure a competition could have produced.
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
