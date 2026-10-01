use super::*;
use crate::tfrrs::map::{Absorb, ListContext, Page, RosterContext};
use census_domain::model::{SourceObservation, SourceRef};
use census_domain::school_index::SchoolIndex;

#[test]
fn unsupported_list_cohort_retains_locator_and_does_not_count_a_phantom_performance() {
    let mut parsed = parse_list_page(LIST);
    parsed.sections.truncate(1);
    let section = parsed.sections.first_mut().unwrap();
    let mut row = section.rows.remove(1);
    row.date = published_date("Mar 28, 2039");
    section.rows = vec![row];
    let index = SchoolIndex::from_schools(&[]);
    let url = "https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i";
    let source = SourceRef::new("tfrrs_in", Some(url.into()));
    let route = parse_list_path(url).unwrap();
    let context = ListContext {
        page: Page {
            source: &source,
            observed_on: "2026-09-30",
            jurisdiction: UsJurisdiction::Indiana,
        },
        list: &route,
        filter: None,
    };
    let mut absorb = Absorb::new(&index);
    absorb.absorb_list(&context, &parsed);
    assert_eq!(absorb.stats.rows_seen, 1);
    assert_eq!(absorb.stats.rows_absorbed, 0);
    assert!(absorb.accumulator.athletes.is_empty());
    assert!(absorb.accumulator.performances.is_empty());
    let (cases, observations) = absorb.accumulator.unsupported.into_parts();
    let SourceObservation::Athlete(raw) = &observations[0] else {
        panic!("athlete observation")
    };
    let grade = raw.observed_grade.as_ref().unwrap();
    assert_eq!(grade.grade.get(), 10);
    assert_eq!(grade.school_year.get(), 2038);
    assert_eq!(grade.grad_year(), None);
    assert_eq!(
        raw.namespace,
        census_domain::model::SourceNamespace::TfrrsAthlete
    );
    assert!(raw.source_row_key.contains(":row:0"));
    assert!(cases[0].detail.contains(url));
    assert!(cases[0].detail.contains(&raw.source_row_key));
}

#[test]
fn unsupported_roster_cohort_keeps_published_grade_and_school_for_review() {
    let mut roster = parse_team_page(ROSTER);
    roster.season.as_mut().unwrap().year = 2040;
    let athlete = roster
        .athletes
        .into_iter()
        .find(|row| row.full_name().is_some() && row.year.is_some())
        .unwrap();
    roster.athletes = vec![athlete];
    let index = SchoolIndex::from_schools(&[]);
    let source = SourceRef::new(
        "tfrrs_nh",
        Some("https://nh.tfrrs.org/teams/xc/Pembroke_m.html".into()),
    );
    let team = parse_team_path("https://nh.tfrrs.org/teams/xc/Pembroke_m.html").unwrap();
    let context = RosterContext {
        page: Page {
            source: &source,
            observed_on: "2026-09-30",
            jurisdiction: UsJurisdiction::NewHampshire,
        },
        team: &team,
    };
    let mut absorb = Absorb::new(&index);
    absorb.absorb_roster(&context, &roster);
    assert_eq!(absorb.stats.roster_rows_absorbed, 0);
    assert!(absorb.accumulator.athletes.is_empty());
    let (cases, observations) = absorb.accumulator.unsupported.into_parts();
    let SourceObservation::Athlete(raw) = &observations[0] else {
        panic!("athlete observation")
    };
    assert_eq!(raw.observed_school.as_deref(), Some("Pembroke"));
    assert_eq!(raw.observed_grade.as_ref().unwrap().school_year.get(), 2040);
    assert!(cases[0].detail.contains("school year 2040"));
}
