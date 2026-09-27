use super::{
    arbitrary_body, parse_meet_index, parse_meet_result_files, parse_raw, seam_config, shaped_body,
    ResultSetRef, OH_FILE_LIST_URL, OH_RAW_URL,
};
use proptest::prelude::*;

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

    #[test]
    fn an_accepted_result_url_is_addressable(meet in 1u32..100_000, rsid in 1u32..10_000_000) {
        let url = format!(
            "https://oh.milesplit.com/meets/{meet}-a-meet-2026/results/{rsid}/raw"
        );
        let parsed = ResultSetRef::parse(&url).expect("a well-formed results URL is accepted");
        prop_assert_eq!(parsed.meet_id, meet.to_string());
        prop_assert_eq!(parsed.rsid, rsid.to_string());
    }

    #[test]
    fn a_formatted_result_url_is_refused(meet in 1u32..100_000, rsid in 1u32..10_000_000) {
        let url = format!(
            "https://oh.milesplit.com/meets/{meet}-a-meet-2026/results/{rsid}/formatted"
        );
        prop_assert!(ResultSetRef::parse(&url).is_none());
    }
}
