
use super::seam_config;
use census_crawl::mshsl::parse_school_detail;
use proptest::prelude::*;

const NAMES: [&str; 4] = [
    "Aitkin High School",
    "Wayzata High School",
    "Foley High School",
    "Academic Arts High School",
];
const ROLES: [&str; 4] = [
    "Activities Director",
    "Athletic Director",
    "Assistant Athletic Director",
    "Athletic Secretary",
];
const STAFF: [&str; 4] = ["Barry Mink", "Jane Doe", "Robert Smith", "Ana Alvarez"];

#[derive(Debug)]
struct Staff {
    role: String,
    name: String,
}

#[derive(Debug)]
struct Detail {
    name: String,
    group: u32,
    enrollment: u32,
    staff: Vec<Staff>,
}

fn pages() -> impl Strategy<Value = Detail> {
    (
        prop::sample::select(Vec::from(NAMES)),
        1u32..100_000,
        1u32..2_000,
        prop::collection::vec(
            (
                prop::sample::select(Vec::from(ROLES)),
                prop::sample::select(Vec::from(STAFF)),
            ),
            1..5,
        ),
    )
        .prop_map(|(name, group, enrollment, staff)| Detail {
            name: name.to_string(),
            group,
            enrollment,
            staff: staff
                .into_iter()
                .map(|(role, name)| Staff {
                    role: role.to_string(),
                    name: name.to_string(),
                })
                .collect(),
        })
}

fn render_detail(page: &Detail) -> String {
    let mut body = format!(
        "<h1 class=\"heading heading--page-title\">\n      <div>{}</div></h1>\n\
         <a href=\"/group/{}/events\">Events</a>\n\
         MSHSL Classification Enrollment: {}<br />\n\
         <div class=\"grid--administration\">\n",
        page.name, page.group, page.enrollment
    );
    for staff in &page.staff {
        body.push_str(&format!(
            "<div class=\"grid__item\"><strong>{}</strong><div>{}</div></div>\n",
            staff.role, staff.name
        ));
    }
    body.push_str("</div>\n<h2>Conference</h2>\n");
    body
}

proptest! {
    #![proptest_config(seam_config())]

    #[test]
    fn a_school_page_publishes_the_facts_it_prints(page in pages()) {
        let parsed = parse_school_detail(&render_detail(&page));
        let group = page.group.to_string();
        prop_assert_eq!(parsed.name.as_deref(), Some(page.name.as_str()), "its own title");
        prop_assert_eq!(parsed.school_id.as_deref(), Some(group.as_str()), "its own group key");
        prop_assert_eq!(parsed.enrollment, Some(page.enrollment), "its own enrollment");
    }

    #[test]
    fn every_printed_administration_row_is_published(page in pages()) {
        let parsed = parse_school_detail(&render_detail(&page));
        prop_assert_eq!(
            parsed.admin.len(),
            page.staff.len(),
            "every printed administration row is published"
        );
        for (entry, staff) in parsed.admin.iter().zip(page.staff.iter()) {
            prop_assert_eq!(entry.role.as_str(), staff.role.as_str(), "the role it printed");
            prop_assert_eq!(entry.name.as_str(), staff.name.as_str(), "the name it printed");
            prop_assert!(entry.email().is_none(), "a row without a hidden address publishes none");
        }
    }
}
