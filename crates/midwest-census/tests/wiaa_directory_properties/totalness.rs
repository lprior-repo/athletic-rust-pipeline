//! Halves of pages, rubbish bodies, and what the seam is allowed to do with them.
//!
//! The directory walker fetches fragments (`LetterBtn=-1` is a legitimate empty answer) and school
//! pages that are sometimes a JSON error, sometimes a login redirect, sometimes markup cut off
//! mid-table. None of those may panic, and none may mint a school: an index row is only published
//! when the row carries a numeric `orgID`, and a staff row is only published when the page printed
//! both a role and a name. The same bytes must also always read the same way — the walker's
//! bookkeeping keys off these reads.

use super::seam_config;
use midwest_census::sources::wiaa::{parse_directory_letter, parse_enrollment, parse_school_page};
use proptest::prelude::*;

/// Tokens from the committed captures, in the order a scrambled or half-rendered page might carry
/// them.
const TOKENS: [&str; 10] = [
    "<table id=\"tblSchools\"><tbody>",
    "<tr><td>",
    "<a href=\"/Directory/GetDirectorySchool?orgID=1234\" title=\"Abbotsford\">",
    "<label class=\"gridTextDataTables\">High School</label>",
    "</td></tr></tbody></table>",
    "<table id=\"tblAdminList\"><tbody><tr><td>1</td><td>Athletic Director</td>",
    "<table id=\"tblCoachList\"><tbody><tr><td>1</td><td>Boys Track and Field</td>",
    "<label class=\"JumboMain\">Abbotsford</label>",
    "<span>Enrollment (2026-2027)</span><span>School:</span>",
    "data-cfemail=\"d3a1b1b2a1b4b6bdb7b6a193b2b1b1bca7a0b5bca1b7fdb8e2e1fda4bafda6a0\"",
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

    /// Arbitrary markup reads to entries or to none — never to a panic, never to an index row without
    /// a numeric id, never to a staff row without a role or a name, and always the same way twice.
    #[test]
    fn arbitrary_markup_reads_to_entries_or_to_none(body in arbitrary_markup()) {
        let entries = parse_directory_letter(&body);
        let page = parse_school_page(&body);
        prop_assert!(
            entries.iter().all(|entry| !entry.org_id.is_empty()
                && entry.org_id.chars().all(|ch| ch.is_ascii_digit())),
            "every published index row carries the numeric id its link printed: {:?}",
            entries
        );
        prop_assert!(
            page.admins.iter().all(|row| !row.role.is_empty() && !row.name.is_empty()),
            "an administration row without a role or a name is not published: {:?}",
            page
        );
        prop_assert!(
            page.coaches.iter().all(|row| !row.sport.is_empty() && !row.name.is_empty()),
            "a coach row without a sport or a name is not published: {:?}",
            page
        );
        prop_assert_eq!(
            parse_directory_letter(&body),
            entries,
            "the same bytes read the same way"
        );
        prop_assert_eq!(parse_school_page(&body), page, "and so do the pages");
        prop_assert_eq!(
            parse_enrollment(&body),
            parse_enrollment(&body),
            "and so does the enrollment"
        );
    }
}
