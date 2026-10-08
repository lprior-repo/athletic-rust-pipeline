use super::*;
use crate::tfrrs::map::{Absorb, ListContext, Page, RosterContext};
use census_domain::model::{SourceObservation, SourceRef};
use census_domain::school_index::SchoolIndex;

#[test]
fn unsupported_list_cohort_retains_locator_and_does_not_count_a_phantom_performance() -> TestResult
{
    let mut parsed = parse_list_page(LIST);
    parsed.sections.truncate(1);
    let section = parsed.sections.first_mut().ok_or("fixture section")?;
    let mut row = section.rows.remove(1);
    row.date = published_date("Mar 28, 2039");
    section.rows = vec![row];
    let index = SchoolIndex::from_schools(&[]);
    let url = "https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i";
    let source = SourceRef::new("tfrrs_in", Some(url.into()));
    let route = parse_list_path(url).ok_or("list route")?;
    let context = ListContext {
        page: Page {
            source: &source,
            observed_on: "2026-09-30",
            performance_as_of: chrono::NaiveDate::from_ymd_opt(2040, 9, 30)
                .ok_or("snapshot date")?,
            jurisdiction: UsJurisdiction::Indiana,
        },
        list: &route,
        filter: None,
    };
    let mut absorb = Absorb::new(&index);
    absorb.absorb_list(&context, &parsed)?;
    check!(eq; absorb.stats.rows_seen, 1);
    check!(eq; absorb.stats.rows_absorbed, 0);
    check!(absorb.accumulator.athletes.is_empty());
    check!(absorb.accumulator.performances.is_empty());
    let (cases, observations) = absorb.accumulator.unsupported.into_parts();
    let SourceObservation::Athlete(raw) = &observations[0] else {
        return Err("athlete observation".into());
    };
    let grade = raw.observed_grade.as_ref().ok_or("published grade")?;
    check!(eq; grade.grade.get(), 10);
    check!(eq; grade.school_year.get(), 2038);
    check!(eq; grade.grad_year(), None);
    check!(eq;
        raw.namespace,
        census_domain::model::SourceNamespace::TfrrsAthlete
    );
    check!(raw.source_row_key.contains(":row:0"));
    check!(cases[0].detail.contains(url));
    check!(cases[0].detail.contains(&raw.source_row_key));
    Ok(())
}

#[test]
fn unsupported_roster_cohort_keeps_published_grade_and_school_for_review() -> TestResult {
    let mut roster = parse_team_page(ROSTER);
    roster.season.as_mut().ok_or("published season")?.year = 2040;
    let athlete = roster
        .athletes
        .into_iter()
        .find(|row| row.full_name().is_some() && row.year.is_some())
        .ok_or("published graded athlete")?;
    roster.athletes = vec![athlete];
    let index = SchoolIndex::from_schools(&[]);
    let source = SourceRef::new(
        "tfrrs_nh",
        Some("https://nh.tfrrs.org/teams/xc/Pembroke_m.html".into()),
    );
    let team =
        parse_team_path("https://nh.tfrrs.org/teams/xc/Pembroke_m.html").ok_or("team route")?;
    let context = RosterContext {
        page: Page {
            source: &source,
            observed_on: "2026-09-30",
            performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 9, 30)
                .ok_or("snapshot date")?,
            jurisdiction: UsJurisdiction::NewHampshire,
        },
        team: &team,
    };
    let mut absorb = Absorb::new(&index);
    absorb.absorb_roster(&context, &roster);
    check!(eq; absorb.stats.roster_rows_absorbed, 0);
    check!(absorb.accumulator.athletes.is_empty());
    let (cases, observations) = absorb.accumulator.unsupported.into_parts();
    let SourceObservation::Athlete(raw) = &observations[0] else {
        return Err("athlete observation".into());
    };
    check!(eq; raw.observed_school.as_deref(), Some("Pembroke"));
    check!(eq; raw.observed_grade.as_ref().ok_or("published grade")?.school_year.get(), 2040);
    check!(cases[0].detail.contains("school year 2040"));
    Ok(())
}
