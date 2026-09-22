//! The spacing and the case a rendered schedule prints its own furniture in, and what it decides.
//!
//! The provider's page is table markup with the month in a heading row, a day in a date cell and two
//! labels in `awayteam`/`hometeam` cells, and every label it publishes has been through a stylesheet:
//! the same meet prints as `Hawkeye  Invitational  Day 1` on one page and `Hawkeye Invitational Day 1`
//! on the next, and the month heading is as likely to be `JANUARY` as `January`. Neither is a fact
//! about the competition — the venue is looked up in the state table verbatim and the date is what
//! orders the refresh — so both are collapsed before a row is published.
//!
//! The laws are stated over pages built from `tests/fixtures/wayzata/track_2026_schedule.html`, whose
//! row markup, month heading and `/links/<slug>` keys are copied here verbatim; the labels and venues
//! are that capture's own.

use super::{rendered_rows, rows, seam_config, SEASON};
use proptest::prelude::*;

/// Meet labels and venues taken from the committed track capture.
const LABELS: [&str; 4] = [
    "USATF Minnesota All-Comers Meet #3",
    "Hawkeye Invitational Day 1",
    "Jack Johnson Classic",
    "Larry Wieczorek Invitational Day 2",
];
const VENUES: [&str; 4] = [
    "University of Minnesota",
    "University of Iowa",
    "Macalester College",
    "Augustana College",
];
/// Provider keys and day cells taken from the same capture.
const SLUGS: [&str; 4] = ["15nug0", "6v95xo", "7vqvs7", "ar41us"];
const DAYS: [&str; 4] = ["Sun. 4", "Fri. 9", "Sat. 17", "Thu. 25"];

/// One competition day: the cells the row prints.
#[derive(Debug)]
struct Day {
    day: String,
    name: String,
    venue: String,
    slug: String,
}

/// One competition day per case, in the order the page prints them.
fn days() -> impl Strategy<Value = Vec<Day>> {
    prop::collection::vec(
        (
            prop::sample::select(Vec::from(DAYS)),
            prop::sample::select(Vec::from(LABELS)),
            prop::sample::select(Vec::from(VENUES)),
            prop::sample::select(Vec::from(SLUGS)),
        ),
        1..5,
    )
    .prop_map(|rows| {
        rows.into_iter()
            .map(|(day, name, venue, slug)| Day {
                day: day.to_string(),
                name: name.to_string(),
                venue: venue.to_string(),
                slug: slug.to_string(),
            })
            .collect()
    })
}

/// A label written the way a stylesheet's whitespace arrives: runs instead of single blanks.
fn spaced(text: &str) -> String {
    text.split(' ').collect::<Vec<_>>().join("  \t")
}

/// The provider's own row markup, with `spaced_text` choosing whether the label cells carry
/// whitespace runs.
fn page(heading: &str, days: &[Day], spaced_text: bool) -> String {
    let label = |text: &str| -> String {
        if spaced_text {
            spaced(text)
        } else {
            text.to_string()
        }
    };
    let mut body = String::from("<table class=\"schedule\">\n");
    body.push_str("<tr class=\"month-title bg-primary text-light\"><td colspan=\"4\"><div class=\"h5 m-0 font-weight-bold\">");
    body.push_str(heading);
    body.push_str("</div></td></tr>\n");
    for day in days {
        let name = label(&day.name);
        let venue = label(&day.venue);
        let aria = label(&format!(
            "track event: {heading} {} 12:00 AM: {} at {}: Results",
            day.day, day.name, day.venue
        ));
        body.push_str("<tr class=\"event-row away\" >\n");
        body.push_str("<td class=\"date font-weight-normal\"> ");
        body.push_str(&day.day);
        body.push_str("</td>\n");
        body.push_str(
            "<td class=\"team awayteam\"><span class=\"font-weight-normal\"> <span title=\"",
        );
        body.push_str(&name);
        body.push_str("\">");
        body.push_str(&name);
        body.push_str("</span></span></td>\n");
        body.push_str("<td class=\"team hometeam\"><span title=\"");
        body.push_str(&venue);
        body.push_str("\">");
        body.push_str(&venue);
        body.push_str("</span></td>\n");
        body.push_str("<td class=\"links\"><a aria-label=\"");
        body.push_str(&aria);
        body.push_str("\" href=\"/links/");
        body.push_str(&day.slug);
        body.push_str(
            "\" target=\"_blank\" class=\"link btn btn-link btn-sm \">Results</a></td>\n",
        );
        body.push_str("</tr>\n");
    }
    body.push_str("</table>\n");
    body
}

/// A page's rows, or the reason it published none.
fn read(name: &str, body: &str) -> Result<Vec<super::MeetRow>, TestCaseError> {
    rows(body, SEASON).map_err(|error| TestCaseError::fail(format!("{name}: {error:?}")))
}

/// The facts a heading decides, as values: the date a row is stamped with, the meet it names, the
/// venue it is held at, and the result link it carries. `aria_label` is left out on purpose — it is
/// the page's own furniture, reprinted verbatim, so the case and blanks the markup carried are its
/// business and not the heading's.
fn facts(rows: &[super::MeetRow]) -> Vec<(String, String, String, String)> {
    rows.iter()
        .map(|row| {
            (
                row.date.clone(),
                row.name.clone(),
                row.location.clone(),
                row.slug.clone().unwrap_or_default(),
            )
        })
        .collect()
}

proptest! {
    #![proptest_config(seam_config())]

    /// A venue is looked up in the state table verbatim and a meet label keys the row, so the blanks
    /// a stylesheet collapses into runs cannot reach either: the same table prints the same rows.
    #[test]
    fn whitespace_runs_in_a_label_are_collapsed(days in days()) {
        let plain = read("plain page", &page("January", &days, false))?;
        let woven = read("spaced page", &page("January", &days, true))?;
        prop_assert_eq!(
            plain.len(),
            days.len(),
            "every competition day of the table is published"
        );
        prop_assert_eq!(
            rendered_rows(&plain),
            rendered_rows(&woven),
            "the same table, spaced differently, publishes the same rows"
        );
    }

    /// The month heading is a name, not a case: `JANUARY`, `January` and a padded ` january ` all
    /// carry the same month into the dates the rows publish.
    #[test]
    fn the_month_heading_is_a_name_not_a_case(days in days(), style in 0usize..3) {
        let heading = match style {
            0 => "January".to_string(),
            1 => "JANUARY".to_string(),
            _ => "  january \t".to_string(),
        };
        let plain = read("plain page", &page("January", &days, false))?;
        let cased = read("cased page", &page(&heading, &days, false))?;
        prop_assert_eq!(
            plain.len(),
            days.len(),
            "the heading is read, so the table publishes its days"
        );
        prop_assert_eq!(
            facts(&plain),
            facts(&cased),
            "`{}` names the month `January` names",
            heading
        );
        prop_assert!(
            plain
                .iter()
                .all(|row| row.date.starts_with("2026-01-")),
            "and every row is dated in the month the heading named: {:?}",
            rendered_rows(&plain)
        );
    }
}
