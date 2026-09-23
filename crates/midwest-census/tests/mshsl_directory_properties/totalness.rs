//! Truncated pages, error bodies and what the seam is allowed to do with them.
//!
//! The listing is fetched page by page and a school page is fetched per school, so most of what
//! arrives is not a complete page: a cut-off `views-row`, a JSON error body, a login redirect or a
//! snapshot with the administration grid missing. None of those may panic, none may mint a school
//! without a slug, and the same bytes must always read the same way — the walker's bookkeeping keys
//! off these reads.

use super::seam_config;
use census_crawl::mshsl::{
    parse_admin_entries, parse_next_listing_page, parse_school_detail, parse_school_list,
};
use proptest::prelude::*;

/// Tokens from the committed captures, in the order a scrambled or half-rendered page might carry
/// them.
const TOKENS: [&str; 10] = [
    "<div class=\"views-row\">",
    "<a href=\"/schools/aitkin-high-school\" class=\"school-teaser__title\">Aitkin High School</a>",
    "<span class=\"locality\">Aitkin</span>",
    "<a rel=\"next\" href=\"?page=2\">Next</a>",
    "<h1 class=\"heading heading--page-title\"><div>Aitkin High School</div></h1>",
    "<a href=\"/group/7/events\">Events</a>",
    "MSHSL Classification Enrollment: 291",
    "<div class=\"grid--administration\">",
    "<div class=\"grid__item\"><strong>Activities Director</strong><div>Barry Mink</div></div>",
    "<a href=\"mailto:?email-protection#d3a1b1b2a1b4b6bdb7b6a193b2b1b1bca7a0b5bca1b7fdb8e2e1fda4bafda6a0\">",
];

/// Arbitrary markup, plus markup assembled from the seam's own tokens: the shapes a truncated,
/// scrambled or non-HTML body actually arrives in.
fn arbitrary_markup() -> impl Strategy<Value = String> {
    prop_oneof![
        prop::collection::vec(any::<char>(), 0..512)
            .prop_map(|chars| chars.into_iter().collect::<String>()),
        prop::collection::vec(prop::sample::select(Vec::from(TOKENS)), 0..48)
            .prop_map(|tokens| tokens.concat()),
    ]
}

proptest! {
    #![proptest_config(seam_config())]

    /// Arbitrary markup reads to schools, entries and pager decisions or to none — never to a panic,
    /// never to a school without a slug, and always the same way twice.
    #[test]
    fn arbitrary_markup_reads_to_schools_or_to_none(body in arbitrary_markup()) {
        let rows = parse_school_list(&body);
        let page = parse_school_detail(&body);
        let admin = parse_admin_entries(&body);
        prop_assert!(
            rows.iter().all(|row| !row.slug.is_empty()
                && !row.slug.contains('/')
                && !row.name.is_empty()),
            "a school is published with the slug its link carried and the name it printed: {:?}",
            rows
        );
        prop_assert!(
            admin.iter().all(|entry| !entry.role.is_empty()),
            "an administration entry without a role is not published: {:?}",
            admin
        );
        prop_assert!(
            admin.iter().all(|entry| entry.emails.iter().all(|email| !email.is_empty())
                && entry.emails.len() == entry.emails.iter().collect::<std::collections::HashSet<_>>().len()),
            "an entry publishes each hidden address once, and never an empty one: {:?}",
            admin
        );
        prop_assert_eq!(parse_school_list(&body), rows, "the same bytes read the same way");
        prop_assert_eq!(parse_school_detail(&body), page, "and so do the pages");
        prop_assert_eq!(parse_admin_entries(&body), admin, "and so do the entries");
        prop_assert_eq!(
            parse_next_listing_page(&body, 0),
            parse_next_listing_page(&body, 0),
            "and so does the pager"
        );
    }
}
