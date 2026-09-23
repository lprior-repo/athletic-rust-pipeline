//! Property tests for the MSHSL directory seam (the `/schools` listing + `/schools/<slug>` pages).
//!
//! MSHSL is the league whose directory publishes a whole school universe and hides every address
//! behind Cloudflare: the listing is Drupal rows (`views-row` chunks carrying a
//! `school-teaser__title` link and a `locality` span), the pager walks `?page=<n>` links, and a
//! school page prints its name, its `/group/<id>/` key, its classification enrollment and an
//! `Administration` grid whose addresses arrive as `email-protection#<hex>` hrefs or `data-cfemail`
//! attributes. The seam's contract is therefore: publish the slug the listing printed (it is the key
//! the store resolves a school by), publish each hidden address once, and never guess a fact a page
//! did not print.
//!
//! Fixtures under `tests/fixtures/mshsl/` are the committed captures — `schools_listing.html` (eight
//! `views-row` chunks and the pager) and `school_detail_aitkin-high-school.html` (one school page) —
//! and the generators below rebuild those shapes token for token: the same row marker, the same
//! `school-teaser__title` link class, the same `MSHSL Classification Enrollment:` line, the same
//! `grid--administration`/`grid__item` administration markup.
//!
//! Four things are held to a law:
//!
//! * **Hidden addresses** — the provider's own Cloudflare encoding decodes back to the address, and a
//!   fragment publishes each hidden address once ([`hidden`]).
//! * **Printed data is published data** — a listed school's slug, name and city are the ones it
//!   printed, and the pager follows the page the listing prints ([`listing`]).
//! * **The page's own facts** — a school page's name, group key and enrollment are the ones it
//!   printed, and every printed administration row is published ([`detail`]).
//! * **Total** — arbitrary markup reads to schools or to none, never to a panic ([`totalness`]).
//!
//! Deterministic by construction: [`seam_config`] pins 64 cases on ChaCha with the fixed seed below,
//! so a failing case is reproducible from the seed alone.

use census_crawl::mshsl::{parse_next_listing_page, parse_school_detail, parse_school_list};
use proptest::prelude::*;
use proptest::test_runner::{RngAlgorithm, RngSeed};

#[path = "mshsl_directory_properties/detail.rs"]
mod detail;
#[path = "mshsl_directory_properties/hidden.rs"]
mod hidden;
#[path = "mshsl_directory_properties/listing.rs"]
mod listing;
#[path = "mshsl_directory_properties/totalness.rs"]
mod totalness;

/// The two committed captures that pin the listing and school-page shapes.
const LISTING: &str = include_str!("../../census-crawl/tests/fixtures/mshsl/schools_listing.html");
const AITKIN: &str =
    include_str!("../../census-crawl/tests/fixtures/mshsl/school_detail_aitkin-high-school.html");

/// 64 cases per property, ChaCha, fixed seed: reproducible and cheap enough for the normal suite.
fn seam_config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x4D53_4853_4C50_5250),
        ..ProptestConfig::default()
    }
}

/// The committed captures read the same way twice, the listing page reads to schools at all, and its
/// pager names the next page — a capture that parsed to nothing would make every property vacuous.
#[test]
fn the_committed_captures_read_the_same_way_twice() {
    assert_eq!(parse_school_list(LISTING), parse_school_list(LISTING));
    assert_eq!(parse_school_list(LISTING).len(), 8);
    assert_eq!(parse_next_listing_page(LISTING, 0), Some(1));
    assert_eq!(parse_school_detail(AITKIN), parse_school_detail(AITKIN));
    assert_eq!(
        parse_school_detail(AITKIN).name.as_deref(),
        Some("Aitkin High School")
    );
}
