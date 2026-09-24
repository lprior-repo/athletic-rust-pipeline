//! Total-ness and determinism: the two laws every consumer of these parsers is allowed to assume.
//!
//! A parser that panics on a malformed page takes the whole run down with it — a census walk is
//! hundreds of pages nobody has read — and a parser that answers differently on the same bytes
//! makes a journaled rerun meaningless. Both are asserted over arbitrary text and over text woven
//! out of the tokens these parsers key on.

use super::{
    arbitrary_body, parse_meet_index, parse_meet_result_files, parse_raw, seam_config, shaped_body,
    ResultSetRef, OH_FILE_LIST_URL, OH_RAW_URL,
};
use proptest::prelude::*;

/// A parser's answer with the error rendered, so two runs are compared as values.
fn answer<T: std::fmt::Debug>(result: Result<T, impl std::fmt::Display>) -> Result<T, String> {
    result.map_err(|error| error.to_string())
}

proptest! {
    #![proptest_config(seam_config())]

    #[test]
    fn parse_raw_answers_or_refuses_arbitrary_bytes(body in arbitrary_body()) {
        let _ = answer(parse_raw(&body, OH_RAW_URL));
    }

    #[test]
    fn parse_raw_answers_or_refuses_shaped_bytes(body in shaped_body()) {
        let _ = answer(parse_raw(&body, OH_RAW_URL));
    }

    #[test]
    fn the_index_answers_or_refuses_arbitrary_bytes(body in arbitrary_body()) {
        let _ = answer(parse_meet_index(&body));
        let _ = answer(parse_meet_result_files(OH_FILE_LIST_URL, &body));
    }

    #[test]
    fn the_index_answers_or_refuses_shaped_bytes(body in shaped_body()) {
        let _ = answer(parse_meet_index(&body));
        let _ = answer(parse_meet_result_files(OH_FILE_LIST_URL, &body));
    }

    #[test]
    fn the_same_bytes_give_the_same_answer(body in shaped_body()) {
        prop_assert_eq!(
            answer(parse_raw(&body, OH_RAW_URL)).map(|page| format!("{page:?}")),
            answer(parse_raw(&body, OH_RAW_URL)).map(|page| format!("{page:?}")),
        );
        prop_assert_eq!(
            answer(parse_meet_index(&body)).map(|meets| format!("{meets:?}")),
            answer(parse_meet_index(&body)).map(|meets| format!("{meets:?}")),
        );
    }

    /// A URL the reader accepts always names a meet and a result set of a validated jurisdiction:
    /// the arm can request it, or it was never accepted.
    #[test]
    fn an_accepted_result_url_is_addressable(meet in 1u32..100_000, rsid in 1u32..10_000_000) {
        let url = format!(
            "https://oh.milesplit.com/meets/{meet}-a-meet-2026/results/{rsid}/raw"
        );
        let parsed = ResultSetRef::parse(&url).expect("a well-formed results URL is accepted");
        prop_assert_eq!(parsed.meet_id, meet.to_string());
        prop_assert_eq!(parsed.rsid, rsid.to_string());
    }

    /// The `/formatted` route serves an empty JS shell, so a URL for it is never accepted.
    #[test]
    fn a_formatted_result_url_is_refused(meet in 1u32..100_000, rsid in 1u32..10_000_000) {
        let url = format!(
            "https://oh.milesplit.com/meets/{meet}-a-meet-2026/results/{rsid}/formatted"
        );
        prop_assert!(ResultSetRef::parse(&url).is_none());
    }
}
