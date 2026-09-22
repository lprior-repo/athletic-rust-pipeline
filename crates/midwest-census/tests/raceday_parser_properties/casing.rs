//! The case and the spacing a provider prints its page furniture in, and what that furniture is
//! allowed to decide.
//!
//! RaceDay exports come from several meet managers' installs of the same product, and the archive
//! carries both `Place` and `PLACE`, both a padded ` Ada Bell ` cell and a flush one, and both `D2`
//! and `Division 2` in a race title. None of that is a fact about the race: the census keys columns on
//! their labels and the side on the words of the title, so a page typed in another case — or padded by
//! another manager's stylesheet — must read to the very same athletes, schools and finishes.
//!
//! Both laws are stated over pages built from the shapes this seam commits: `DECLINED_ROW`'s
//! five-column grid, the capture's `<span>`-wrapped header labels, and one title per spelling of a
//! division. The cells come from the published values of both — `Ada Bell`/`Whitewater`/`19:02.10`
//! from the declined-row page, `Aaron Thomas`/`05:21.42` from the Racine sectional capture.

use super::{parse_body, rendered_rows, seam_config, ParsedMeet};
use census_domain::model::Gender;
use proptest::prelude::*;

/// The five columns of a finish list, in the order the committed capture prints them.
const LABELS: [&str; 5] = ["Place", "Name", "Year", "Team Name", "Finish"];

/// Athlete cells taken from the committed capture and the lane's own pages.
const NAMES: [&str; 4] = ["Ada Bell", "Jack Hefty", "Aaron Thomas", "Zachary Haleem"];
const SCHOOLS: [&str; 3] = ["Whitewater", "East Troy", "Seymour"];
const YEARS: [&str; 3] = ["9", "11", "12"];
const FINISHES: [&str; 4] = ["05:21.42", "17:13.69", "19:02.10", "20:11.44"];

/// One athlete row of a generated page.
#[derive(Debug)]
struct Athlete {
    place: String,
    name: String,
    year: String,
    school: String,
    finish: String,
}

impl Athlete {
    /// The five cells in the order the layout prints them.
    fn cells(&self) -> [&str; 5] {
        [
            &self.place,
            &self.name,
            &self.year,
            &self.school,
            &self.finish,
        ]
    }
}

/// One athlete row per case, placed in the order the grid prints them.
fn athletes() -> impl Strategy<Value = Vec<Athlete>> {
    prop::collection::vec(
        (
            prop::sample::select(Vec::from(NAMES)),
            prop::sample::select(Vec::from(YEARS)),
            prop::sample::select(Vec::from(SCHOOLS)),
            prop::sample::select(Vec::from(FINISHES)),
        ),
        1..6,
    )
    .prop_map(|rows| {
        rows.into_iter()
            .enumerate()
            .map(|(index, (name, year, school, finish))| Athlete {
                place: index.saturating_add(1).to_string(),
                name: name.to_string(),
                year: year.to_string(),
                school: school.to_string(),
                finish: finish.to_string(),
            })
            .collect()
    })
}

/// A cell the way a manager's stylesheet pads it: blanks and a no-break space around the value.
fn padded(cell: &str) -> String {
    format!("  &nbsp;\u{a0}{cell}&nbsp;  ")
}

/// One export page: the wrapped header labels the capture prints, the grid rows `DECLINED_ROW`
/// prints, and `labels_upper`/`cells_padded` choosing how the page is written.
fn page(title: &str, rows: &[Athlete], labels_upper: bool, cells_padded: bool) -> String {
    let mut body =
        String::from("<html><body>\n<h3 id=\"section-2\"><span class=\"react-tooltip-wrapper\">");
    body.push_str(title);
    body.push_str("</span></h3>\n<table class=\"data-display striped full-width no-wrap\">\n");
    body.push_str("<thead class=\"header\" id=\"header-1\"><tr>");
    for label in LABELS {
        let label = if labels_upper {
            label.to_ascii_uppercase()
        } else {
            label.to_string()
        };
        body.push_str("<th class=\"text-center\"><span class=\"react-tooltip-wrapper\">");
        body.push_str(&label);
        body.push_str("</span></th>");
    }
    body.push_str("</tr></thead>\n<tbody>\n");
    for row in rows {
        body.push_str("<tr>");
        for cell in row.cells() {
            let cell = if cells_padded {
                padded(cell)
            } else {
                cell.to_string()
            };
            body.push_str("<td>");
            body.push_str(&cell);
            body.push_str("</td>");
        }
        body.push_str("</tr>\n");
    }
    body.push_str("</tbody>\n</table>\n</body></html>\n");
    body
}

