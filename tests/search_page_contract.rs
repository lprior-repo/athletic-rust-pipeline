#![forbid(unsafe_code)]

// The live `POST https://www.athletic.net/Search.aspx/runSearch` page contract, pinned to bytes the
// endpoint actually returned.
//
// Each fixture is an excerpt of a retained live capture (`research/captures/g1/`): every row in it
// is byte-identical to its source row, only the row selection and the pager's link list are cut
// down, except `run-search-sibling-sport-only.body`, which is a whole 725-byte response. The
// counted inventory of both live corpora, and the authorized capture that came back as a Cloudflare
// managed challenge, are recorded in `research/G1-LIVE-SEARCH-CONTRACT.md`.

use athletic_rust_pipeline::{
    domain::{evidence::Sport, identity::EvidenceDigest},
    search::{parse_page, PageRejection, SearchPage, SearchProgress, SearchQuery},
};
use serde_json::json;
use sha2::{Digest, Sha256};

/// Rows 0, 2 and 5 of `pilot-01db901cda4e.body` — query `Robert Wallace`, cross country, stage 0,
/// fetched 2026-09-16T22:21:47Z, HTTP 200, 16,239 bytes, sha256
/// `01db901cda4eb5dd586d0a9bc1806a2b2457f77d2491713e1c7b83dd6d2f1a96`. The pipeline recorded
/// `search page contains malformed or unrelated result rows` for that page: row 0 links the athlete
/// for both sports, row 2 is the site's `/athlete//<sport>` placeholder.
const MIXED: &str = include_str!("fixtures/search/run-search-sibling-and-placeholder.body");

/// Rows 0, 1 and 8 of `pilot-007b7e533499.body` — query `Emma Easter`, cross country, stage 0,
/// fetched 2026-09-16T22:26:03Z, HTTP 200, 14,643 bytes, sha256
/// `007b7e533499f85ce74016e89d84f76050d5b25073f8f5d6d9dbb5bbfd6cc151`. The pipeline recorded
/// `search page contains malformed or unrelated result rows` for that page: row 8 is
/// track-and-field only, which is what the endpoint answers a cross-country query with.
const OTHER_SPORT: &str = include_str!("fixtures/search/run-search-other-sport-row.body");

/// The whole `lane-out-live-outdoor-boys-0ec156e0ae3c.body` — query `Outside Example`, track and
/// field, stage 0, fetched 2026-09-21T01:36:43Z, HTTP 200, 725 bytes, sha256
/// `0ec156e0ae3c7d9d84c17111e13ea2a9a67ecbab8228d32ec557192491912ede`. The pipeline recorded
/// `search page ended before the advertised result count` for that page: its one row is
/// cross-country, so this query had no candidate at all.
const SIBLING_ONLY: &str = include_str!("fixtures/search/run-search-sibling-sport-only.body");

/// The Cloudflare managed challenge the authorized 2026-09-22 capture received (HTTP 403,
/// `cf-mitigated: challenge`), trimmed to its structural lines from
/// `search-post-20260922T044528Z.body`, 6,216 bytes, sha256
/// `63d308ffe76274183a4458248ed1d4f2970789b13a3451d24c549316a286d85f`.
const CHALLENGE: &str = include_str!("fixtures/search/blocked-challenge.html");

