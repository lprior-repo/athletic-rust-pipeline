use super::*;
use census_domain::model::{AdjustmentBasis, AdjustmentKind};

const TRACK_SIZE_NOTE: &str = "Converted from 9:22.21 for Track Size. ";

type Retained = (Vec<census_domain::model::CanonicalPerformance>, u64, u64);

fn absorbed(sections: Vec<ParsedSection>) -> TestResult<Retained> {
    let url = "https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i";
    let source = SourceRef::new("tfrrs_in", Some(url.to_string()));
    let route = parse_list_path(url).ok_or("list route")?;
    let context = ListContext {
        page: Page {
            source: &source,
            observed_on: "2026-10-12",
            performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 10, 12)
                .ok_or("snapshot date")?,
            jurisdiction: UsJurisdiction::Indiana,
        },
        list: &route,
        filter: None,
    };
    let index = SchoolIndex::from_schools(&[]);
    let mut absorb = Absorb::new(&index);
    let page = ParsedList { sections };
    absorb.absorb_list(&context, &page)?;
    let converted = absorb.stats.marks_converted;
    let unconverted = absorb.stats.marks_unconverted;
    Ok((
        absorb.accumulator.performances.into_values().collect(),
        converted,
        unconverted,
    ))
}

#[test]
fn a_track_size_conversion_keeps_the_original_and_the_adjusted_mark() -> TestResult {
    let mut row = time_row("Jane Doe", 9_000_002, "9:15.75", YearToken::Junior);
    row.converted_note = Some(TRACK_SIZE_NOTE.to_string());
    let (performances, converted, _) =
        absorbed(vec![metric_section("3200 Meters", 67, vec![row])])?;
    check!(eq; converted, 1, "the host's conversion note is counted");
    let performance = performances
        .first()
        .ok_or("the converted row is retained")?;
    check!(eq;
        performance.mark,
        Mark::TimeSeconds(clock_seconds("9:15.75").ok_or("published clock")?),
        "the row publishes the host's converted mark"
    );
    let adjustment = performance
        .adjustment
        .as_ref()
        .ok_or("the conversion is retained beside the mark")?;
    check!(eq; adjustment.kind, AdjustmentKind::TrackSizeConversion);
    check!(eq; adjustment.basis, AdjustmentBasis::Known);
    check!(eq;
        adjustment.original_mark,
        Some(Mark::TimeSeconds(clock_seconds("9:22.21").ok_or("original clock")?)),
        "the mark the host converted from is the actual performance"
    );
    check!(eq; adjustment.adjusted_mark, performance.mark);
    check!(adjustment
        .note
        .as_deref()
        .is_some_and(|note| note.contains("for Track Size")));
    Ok(())
}

#[test]
fn a_fifty_five_metre_note_stays_a_published_sixty_metre_mark() -> TestResult {
    let mut row = time_row("John Doe", 9_000_003, "6.98", YearToken::Sophomore);
    row.converted_note = Some("Converted  from 6.49 (55)".to_string());
    let (performances, converted, _) = absorbed(vec![metric_section("60 Meters", 60, vec![row])])?;
    check!(eq; converted, 1, "the note is retained");
    let performance = performances.first().ok_or("the row is retained")?;
    check!(eq;
        performance.mark,
        Mark::TimeSeconds(clock_seconds("6.98").ok_or("published clock")?),
        "the published sixty metre mark is untouched"
    );
    check!(
        performance.adjustment.is_none(),
        "a 55m conversion is not a track-size conversion"
    );
    Ok(())
}

#[test]
fn an_unreadable_conversion_original_keeps_the_note_and_the_unknown_basis() -> TestResult {
    let mut row = time_row("Ann Doe", 9_000_004, "9:15.75", YearToken::Senior);
    row.converted_note = Some("Converted from NT for Track Size. ".to_string());
    let (performances, converted, _) =
        absorbed(vec![metric_section("3200 Meters", 67, vec![row])])?;
    check!(eq; converted, 1, "the host still published a conversion");
    let performance = performances.first().ok_or("the row is retained")?;
    let adjustment = performance
        .adjustment
        .as_ref()
        .ok_or("the conversion is retained")?;
    check!(eq; adjustment.kind, AdjustmentKind::TrackSizeConversion);
    check!(eq; adjustment.basis, AdjustmentBasis::Unknown);
    check!(eq;
        adjustment.original_mark,
        None,
        "an unreadable original is never invented"
    );
    check!(eq; adjustment.adjusted_mark, performance.mark);
    Ok(())
}

#[test]
fn replaying_a_converted_row_preserves_the_adjustment_without_duplication() -> TestResult {
    let mut row = time_row("Sam Doe", 9_000_005, "9:15.75", YearToken::Junior);
    row.converted_note = Some(TRACK_SIZE_NOTE.to_string());
    let (performances, converted, _) =
        absorbed(vec![metric_section("3200 Meters", 67, vec![row.clone()])])?;
    let first = performances
        .into_iter()
        .next()
        .ok_or("first absorption retained the row")?;
    let (replayed, replay_converted, _) =
        absorbed(vec![metric_section("3200 Meters", 67, vec![row])])?;
    check!(eq; replay_converted, converted, "replay counts the conversion the same way");
    let second = replayed
        .into_iter()
        .next()
        .ok_or("replay retained the row")?;
    check!(eq; first.id, second.id, "replay produces the same performance identity");
    check!(eq; first.mark, second.mark, "replay preserves the published converted mark");
    check!(
        first.adjustment.is_some() == second.adjustment.is_some(),
        "replay preserves the presence of the adjustment"
    );
    let first_adj = first.adjustment.ok_or("first had adjustment")?;
    let second_adj = second.adjustment.ok_or("replay had adjustment")?;
    check!(eq; first_adj.kind, second_adj.kind);
    check!(eq; first_adj.basis, second_adj.basis);
    check!(eq; first_adj.original_mark, second_adj.original_mark);
    check!(eq; first_adj.adjusted_mark, second_adj.adjusted_mark);
    Ok(())
}
