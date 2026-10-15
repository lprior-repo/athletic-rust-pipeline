use super::seam_config;
use anyhow::{bail, ensure, Context, Result};
use census_crawl::mshsl::{parse_next_listing_page, parse_school_list, school_page_url};
use census_crawl::{CrawlError, SCHOOL_BINDINGS};
use proptest::prelude::*;
use std::fmt::Write;

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
        let entries = parse_school_list(&render_listing(&rows, &[0]))
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
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

fn row(slug: &str, name: &str, city: Option<&str>) -> String {
    let mut body = format!(
        "<div class=\"views-row\"><a href=\"/schools/{slug}\" class=\"school-teaser__title\">{name}</a>"
    );
    if let Some(city) = city {
        body.push_str(&format!("<span class=\"locality\">{city}</span>"));
    }
    body.push_str("</div>");
    body
}

fn schema(body: &str, expected: &str) -> Result<()> {
    match parse_school_list(body) {
        Err(CrawlError::Schema { url, detail }) => {
            ensure!(url == census_crawl::mshsl::SCHOOL_LIST_URL, "{url}");
            ensure!(detail == expected, "{detail}");
            Ok(())
        }
        other => bail!("expected Schema({expected}), got {other:?}"),
    }
}

#[test]
fn empty_and_unrelated_markup_are_typed_listing_refusals() -> Result<()> {
    for body in ["", "<html><body>no rows</body></html>"] {
        schema(body, "listing row must contain one school title")?;
    }
    Ok(())
}

#[test]
fn duplicate_rows_do_not_skip_the_following_school() -> Result<()> {
    let first = row("first", "First School", Some("Alpha"));
    let last = row("last", "Last School", Some("Omega"));
    let rows = parse_school_list(&format!("{first}{first}{last}"))?;
    let cells: Vec<_> = rows
        .iter()
        .map(|row| (row.slug.as_str(), row.name.as_str(), row.city.as_deref()))
        .collect();
    ensure!(
        cells
            == vec![
                ("first", "First School", Some("Alpha")),
                ("last", "Last School", Some("Omega")),
            ],
        "{cells:?}"
    );
    Ok(())
}

#[test]
fn a_missing_city_never_borrows_the_next_schools_locality() -> Result<()> {
    let html = format!(
        "{}{}{}",
        row("first", "First", Some("Alpha")),
        row("middle", "Middle", None),
        row("last", "Last", Some("Omega")),
    );
    let rows = parse_school_list(&html)?;
    let cells: Vec<_> = rows
        .iter()
        .map(|row| (row.slug.as_str(), row.city.as_deref()))
        .collect();
    ensure!(
        cells
            == vec![
                ("first", Some("Alpha")),
                ("middle", None),
                ("last", Some("Omega"))
            ],
        "{cells:?}"
    );
    Ok(())
}

#[test]
fn conflicting_slug_names_or_cities_refuse_the_listing() -> Result<()> {
    let first = row("owner", "First School", Some("Alpha"));
    for conflicting in [
        row("owner", "Other School", Some("Alpha")),
        row("owner", "First School", Some("Other City")),
        row("owner", "First School", None),
    ] {
        schema(
            &format!("{first}{conflicting}"),
            "conflicting identity for listing slug",
        )?;
        schema(
            &format!("{conflicting}{first}"),
            "conflicting identity for listing slug",
        )?;
    }
    Ok(())
}

#[test]
fn malformed_school_rows_cannot_disappear_between_valid_rows() -> Result<()> {
    let first = row("first", "First", Some("Alpha"));
    let last = row("last", "Last", Some("Omega"));
    for (malformed, expected) in [
        ("<div class=\"views-row\"><a href=\"/schools\" class=\"school-teaser__title\">No Slug</a></div>", "listing row has no school link"),
        ("<div class=\"views-row\">No title</div>", "listing row must contain one school title"),
    ] {
        schema(&format!("{first}{malformed}{last}"), expected)?;
    }
    schema(
        &row("nested/owner", "School", None),
        "listing row has an invalid school identity",
    )?;
    schema(
        &row("owner", " &nbsp; ", None),
        "listing row has an invalid school identity",
    )?;
    Ok(())
}

