use super::seam_config;
use census_crawl::mshsl::{parse_admin_entries, parse_school_list, SCHOOL_LIST_URL};
use census_crawl::CrawlError;
use proptest::prelude::*;

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

    #[test]
    fn arbitrary_markup_yields_nonempty_school_rows_or_a_typed_schema_refusal(body in arbitrary_markup()) {
        match parse_school_list(&body) {
            Ok(rows) => {
                prop_assert!(!rows.is_empty(), "arbitrary markup must not become a successful empty listing");
                prop_assert!(rows.iter().all(|row| !row.slug.is_empty()
                    && !row.slug.contains('/') && !row.name.is_empty()), "{rows:?}");
                let slugs: std::collections::HashSet<_> = rows.iter().map(|row| row.slug.as_str()).collect();
                prop_assert_eq!(slugs.len(), rows.len(), "each admitted school slug is published once");
            }
            Err(CrawlError::Schema { url, detail }) => {
                prop_assert_eq!(url, SCHOOL_LIST_URL);
                prop_assert!(!detail.is_empty());
            }
            Err(error) => return Err(TestCaseError::fail(format!("unexpected listing failure: {error}"))),
        }
        let admin = parse_admin_entries(&body);
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
    }
}
