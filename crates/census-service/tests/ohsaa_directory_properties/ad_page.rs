
use super::seam_config;
use census_crawl::ohsaa::parse_ad_page;
use proptest::prelude::*;

const ROLES: [&str; 4] = [
    "Athletic Director:",
    "Assistant Athletic Director:",
    "Athletic Secretary:",
    "Principal:",
];
const CANONICAL: [&str; 4] = [
    "athletic director",
    "assistant athletic director",
    "athletic secretary",
    "principal",
];
const VALUE_LABELS: [&str; 2] = ["Email:", "Phone: (614) 718-8142"];
const PEOPLE: [&str; 4] = ["Jennifer Music", "Adam Banks", "Barry Mink", "Joe DePalma"];

#[derive(Debug, Clone)]
struct Row {
    role: usize,
    person: usize,
    value_label: bool,
}

fn rows() -> impl Strategy<Value = Vec<Row>> {
    prop::collection::vec((0usize..4, 0usize..4, any::<bool>()), 1..5).prop_map(|rows| {
        rows.into_iter()
            .map(|(role, person, value_label)| Row {
                role,
                person,
                value_label,
            })
            .collect()
    })
}

fn render_ad(rows: &[Row]) -> String {
    let mut body = String::from("<table><tbody>\n");
    for (index, row) in rows.iter().enumerate() {
        let role = ROLES.get(row.role).copied().unwrap_or_default();
        body.push_str(&format!(
            "<tr><td><span class=\"athleticDepartmentSubheader\">{role}</span></td>"
        ));
        if row.value_label {
            let value = VALUE_LABELS
                .get(row.person % VALUE_LABELS.len())
                .copied()
                .unwrap_or_default();
            body.push_str(&format!(
                "<td><span class=\"athleticDepartmentSubheader\">{value}</span></td></tr>\n"
            ));
        } else {
            body.push_str("<td></td></tr>\n");
        }
        let person = PEOPLE.get(row.person).copied().unwrap_or_default();
        let address = format!("coach{index}@example.org");
        body.push_str(&format!(
            "<tr><td><span class=\"fieldValue\">{person}</span></td>\
             <td><a href=\"mailto:{address}\">{address}</a></td></tr>\n"
        ));
        body.push_str("<tr><td colspan=\"2\"><br/></td></tr>\n");
    }
    body.push_str("</tbody></table>\n");
    body
}

type Person = (String, Option<String>);

type Office = (String, String);

type Publication = (Option<Person>, Vec<Office>);

fn expectation(rows: &[Row]) -> Publication {
    let mut director: Option<Person> = None;
    let mut office: Vec<Office> = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let role = CANONICAL.get(row.role).copied().unwrap_or_default();
        let person = PEOPLE
            .get(row.person)
            .copied()
            .unwrap_or_default()
            .to_string();
        let address = Some(format!("coach{index}@example.org"));
        if role == "athletic director" && director.is_none() {
            director = Some((person, address));
        } else {
            office.push((role.to_string(), person));
        }
    }
    (director, office)
}

proptest! {
    #![proptest_config(seam_config())]

    #[test]
    fn the_director_the_page_names_first_is_the_one_published(rows in rows()) {
        let page = parse_ad_page(&render_ad(&rows));
        let (director, office) = expectation(&rows);
        prop_assert_eq!(page.director, director, "the first named athletic director");
        prop_assert_eq!(page.office_roles, office, "the office roles behind them");
    }
}
