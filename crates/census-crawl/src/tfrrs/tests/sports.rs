use super::{Absorb, ListContext, Page, SchoolIndex, SourceRef, Sport, UsJurisdiction, LIST};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const BASE: &str = "https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List";

#[test]
fn tfrrs_recorded_list_meets_retain_publication_sport() -> TestResult {
    let url = format!("{BASE}/2026/i");
    let route = crate::tfrrs::parse_list_path(&url).ok_or("published list route")?;
    let source = SourceRef::new("tfrrs_in", Some(url.clone()));
    let index = SchoolIndex::from_schools(&[]);
    let mut absorb = Absorb::new(&index);
    let page = Page {
        source: &source,
        observed_on: "2026-09-22T03:55:31Z",
        performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 10, 12).ok_or("date")?,
        jurisdiction: UsJurisdiction::Indiana,
    };
    let parsed = crate::tfrrs::parse_list_page(LIST);
    absorb.absorb_list(
        &ListContext {
            page,
            list: &route,
            filter: None,
        },
        &parsed,
    )?;
    check!(
        !absorb.accumulator.meets.is_empty(),
        "fixture must project meets"
    );
    for meet in absorb.accumulator.meets.values() {
        check!(eq; meet.sports.as_slice(), &[Sport::IndoorTrack], "{} loses publication sport", meet.name);
    }
    let conflicting = crate::tfrrs::parse_list_path(&format!("{BASE}/2026/o"))
        .ok_or("conflicting published route")?;
    let error = absorb
        .absorb_list(
            &ListContext {
                page,
                list: &conflicting,
                filter: None,
            },
            &parsed,
        )
        .err()
        .ok_or("conflicting publication was silently accepted")?;
    check!(error
        .to_string()
        .contains("publications disagree on meet sport"));
    for meet in absorb.accumulator.meets.values() {
        check!(eq; meet.sports.as_slice(), &[Sport::OutdoorTrack, Sport::IndoorTrack]);
        check!(eq; meet.retained_conflicts.len(), 1);
    }
    let unknown = crate::tfrrs::parse_list_path(BASE).ok_or("unknown sport route")?;
    let mut unknown_projection = Absorb::new(&index);
    unknown_projection.absorb_list(
        &ListContext {
            page,
            list: &unknown,
            filter: Some(crate::tfrrs::parse::YearToken::Junior),
        },
        &parsed,
    )?;
    check!(eq; unknown_projection.accumulator.meets.len(), 0);
    check!(eq; unknown_projection.accumulator.performances.len(), 0);
    check!(unknown_projection.stats.rows_without_season > 0);
    Ok(())
}
