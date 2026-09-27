use super::seam_config;
use census_crawl::wiaa::{parse_directory_letter, parse_enrollment, parse_school_page};
use proptest::prelude::*;

const NAMES: [&str; 4] = ["Abbotsford", "Adams-Friendship", "Altoona", "Amherst"];
const LEVELS: [&str; 3] = ["High School", "Middle School", "Combined"];
const CITIES: [&str; 4] = ["Abbotsford", "Adams", "Altoona", "Amherst Junction"];

#[derive(Debug)]
struct IndexRow {
    org_id: u32,
    name: String,
    level: String,
    city: String,
}

fn index_rows() -> impl Strategy<Value = Vec<IndexRow>> {
    prop::collection::vec(
        (
            1u32..40_000,
            prop::sample::select(Vec::from(NAMES)),
            prop::sample::select(Vec::from(LEVELS)),
            prop::sample::select(Vec::from(CITIES)),
        ),
        1..5,
    )
    .prop_map(|rows| {
        rows.into_iter()
            .map(|(org_id, name, level, city)| IndexRow {
                org_id,
                name: name.to_string(),
                level: level.to_string(),
                city: city.to_string(),
            })
            .collect()
    })
}

fn render_letter(rows: &[IndexRow]) -> String {
    let mut body = String::from(
        "<div class=\"panel gridFonts\"><div id=\"tableContainer\"><table id=\"tblSchools\" \
         class=\"table table-striped align-middle\"><thead><tr><th class=\"no-sort\">School</th>\
         </tr></thead><tbody>\n",
    );
    for row in rows {
        body.push_str(&format!(
            "<tr><td><a href=\"/Directory/GetDirectorySchool?orgID={}\" title=\"{}\">{}</a>\
             <label class=\"gridTextDataTables\">{}</label>\
             <label class=\"gridTextDataTables\">Public</label>\
             <label class=\"gridTextDataTables\">{}</label></td></tr>\n",
            row.org_id, row.name, row.name, row.level, row.city
        ));
    }
    body.push_str("</tbody></table></div></div>\n");
    body
}

#[derive(Debug)]
struct PageFacts {
    name: String,
    enrollment: u32,
    admins: usize,
    coaches: usize,
}

fn page_facts() -> impl Strategy<Value = PageFacts> {
    (
        prop::sample::select(Vec::from(NAMES)),
        1u32..2_000,
        1..5usize,
        1..5usize,
    )
        .prop_map(|(name, enrollment, admins, coaches)| PageFacts {
            name: name.to_string(),
            enrollment,
            admins,
            coaches,
        })
}

fn render_school(page: &PageFacts) -> String {
    let mut body = format!(
        "<div class=\"container\"><label class=\"JumboMain\">{}</label>\n\
         <span>Level</span><h5 class=\"m-0\">High School</h5>\n\
         <span>Enrollment (2026-2027)</span><span>School:</span><b>{}</b>\n",
        page.name, page.enrollment
    );
    body.push_str("<table id=\"tblAdminList\" class=\"table\"><tbody>\n");
    for index in 0..page.admins {
        body.push_str(&format!(
            "<tr><td>1</td><td>Athletic Director</td><td><b>Director {index}</b></td>\
             <td><a href=\"mailto:director{index}@example.org\">Mail</a></td></tr>\n"
        ));
    }
    body.push_str("</tbody></table>\n<table id=\"tblCoachList\" class=\"table\"><tbody>\n");
    for index in 0..page.coaches {
        body.push_str(&format!(
            "<tr><td>1</td><td>Boys Track and Field</td><td><b>Coach {index}</b></td>\
             <td>Head Coach</td><td><a href=\"mailto:coach{index}@example.org\">Mail</a></td></tr>\n"
        ));
    }
    body.push_str("</tbody></table></div>\n");
    body
}

proptest! {
    #![proptest_config(seam_config())]

    #[test]
    fn an_index_row_publishes_the_id_and_labels_it_prints(rows in index_rows()) {
        let entries = parse_directory_letter(&render_letter(&rows));
        prop_assert_eq!(entries.len(), rows.len(), "every printed row is published");
        for (entry, row) in entries.iter().zip(rows.iter()) {
            let org_id = row.org_id.to_string();
            prop_assert_eq!(
                entry.org_id.as_str(),
                org_id.as_str(),
                "the row publishes its own `orgID`"
            );
            prop_assert_eq!(entry.name.as_str(), row.name.as_str(), "and its own name");
            prop_assert_eq!(entry.level.as_str(), row.level.as_str(), "and its own level");
            prop_assert_eq!(entry.city.as_str(), row.city.as_str(), "and its own city");
        }
    }

    #[test]
    fn a_school_page_publishes_the_name_and_totals_it_prints(page in page_facts()) {
        let body = render_school(&page);
        let parsed = parse_school_page(&body);
        prop_assert_eq!(parsed.name.as_str(), page.name.as_str(), "the page's own name");
        prop_assert_eq!(
            parsed.enrollment,
            Some(page.enrollment),
            "the `Enrollment (<school year>)` total"
        );
        prop_assert_eq!(parsed.admins.len(), page.admins, "every printed administration row");
        prop_assert_eq!(parsed.coaches.len(), page.coaches, "every printed coach row");
        prop_assert_eq!(parse_enrollment(&body), Some(page.enrollment), "as read on its own");
    }
}
