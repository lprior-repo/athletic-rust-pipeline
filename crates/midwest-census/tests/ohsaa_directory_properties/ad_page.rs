//! The athletic-department table: label rows naming a role, value rows carrying the person.
//!
//! The association writes this table as pairs of rows: a row of `athleticDepartmentSubheader` spans
//! naming the role (and repeating an `Email:` label), then a row whose `fieldValue` span carries the
//! person and whose mailto href carries the address. `Phone:`/`Fax:` labels usually carry their value
//! in the label row itself, and separator rows are `<br>`. The laws: the first row that names an
//! athletic director is the director, every other named office role is published under the
//! association's own label, and a name printed under a label the pipeline does not track is not a
//! role at all.

use super::seam_config;
use midwest_census::sources::ohsaa::parse_ad_page;
use proptest::prelude::*;

/// The label rows the capture prints, and the canonical role each one is published under.
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
/// The labels that name a value column rather than a person, as the capture prints them.
const VALUE_LABELS: [&str; 2] = ["Email:", "Phone: (614) 718-8142"];
/// The people printed under those labels.
const PEOPLE: [&str; 4] = ["Jennifer Music", "Adam Banks", "Barry Mink", "Joe DePalma"];

/// One printed row pair: which role label precedes which person.
#[derive(Debug, Clone)]
struct Row {
    role: usize,
    person: usize,
    value_label: bool,
}

/// The table's row pairs, with the association's value labels mixed in.
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

/// The table markup `ad_dublin_coffman.html` prints: a label row, then the value row it labels.
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

/// One published person: the name the table printed and the address its row carried.
type Person = (String, Option<String>);

/// One office role behind the director, as the association's own label publishes it.
type Office = (String, String);

/// What the page publishes: the director it names first, and the office roles behind them.
type Publication = (Option<Person>, Vec<Office>);

/// The director the page names first, and the office roles behind them, in document order — the shape
/// the parser documents.
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

    /// The director the page names first is the director published, with the address that row
    /// carries, and every other named office role is published under the association's own label with
    /// the person that row printed.
    #[test]
    fn the_director_the_page_names_first_is_the_one_published(rows in rows()) {
        let page = parse_ad_page(&render_ad(&rows));
        let (director, office) = expectation(&rows);
        prop_assert_eq!(page.director, director, "the first named athletic director");
        prop_assert_eq!(page.office_roles, office, "the office roles behind them");
    }
}