#[test]
fn standalone_rows_are_supported_but_ambiguous_segments_are_refused() -> Result<()> {
    let link = "<a href=\"/schools/owner\" class=\"school-teaser__title\">School</a>";
    let rows = parse_school_list(&format!("{link}<span class=\"locality\">City</span>"))?;
    ensure!(rows.len() == 1);
    let school = rows.first().context("standalone school")?;
    ensure!(
        school.slug == "owner" && school.name == "School" && school.city.as_deref() == Some("City")
    );
    schema(
        &format!("{link}{link}"),
        "listing row must contain one school title",
    )?;
    schema(
        &format!("<div class=\"views-row\">{link}{link}</div>"),
        "listing row must contain one school title",
    )?;
    schema(
        &format!("{link}{}", row("next", "Next", None)),
        "school title precedes the first listing row",
    )?;
    schema(
        &format!(
            "{}{link}<span class=\"locality\">Broken",
            "<div class=\"views-row\">"
        ),
        "listing row has malformed locality",
    )?;
    schema(
        &format!(
            "{}{link}<span class=\"locality\">A</span><span class=\"locality\">B</span>",
            "<div class=\"views-row\">"
        ),
        "listing row has multiple localities",
    )?;
    Ok(())
}

#[test]
fn listing_entities_and_whitespace_preserve_the_published_cells() -> Result<()> {
    let rows = parse_school_list(&row(
        "owner",
        " O&#039;Brien &amp;nbsp; &lt;North&gt;\u{a0} A &amp; B ",
        Some(" St.&#160;Paul "),
    ))?;
    let school = rows.first().context("normalized school")?;
    ensure!(
        school.name == "O'Brien &nbsp; <North> A & B",
        "{}",
        school.name
    );
    ensure!(school.city.as_deref() == Some("St. Paul"));
    Ok(())
}

#[test]
fn all_65536_school_rows_are_admitted_and_the_next_is_refused() -> Result<()> {
    let mut html = String::new();
    for index in 0..SCHOOL_BINDINGS {
        write!(
            &mut html,
            "{}",
            row(
                &format!("school-{index}"),
                &format!("School {index}"),
                Some("City")
            )
        )?;
    }
    let rows = parse_school_list(&html)?;
    ensure!(rows.len() == SCHOOL_BINDINGS);
    ensure!(rows.first().context("first school")?.slug == "school-0");
    ensure!(rows.last().context("last school")?.slug == "school-65535");
    let excess = SCHOOL_BINDINGS.checked_add(1).context("row count")?;
    html.push_str(&row("overflow", "Overflow", None));
    match parse_school_list(&html) {
        Err(CrawlError::Resource {
            resource,
            requested,
            limit,
        }) => {
            ensure!(
                resource == "MSHSL listing rows" && requested == excess && limit == SCHOOL_BINDINGS
            );
        }
        _ => bail!("expected row admission failure"),
    }
    Ok(())
}

#[test]
fn a_32_mib_listing_is_admitted_before_any_larger_body_is_refused() -> Result<()> {
    const LIMIT: usize = 32 * 1024 * 1024;
    let mut html = String::from(
        "<div class=\"views-row\"><a href=\"/schools/owner\" class=\"school-teaser__title\">",
    );
    let suffix = "</a></div>";
    let name_bytes = LIMIT
        .checked_sub(html.len())
        .and_then(|bytes| bytes.checked_sub(suffix.len()))
        .context("name budget")?;
    let remaining = name_bytes
        .checked_add(suffix.len())
        .context("remaining HTML budget")?;
    html.try_reserve_exact(remaining)?;
    html.extend(std::iter::repeat_n('x', name_bytes));
    html.push_str(suffix);
    let rows = parse_school_list(&html)?;
    ensure!(html.len() == LIMIT && rows.len() == 1);
    ensure!(rows.first().context("maximum-sized school")?.name.len() == name_bytes);
    drop(rows);
    html.try_reserve_exact(1)?;
    html.push(' ');
    match parse_school_list(&html) {
        Err(CrawlError::Resource {
            resource,
            requested,
            limit,
        }) => {
            ensure!(
                resource == "MSHSL listing HTML bytes" && requested == html.len() && limit == LIMIT
            );
        }
        _ => bail!("expected byte admission failure"),
    }
    Ok(())
}