/// One race title: the division written the way `abbreviated` writes it, the side that ran, and
/// `upper` choosing the case the side is printed in.
fn title(division: u8, girls: bool, upper: bool, abbreviated: bool) -> String {
    let side = if girls { "Girls" } else { "Boys" };
    let side = if upper {
        side.to_ascii_uppercase()
    } else {
        side.to_string()
    };
    let head = if abbreviated {
        format!("WIAA D{division} XC Sectionals")
    } else {
        format!("WIAA Division {division} XC Sectionals")
    };
    format!("{head} - {side} Race Team Finish List-XC")
}

/// A page's parse, or the reason it is not a meet.
fn meet(page: &str) -> Result<ParsedMeet, TestCaseError> {
    parse_body(page).map_err(|error| TestCaseError::fail(format!("the page is a meet: {error:?}")))
}

/// The law: the page's case and padding decide nothing about the readings it yields.
fn same_readings(plain: &str, decorated: &str, rows: usize) -> Result<(), TestCaseError> {
    let plain = meet(plain)?;
    let decorated = meet(decorated)?;
    prop_assert_eq!(
        plain.rows_parsed,
        rows,
        "every athlete row of the grid is a performance"
    );
    prop_assert_eq!(
        rendered_rows(&plain),
        rendered_rows(&decorated),
        "the same grid, typed and padded differently, reads to the same rows"
    );
    Ok(())
}

proptest! {
    #![proptest_config(seam_config())]

    /// A column is its label's words, not their case, and a cell is its value, not the blanks a
    /// stylesheet pads it with: the manager who SHOUTS a header or pads a cell publishes the same
    /// finish list as the one who does not.
    #[test]
    fn label_case_and_cell_padding_do_not_move_a_reading(rows in athletes()) {
        let race = title(2, false, false, true);
        same_readings(
            &page(&race, &rows, false, false),
            &page(&race, &rows, true, true),
            rows.len(),
        )?;
    }

    /// A race is classified from the words the page prints, not from how it capitalises them, and the
    /// two spellings the archive uses for a division name the same division.
    #[test]
    fn a_race_is_classified_from_its_words(
        division in 1u8..4,
        girls in any::<bool>(),
        upper in any::<bool>(),
        rows in athletes(),
    ) {
        let long = title(division, girls, upper, false);
        let short = title(division, girls, upper, true);
        let from_long = meet(&page(&long, &rows, false, false))?;
        let from_short = meet(&page(&short, &rows, false, false))?;

        let side = if girls { Gender::Girls } else { Gender::Boys };
        let division_name = format!("Division {division}");
        for (name, title, parsed) in [
            ("division spelled out", long.as_str(), &from_long),
            ("D-abbreviation", short.as_str(), &from_short),
        ] {
            let event = parsed
                .events
                .first()
                .ok_or_else(|| TestCaseError::fail(format!("{name}: the page publishes the race")))?;
            prop_assert_eq!(
                event.gender,
                side,
                "{}: `{}` names the side that ran, whatever its case",
                name,
                title
            );
            prop_assert_eq!(
                event.division.as_deref(),
                Some(division_name.as_str()),
                "{}: `{}` names its division",
                name,
                title
            );
        }
        prop_assert_eq!(
            rendered_rows(&from_long),
            rendered_rows(&from_short),
            "the two spellings of one race read to the same rows"
        );
    }
}
