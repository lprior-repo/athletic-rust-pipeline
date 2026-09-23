//! Property tests for the WIAA directory seam (letter fragments + `GetDirectorySchool` pages).
//!
//! WIAA's directory is the one adapter that reads obfuscated contact data: every address on a school
//! page is a Cloudflare `data-cfemail` payload, decoded the way the provider's own
//! `email-decode.min.js` decodes it (a key byte, then every address byte XORed with it), and every
//! index row is a `GetDirectorySchool` link whose `orgID` is the school's stable key. The pages are
//! assembled from a scatter of tables (`tblSchools`, `tblAdminList`, `tblCoachList`) plus a
//! `JumboMain` label, so the seam's contract is as much "publish what the page prints" as it is
//! "never mint a school".
//!
//! Fixtures under `tests/fixtures/wiaa/` are the committed captures — `directory_letter_a.html` (the
//! `LetterBtn=A` fragment with its `#tblSchools` rows) and `school_org1_abbotsford.html` (one
//! `GetDirectorySchool` page) — and the generators below rebuild those two shapes token for token:
//! the same `gridTextDataTables` labels, the same `Enrollment (…)</span><span>School:</span><b>`
//! block, the same table ids. A property is therefore exercised on markup the seam really receives.
//!
//! Four things are held to a law:
//!
//! * **Payload round trip** — the provider's own encoding of an address decodes back to it, and no
//!   payload decodes to something that is not a published address ([`payloads`]).
//! * **Printed data is published data** — an index row's id, name, level and city are the ones it
//!   prints, and a school page's name and enrollment are the ones it prints ([`printed`]).
//! * **Labels are words, not case** — a published sport, coaching or administration label is read
//!   from its words, and an honorific is not part of a coach's identity ([`labels`]).
//! * **Total** — arbitrary markup reads to entries or to none, never to a panic, and never to a row
//!   without an id or a staff row without a name ([`totalness`]).
//!
//! Deterministic by construction: [`seam_config`] pins 64 cases on ChaCha with the fixed seed below,
//! so a failing case is reproducible from the seed alone.

use census_crawl::wiaa::{parse_directory_letter, parse_school_page};
use proptest::prelude::*;
use proptest::test_runner::{RngAlgorithm, RngSeed};

#[path = "wiaa_directory_properties/labels.rs"]
mod labels;
#[path = "wiaa_directory_properties/payloads.rs"]
mod payloads;
#[path = "wiaa_directory_properties/printed.rs"]
mod printed;
#[path = "wiaa_directory_properties/totalness.rs"]
mod totalness;

/// The two committed captures that pin the index and school-page shapes.
const DIRECTORY_A: &str =
    include_str!("../../census-crawl/tests/fixtures/wiaa/directory_letter_a.html");
const ABBOTSFORD: &str =
    include_str!("../../census-crawl/tests/fixtures/wiaa/school_org1_abbotsford.html");

/// 64 cases per property, ChaCha, fixed seed: reproducible and cheap enough for the normal suite.
fn seam_config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x5749_4141_5052_4F50),
        ..ProptestConfig::default()
    }
}

/// The committed captures read the same way twice, and the seed pages read to schools at all: a
/// fragment that parsed to nothing would make every property above vacuous.
#[test]
fn the_committed_captures_read_the_same_way_twice() {
    assert_eq!(
        parse_directory_letter(DIRECTORY_A),
        parse_directory_letter(DIRECTORY_A)
    );
    assert!(!parse_directory_letter(DIRECTORY_A).is_empty());
    assert_eq!(parse_school_page(ABBOTSFORD), parse_school_page(ABBOTSFORD));
    assert_eq!(parse_school_page(ABBOTSFORD).name, "Abbotsford");
}