fn digest() -> EvidenceDigest {
    EvidenceDigest::parse("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        .expect("fixed digest")
}

fn query(text: &str, sport: Sport) -> SearchQuery {
    SearchQuery::new(text, sport, 0).expect("fixed query")
}

fn parse_fixture(body: &str, query: &SearchQuery) -> SearchPage {
    parse_page(query, 0, digest(), body.as_bytes())
        .expect("the fixture parses as the retained live page contract")
}

fn named_rejection(error: &anyhow::Error) -> &PageRejection {
    let named = error.downcast_ref::<PageRejection>();
    named.expect("a named page rejection")
}

#[test]
fn the_real_mixed_page_parses_into_the_queried_sports_candidates() {
    let search = query("Robert Wallace", Sport::CrossCountry);
    let parsed = parse_fixture(MIXED, &search);

    assert_eq!(parsed.advertised_count(), 256);
    assert_eq!(parsed.next_offset, Some(10));
    assert!(parsed.issues.is_empty(), "issues {:?}", parsed.issues);
    assert_eq!(parsed.results().len(), 2);

    // Row 0 links one athlete for both sports; the cross-country link is the queried one.
    let both = &parsed.results()[0];
    assert_eq!(both.id().get(), 26606023);
    assert_eq!(both.name(), "Robert Wallace");
    assert_eq!(both.sport, Sport::CrossCountry);
    assert_eq!(
        both.url().as_str(),
        "https://athletic.net/athlete/26606023/cross-country"
    );
    assert_eq!(both.evidence.document, digest());
    assert_eq!(both.evidence.locator, "/d/results/page/0/row/0");

    // Row 2 is a second athlete of that name: one profile URL, the queried sport.
    let single = &parsed.results()[1];
    assert_eq!(single.id().get(), 10502682);
    assert_eq!(single.name(), "Robert Wallace");
    assert_eq!(single.sport, Sport::CrossCountry);
    assert_eq!(
        single.url().as_str(),
        "https://athletic.net/athlete/10502682/cross-country"
    );
    assert_eq!(single.evidence.locator, "/d/results/page/0/row/2");

    // Row 1 is the endpoint's `/athlete//<sport>` placeholder: no identity, so it is counted, not
    // read, as a row this query cannot use.
    assert_eq!(parsed.skipped, 1);

    // The page sits mid-pagination, so the walk continues instead of failing the page.
    let mut progress = SearchProgress::new(search);
    progress.consume(parsed).expect("mixed rows reconcile");
    assert!(!progress.complete(), "the walk continues");
    assert_eq!(progress.next_offset(), Some(10));
    assert_eq!(progress.pages(), 1);
}

#[test]
fn the_real_other_sport_row_is_counted_not_rejected() {
    let search = query("Emma Easter", Sport::CrossCountry);
    let parsed = parse_fixture(OTHER_SPORT, &search);

    assert_eq!(parsed.advertised_count(), 271);
    assert_eq!(parsed.next_offset, Some(10));
    assert!(parsed.issues.is_empty(), "issues {:?}", parsed.issues);

    let both = &parsed.results()[0];
    assert_eq!(both.id().get(), 8959975);
    assert_eq!(both.name(), "Emma Easter");
    assert_eq!(both.sport, Sport::CrossCountry);
    assert_eq!(
        both.url().as_str(),
        "https://athletic.net/athlete/8959975/cross-country"
    );
    assert_eq!(both.evidence.locator, "/d/results/page/0/row/0");

    let surname = &parsed.results()[1];
    assert_eq!(surname.id().get(), 29563262);
    assert_eq!(surname.name(), "Emma Easterday");
    assert_eq!(surname.sport, Sport::CrossCountry);
    assert_eq!(
        surname.url().as_str(),
        "https://athletic.net/athlete/29563262/cross-country"
    );
    assert_eq!(surname.evidence.locator, "/d/results/page/0/row/1");

    // Row 8 is track-and-field only: the endpoint answered the sibling sport, and that row is a
    // skip that never becomes a candidate of the queried sport.
    assert_eq!(parsed.skipped, 1);
    let ids: Vec<u64> = parsed.results().iter().map(|c| c.id().get()).collect();
    assert_eq!(ids, [8959975, 29563262]);

    let mut progress = SearchProgress::new(search);
    progress.consume(parsed).expect("sibling row reconciles");
    assert!(!progress.complete(), "the walk continues");
    assert_eq!(progress.next_offset(), Some(10));
}

#[test]
fn the_real_sibling_sport_only_page_completes_the_walk_without_a_match() {
    // This fixture is the captured response itself, byte for byte.
    assert_eq!(
        format!("{:x}", Sha256::digest(SIBLING_ONLY.as_bytes())),
        "0ec156e0ae3c7d9d84c17111e13ea2a9a67ecbab8228d32ec557192491912ede"
    );

    let search = query("Outside Example", Sport::TrackField);
    let parsed = parse_fixture(SIBLING_ONLY, &search);

    assert_eq!(parsed.advertised_count(), 1);
    assert_eq!(parsed.next_offset, None);
    assert!(parsed.results().is_empty(), "{:?}", parsed.results());
    assert!(parsed.issues.is_empty(), "issues {:?}", parsed.issues);
    assert_eq!(parsed.skipped, 1);

    let mut progress = SearchProgress::new(search);
    progress.consume(parsed).expect("sibling-only completes");
    assert!(progress.complete(), "the walk ends with the pager");
    assert!(progress.candidates().is_empty(), "with no candidate");
}

#[test]
fn a_blocked_page_fails_by_name_instead_of_parsing_empty() {
    let search = query("Ryan Masocha", Sport::TrackField);
    let error = parse_page(&search, 0, digest(), CHALLENGE.as_bytes())
        .expect_err("a managed challenge cannot parse as a search page");

    let rejection = named_rejection(&error);
    assert_eq!(rejection.class(), "not_json_envelope");
    let message = error.to_string();
    assert!(message.contains("not the endpoint JSON envelope"));
}

#[test]
fn a_page_without_its_row_anchors_fails_by_name() {
    let body = json!({
        "d": {
            "count": 12,
            "pager": "<div class='text-center'><ul class='pagination'><li class='active'><a href='#'>1</a></li><li data-start='10'><a href='#'>2</a></li></ul></div>",
            "results": "<div class='text-center'>No results found.</div>"
        }
    });
    let search = query("Synthetic Runner", Sport::TrackField);
    let error = parse_page(&search, 0, digest(), body.to_string().as_bytes())
        .expect_err("12 advertised rows with no result row cannot parse");

    let rejection = named_rejection(&error);
    assert_eq!(rejection.class(), "rows_absent");
    assert!(error.to_string().contains("advertises 12 result"));
}

#[test]
fn a_search_the_endpoint_reports_empty_is_a_parsed_empty_page() {
    let body = json!({
        "d": {
            "count": 0,
            "pager": "<div class='text-center'><ul class='pagination'><li class='active'><a href='#'>1</a></li></ul></div>",
            "results": "<div class='text-center'>No results found.</div>"
        }
    });
    let search = query("Synthetic Runner", Sport::TrackField);
    let parsed = parse_page(&search, 0, digest(), body.to_string().as_bytes())
        .expect("a zero-result search is a page the walk completes");

    assert_eq!(parsed.advertised_count(), 0);
    assert!(parsed.results().is_empty(), "{:?}", parsed.results());
    assert!(parsed.issues.is_empty(), "issues {:?}", parsed.issues);
    assert_eq!(parsed.skipped, 0);
    assert_eq!(parsed.next_offset, None);

    let mut progress = SearchProgress::new(search);
    progress.consume(parsed).expect("empty search reconciles");
    assert!(progress.complete(), "an empty search completes");
    assert!(progress.candidates().is_empty(), "with no candidate");
}
