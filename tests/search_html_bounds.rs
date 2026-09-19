#![forbid(unsafe_code)]

use athletic_rust_pipeline::{
    domain::{evidence::Sport, identity::EvidenceDigest},
    search::{parse_page, SearchQuery},
};
use serde_json::json;

fn digest() -> EvidenceDigest {
    EvidenceDigest::parse("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        .expect("fixed digest")
}

fn query() -> SearchQuery {
    SearchQuery::new("Synthetic Runner", Sport::TrackField, 0).expect("fixed query")
}

#[test]
fn flat_result_and_pager_documents_preserve_candidates_and_offsets() {
    let noise = "<span></span>".repeat(500_001);
    let body = json!({
        "d": {
            "count": 1,
            "pager": format!("{noise}<!--x--><a data-start='12'>next</a>"),
            "results": format!("{noise}x<!--x--><tr><td><a href='/athlete/123/track-and-field'>Synthetic Runner</a></td></tr>")
        }
    });
    let parsed = parse_page(&query(), 0, digest(), body.to_string().as_bytes())
        .expect("bounded streaming handles flat result and pager documents");
    assert_eq!(parsed.results().len(), 1);
    assert_eq!(parsed.results()[0].name(), "Synthetic Runner");
    assert_eq!(parsed.next_offset, Some(12));
}

#[test]
fn rows_and_pager_preserve_foreign_text_and_offsets() {
    let body = json!({
        "d": {
            "count": 1,
            "pager": "<!-- fake <a data-start='999'>bad</a> --><a data-start='12'>next</a><a data-start='4'>current</a>",
            "results": "<!-- fake <tr><a href='/athlete/999/track-and-field'>bad</a></tr> --><tr><td><a href='/athlete/123/track-and-field'>Jos\u{00e9} Runner</a></td></tr>"
        }
    });
    let parsed = parse_page(&query(), 0, digest(), body.to_string().as_bytes())
        .expect("ordinary search markup remains extractable");

    assert_eq!(parsed.results().len(), 1);
    assert_eq!(parsed.results()[0].name(), "Jos\u{00e9} Runner");
    assert_eq!(parsed.next_offset, Some(4));
}
