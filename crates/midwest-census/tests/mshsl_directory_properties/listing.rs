//! The `/schools` listing and its pager.
//!
//! The listing is the only place the crawl learns the school universe: one `views-row` chunk per
//! school carrying a `school-teaser__title` link (whose `/schools/<slug>` path is the key the store
//! resolves a school by) and a `locality` span naming its city. The pager decides whether the walk
//! continues. The laws: a listed school is published with its own slug, name and city, a school the
//! page lists twice is published once, and the pager follows page `current + 1` exactly when the page
//! links to it.

use super::seam_config;
use midwest_census::sources::mshsl::{parse_next_listing_page, parse_school_list, school_page_url};
use proptest::prelude::*;

/// Slugs, names and cities taken from the committed listing capture.
const SLUGS: [&str; 4] = [
    "aitkin-high-school",
    "wayzata-high-school",
    "foley-high-school",
    "academic-arts-high-school",
];
const NAMES: [&str; 4] = [
    "Aitkin High School",
    "Wayzata High School",
    "Foley High School",
    "Academic Arts High School",
];
const CITIES: [&str; 4] = ["Aitkin", "Plymouth", "Foley", "West St. Paul"];

/// One listed school: the slug its link carries, its name and its city.
#[derive(Debug)]
struct Listed {
    slug: String,
    name: String,
    city: String,
}

/// A listing page's rows, with each slug listed at most once.
fn listed_schools() -> impl Strategy<Value = Vec<Listed>> {
    prop::collection::vec(
        (
            prop::sample::select(Vec::from(SLUGS)),
            prop::sample::select(Vec::from(NAMES)),
            prop::sample::select(Vec::from(CITIES)),
        ),
        1..5,
    )
    .prop_map(|rows| {
        let mut out: Vec<Listed> = Vec::new();
        for (slug, name, city) in rows {
            if out.iter().any(|listed| listed.slug == slug) {
                continue;
            }
            out.push(Listed {
                slug: slug.to_string(),
                name: name.to_string(),
                city: city.to_string(),
            });
        }
        out
    })
}

/// The listing markup `schools_listing.html` prints: one `views-row` per school and the Drupal pager.
fn render_listing(rows: &[Listed], pager: &[usize]) -> String {
    let mut body = String::from("<div class=\"view-content\">\n");
    for row in rows {
        body.push_str(&format!(
            "<div class=\"views-row\"><div class=\"school-teaser\">\
             <a href=\"/schools/{}\" class=\"school-teaser__title\">{}</a>\
             <span class=\"locality\">{}</span></div></div>\n",
            row.slug, row.name, row.city
        ));
    }
    body.push_str("</div>\n<nav class=\"pager\"><ul class=\"pager__items js-pager__items\">\n");
    for page in pager {
        body.push_str(&format!(
            "<li class=\"pager__item\"><a href=\"?page={page}\">{page}</a></li>\n"
        ));
    }
    body.push_str("</ul></nav>\n");
    body
}

proptest! {
    #![proptest_config(seam_config())]

    /// Every listed school is published once, with the slug its own link carried and the city it
    /// printed beside it — the slug is the key a later crawl resolves the school by.
    #[test]
    fn a_listed_school_is_published_with_its_own_slug(rows in listed_schools()) {
        let entries = parse_school_list(&render_listing(&rows, &[0]));
        prop_assert_eq!(entries.len(), rows.len(), "every listed school is published once");
        for (entry, row) in entries.iter().zip(rows.iter()) {
            prop_assert_eq!(entry.slug.as_str(), row.slug.as_str(), "the slug its link carried");
            prop_assert_eq!(entry.name.as_str(), row.name.as_str(), "the name it printed");
            prop_assert_eq!(entry.city.as_deref(), Some(row.city.as_str()), "the city it printed");
            prop_assert!(
                school_page_url(&entry.slug).ends_with(&format!("/{}", row.slug)),
                "the page URL is built from the slug the listing printed"
            );
        }
    }

    /// The pager follows the page after the one being read, and only when the listing links to it: a
    /// walk cannot skip a page of schools and cannot start reading a page that does not exist.
    #[test]
    fn the_pager_follows_the_page_the_listing_prints(current in 0usize..8, pages in 1usize..10) {
        let links: Vec<usize> = (0..pages).collect();
        let html = render_listing(&[], &links);
        let expected = (pages > current + 1).then_some(current + 1);
        prop_assert_eq!(
            parse_next_listing_page(&html, current),
            expected,
            "a listing printing pages 0..{} read as page {}",
            pages,
            current
        );
    }
}
