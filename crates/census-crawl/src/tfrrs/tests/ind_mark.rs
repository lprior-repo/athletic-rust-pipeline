use super::*;
use census_domain::model::CanonicalPerformance;

fn published_row(
    name: &str,
    athlete_id: u64,
    mark: &str,
    wind: Option<f64>,
    converted_note: Option<&str>,
) -> ParsedRow {
    ParsedRow {
        place: Some(1),
        athlete: Some(ParsedAthlete {
            id: Some(athlete_id),
            name: name.to_string(),
            href_name: Some(name.to_string()),
        }),
        relay_members: Vec::new(),
        team: Some(ParsedTeam {
            name: "Kokomo".to_string(),
            path: "https://indiana.tfrrs.org/teams/tf/Kokomo_m.html".to_string(),
            gender: Some(Gender::Boys),
        }),
        year: Some(YearToken::Junior),
        mark: Some(ParsedMark::Time(mark.to_string())),
        conv_metres: None,
        converted_note: converted_note.map(str::to_string),
        meet: Some(ParsedMeet {
            name: "Hoosier State Relays".to_string(),
            id: Some(94159),
            path: None,
        }),
        date: published_date("Mar 28, 2026"),
        wind,
    }
}

fn section(rows: Vec<ParsedRow>) -> ParsedSection {
    ParsedSection {
        label: "60 Meters".to_string(),
        gender: Some(Gender::Boys),
        event_hnd: Some(46),
        rows,
    }
}

fn absorbed_pair(first: ParsedRow, second: ParsedRow) -> TestResult<Vec<CanonicalPerformance>> {
    let url = "https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i";
    let source = SourceRef::new("tfrrs_in", Some(url.to_string()));
    let route = parse_list_path(url).ok_or("list route")?;
    let page = Page {
        source: &source,
        observed_on: "2026-09-30",
        performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 9, 30).ok_or("as of date")?,
        jurisdiction: UsJurisdiction::Indiana,
    };
    let context = ListContext {
        page,
        list: &route,
        filter: None,
    };
    let index = SchoolIndex::from_schools(&[]);
    let mut absorb = Absorb::new(&index);
    let first_page = ParsedList {
        sections: vec![section(vec![first])],
    };
    absorb.absorb_list(&context, &first_page)?;
    let second_page = ParsedList {
        sections: vec![section(vec![second])],
    };
    absorb.absorb_list(&context, &second_page)?;
    Ok(absorb.accumulator.performances.into_values().collect())
}

#[test]
fn ind_mark_conflict_faster_first() -> TestResult {
    let performances = absorbed_pair(
        published_row(
            "John Doe",
            9_000_101,
            "9:15.75",
            None,
            Some("converted from 9:22.21 for track size"),
        ),
        published_row(
            "John Doe",
            9_000_101,
            "9:15.75",
            None,
            Some("converted from 9:21.90 for track size"),
        ),
    )?;
    check!(eq; performances.len(), 1, "same identity merges to one row");
    let performance = &performances[0];
    check!(
        eq; performance.retained_conflicts.len(), 1,
        "two adjusted reports of the same published mark retain a conflict"
    );
    check!(
        performance.retained_conflicts[0].family == "Individual result scalar",
        "the conflict family names the contradiction"
    );
    Ok(())
}

#[test]
fn ind_mark_conflict_slower_first() -> TestResult {
    let performances = absorbed_pair(
        published_row(
            "John Doe",
            9_000_102,
            "9:15.75",
            None,
            Some("converted from 9:21.90 for track size"),
        ),
        published_row(
            "John Doe",
            9_000_102,
            "9:15.75",
            None,
            Some("converted from 9:22.21 for track size"),
        ),
    )?;
    check!(eq; performances.len(), 1, "same identity merges to one row");
    let performance = &performances[0];
    check!(
        eq; performance.retained_conflicts.len(), 1,
        "order of arrival does not hide the conflict"
    );
    check!(
        performance.retained_conflicts[0].family == "Individual result scalar",
        "the conflict family names the contradiction"
    );
    Ok(())
}

#[test]
fn ind_mark_identical_duplicate_collapses() -> TestResult {
    let performances = absorbed_pair(
        published_row("John Doe", 9_000_103, "50.00", None, None),
        published_row("John Doe", 9_000_103, "50.00", None, None),
    )?;
    check!(eq; performances.len(), 1, "byte-identical duplicates collapse");
    let performance = &performances[0];
    check!(
        performance.retained_conflicts.is_empty(),
        "identical duplicates produce no conflict"
    );
    Ok(())
}

#[test]
fn ind_mark_wind_differs_retains_both() -> TestResult {
    let performances = absorbed_pair(
        published_row("John Doe", 9_000_104, "50.00", Some(0.5), None),
        published_row("John Doe", 9_000_104, "50.00", Some(2.0), None),
    )?;
    check!(eq; performances.len(), 1, "same identity merges to one row");
    let performance = &performances[0];
    check!(
        performance.retained_conflicts.is_empty(),
        "wind alone is not a scalar conflict; both facts merge"
    );
    check!(
        performance.wind_mps == Some(0.5),
        "the first wind is retained"
    );
    Ok(())
}
