use super::seam_config;
use census_crawl::ohsaa::{
    parse_ad_page, parse_coach_cell, parse_search, parse_sport_label, parse_sports_table,
};
use proptest::prelude::*;

const CANONICAL: [&str; 7] = [
    "assistant athletic director",
    "assistant athletic secretary",
    "athletic secretary",
    "athletic trainer",
    "principal",
    "superintendent",
    "business manager",
];

const TOKENS: [&str; 10] = [
    "<table id=\"tblSearchResults\"><tbody>",
    "<tr><td>DUBLIN COFFMAN</td><td>Dublin</td><td>",
    "<a class=\"btn btn-warning\" href=\"/Outside/Schedule?ohsaaId=474\">View</a>",
    "</td></tr></tbody></table>",
    "<tr id=\"informationSportHeaderRow\"><th>Sport</th></tr>",
    "<tr><td><span class=\"fieldValue\">Track &amp; Field</span></td>",
    "<td><a href=\"mailto:coach@example.org\" class=\"fieldValue\">Joe DePalma (Div-I)</a></td></tr>",
    "<tr><td><span class=\"athleticDepartmentSubheader\">Athletic Director:</span></td>",
    "<tr><td><span class=\"fieldValue\">Jennifer Music</span></td></tr>",
    "<tr><td colspan=\"2\"><br/></td></tr>",
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
    fn arbitrary_markup_reads_to_rows_or_to_none(body in arbitrary_markup()) {
        let results = parse_search(&body);
        let sections = parse_sports_table(&body);
        let page = parse_ad_page(&body);
        prop_assert!(
            results.iter().all(|row| !row.ohsaa_id.is_empty()
                && row.ohsaa_id.chars().all(|ch| ch.is_ascii_digit())),
            "every published result row carries the id its link printed: {:?}",
            results
        );
        prop_assert!(
            sections.iter().all(|(label, _, _)| parse_sport_label(label).is_some()),
            "a sport the pipeline does not track is not published: {:?}",
            sections
        );
        prop_assert!(
            sections.iter().all(|(_, boys, girls)| {
                [boys, girls].into_iter().flatten().all(|entry| !entry.name.is_empty())
            }),
            "a coach without a name is not published: {:?}",
            sections
        );
        prop_assert!(
            page.office_roles.iter().all(|(role, name)| !name.is_empty()
                && (CANONICAL.contains(&role.as_str()) || role == "athletic director")),
            "an office role is published under one of the association's labels, with a name: {:?}",
            page.office_roles
        );
        prop_assert!(
            page.director.as_ref().is_none_or(|(name, _)| !name.is_empty()),
            "a director without a name is not published: {:?}",
            page.director
        );
        prop_assert_eq!(parse_search(&body), results, "the same bytes read the same way");
        prop_assert_eq!(
            format!("{:?}", parse_sports_table(&body)),
            format!("{:?}", sections),
            "and so do the sports tables"
        );
        prop_assert_eq!(
            format!("{:?}", parse_ad_page(&body)),
            format!("{:?}", page),
            "and so do the departments"
        );
        prop_assert_eq!(
            parse_coach_cell(&body).map(|entry| entry.name),
            parse_coach_cell(&body).map(|entry| entry.name),
            "and so do the coach cells"
        );
    }
}
