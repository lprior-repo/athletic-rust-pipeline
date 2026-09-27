use super::seam_config;
use census_crawl::wiaa::{parse_directory_letter, parse_enrollment, parse_school_page};
use proptest::prelude::*;

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
