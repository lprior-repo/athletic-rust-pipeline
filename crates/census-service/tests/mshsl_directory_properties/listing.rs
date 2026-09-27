
use super::seam_config;
use census_crawl::mshsl::{parse_next_listing_page, parse_school_list, school_page_url};
use proptest::prelude::*;

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

#[derive(Debug)]
struct Listed {
    slug: String,
    name: String,
    city: String,
}

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
