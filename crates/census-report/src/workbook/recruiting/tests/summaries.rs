use super::*;

fn cross_country_contexts(fixture: &Fixture, count: usize) -> Vec<(EventId, String)> {
    (0..count)
        .map(|index| {
            let name = format!("Published XC Course {index}");
            let meet_id = meet(
                &fixture.store,
                UsJurisdiction::Wisconsin,
                &name,
                "2026-09-15",
                CompetitionLevel::Invitational,
                Sport::CrossCountry,
            );
            let event_id = event(&fixture.store, &meet_id, EventKind::CrossCountry);
            let value = 180_000_i32
                .checked_add(i32::try_from(index).unwrap().checked_mul(100).unwrap())
                .unwrap();
            let mark = Mark::TimeSeconds(CentiSeconds::new(value));
            let displayed = bests::mark_text(&mark);
            performance(
                &fixture.store,
                &PerformanceRow {
                    athlete: &fixture.julian,
                    school: &fixture.wi_school,
                    meet: &meet_id,
                    event: &event_id,
                    kind: &EventKind::CrossCountry,
                    date: "2026-09-15",
                },
                mark,
                "wiaa_results",
                "https://wiaa.test/xc/results",
            );
            (event_id, displayed)
        })
        .collect()
}

#[test]
fn headline_and_wide_cells_retain_every_compatible_course_context_after_ten_prs() {
    let fixture = fixture();
    let contexts = cross_country_contexts(&fixture, 12);
    let (mut book, _) = written(&fixture);
    let athletes = sheet(&mut book, "Athletes");
    let row = row_of(&athletes, fixture.julian.as_str());
    let headline = text(&athletes, row, column_of(&athletes, "Headline PR summary"));
    let wide = text(&athletes, row, column_of(&athletes, "XC (s)"));
    let prs = sheet(&mut book, "PRs");
    for (event, mark) in contexts {
        for summary in [&headline, &wide] {
            assert!(
                summary.contains(&format!("XC {mark} [xc, na, unknown, event {event}]")),
                "{summary}"
            );
        }
        let pr = row_where(&prs, |row| text(&prs, row, 25) == event.as_str());
        assert_eq!(text(&prs, pr, 9), mark);
        assert_eq!(text(&prs, pr, 0), fixture.julian.as_str());
    }
}

#[test]
fn excel_limit_is_explicit_while_the_pr_sheet_retains_all_course_contexts() {
    let fixture = fixture();
    let contexts = cross_country_contexts(&fixture, 700);
    let (mut book, _) = written(&fixture);
    let athletes = sheet(&mut book, "Athletes");
    let row = row_of(&athletes, fixture.julian.as_str());
    for header in ["Headline PR summary", "XC (s)"] {
        let summary = text(&athletes, row, column_of(&athletes, header));
        assert!(summary.ends_with("; additional classified marks in PRs"));
        assert!(summary.encode_utf16().count() <= 32_767);
    }
    let prs = sheet(&mut book, "PRs");
    assert_eq!(prs.height(), 702);
    for (event, mark) in contexts {
        let pr = row_where(&prs, |row| text(&prs, row, 25) == event.as_str());
        assert_eq!(text(&prs, pr, 9), mark);
        assert_eq!(text(&prs, pr, 16), "https://wiaa.test/xc/results");
    }
    verified_publication(&fixture.store, 2);
}

#[test]
fn complete_summary_that_fits_excel_does_not_reserve_an_overflow_marker() {
    let fixture = fixture();
    let contexts = cross_country_contexts(&fixture, 555);
    let (mut book, _) = written(&fixture);
    let athletes = sheet(&mut book, "Athletes");
    let row = row_of(&athletes, fixture.julian.as_str());
    let wide = text(&athletes, row, column_of(&athletes, "XC (s)"));
    assert_eq!(wide.encode_utf16().count(), 32_743);
    for (event, mark) in contexts {
        assert!(wide.contains(&format!("XC {mark} [xc, na, unknown, event {event}]")));
    }
    verified_publication(&fixture.store, 2);
}
