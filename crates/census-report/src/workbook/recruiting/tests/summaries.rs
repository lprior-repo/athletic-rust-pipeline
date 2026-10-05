use super::*;

fn five_k_performance(fixture: &Fixture, sport: Sport) -> TestResult<(EventId, String)> {
    let meet_id = meet(
        &fixture.store,
        UsJurisdiction::Wisconsin,
        &format!("Published 5000m {}", sport.stable_key()),
        "2026-09-15",
        CompetitionLevel::Invitational,
        sport,
    )?;
    let kind = EventKind::Track5000m;
    let event_id = event(&fixture.store, &meet_id, kind.clone())?;
    let mark = Mark::TimeSeconds(CentiSeconds::new(180_000));
    let displayed = bests::mark_text(&mark);
    performance(
        &fixture.store,
        &PerformanceRow {
            athlete: &fixture.julian,
            school: &fixture.wi_school,
            meet: &meet_id,
            event: &event_id,
            kind: &kind,
            date: "2026-09-15",
        },
        mark,
        "wiaa_results",
        "https://wiaa.test/5000m/results",
    )?;
    Ok((event_id, displayed))
}

#[test]
fn cross_country_5000m_fills_xc_cell_and_leaves_track_5000m_empty() -> TestResult {
    let fixture = fixture()?;
    let (event, mark) = five_k_performance(&fixture, Sport::CrossCountry)?;

    let (mut book, _) = written(&fixture)?;
    let athletes = sheet(&mut book, "Athletes")?;
    let row = row_of(&athletes, fixture.julian.as_str())?;

    check!(eq; text(&athletes, row, column_of(&athletes, "XC (s)")?),
        format!("XC {mark} [xc, na, unknown, event {event}]"));
    check!(eq; text(&athletes, row, column_of(&athletes, "5000m (s)")?), "");
    Ok(())
}

#[test]
fn outdoor_5000m_fills_track_5000m_cell_and_leaves_xc_empty() -> TestResult {
    let fixture = fixture()?;
    let (_, mark) = five_k_performance(&fixture, Sport::OutdoorTrack)?;

    let (mut book, _) = written(&fixture)?;
    let athletes = sheet(&mut book, "Athletes")?;
    let row = row_of(&athletes, fixture.julian.as_str())?;

    check!(eq; text(&athletes, row, column_of(&athletes, "5000m (s)")?),
        format!("5000m {mark} [outdoor, na, unknown]"));
    check!(eq; text(&athletes, row, column_of(&athletes, "XC (s)")?), "");
    Ok(())
}

#[test]
fn event_list_and_headline_name_cross_country_5000m_as_xc() -> TestResult {
    let fixture = fixture()?;
    let (event, mark) = five_k_performance(&fixture, Sport::CrossCountry)?;

    let (mut book, _) = written(&fixture)?;
    let athletes = sheet(&mut book, "Athletes")?;
    let row = row_of(&athletes, fixture.julian.as_str())?;
    let headline = text(&athletes, row, column_of(&athletes, "Headline PR summary")?);

    check!(eq; text(&athletes, row, column_of(&athletes, "Event list")?), "XC; 400m");
    check!(
        headline
            .split("; ")
            .any(|entry| entry == format!("XC {mark} [xc, na, unknown, event {event}]")),
        "{headline}"
    );
    check!(!headline.contains("5000m"), "{headline}");
    Ok(())
}

fn cross_country_contexts(fixture: &Fixture, count: usize) -> TestResult<Vec<(EventId, String)>> {
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
            )?;
            let event_id = event(&fixture.store, &meet_id, EventKind::CrossCountry)?;
            let offset = i32::try_from(index)?
                .checked_mul(100)
                .ok_or("fixture mark offset overflow")?;
            let value = 180_000_i32
                .checked_add(offset)
                .ok_or("fixture mark overflow")?;
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
            )?;
            Ok((event_id, displayed))
        })
        .collect()
}

#[test]
fn headline_and_wide_cells_retain_every_compatible_course_context_after_ten_prs() -> TestResult {
    let fixture = fixture()?;
    let contexts = cross_country_contexts(&fixture, 12)?;
    let (mut book, _) = written(&fixture)?;
    let athletes = sheet(&mut book, "Athletes")?;
    let row = row_of(&athletes, fixture.julian.as_str())?;
    let headline = text(&athletes, row, column_of(&athletes, "Headline PR summary")?);
    let wide = text(&athletes, row, column_of(&athletes, "XC (s)")?);
    let prs = sheet(&mut book, "PRs")?;
    for (event, mark) in contexts {
        for summary in [&headline, &wide] {
            check!(
                summary.contains(&format!("XC {mark} [xc, na, unknown, event {event}]")),
                "{summary}"
            );
        }
        let pr = row_where(&prs, |row| text(&prs, row, 25) == event.as_str())?;
        check!(eq; text(&prs, pr, 9), mark);
        check!(eq; text(&prs, pr, 0), fixture.julian.as_str());
    }
    Ok(())
}

#[test]
fn excel_limit_is_explicit_while_the_pr_sheet_retains_all_course_contexts() -> TestResult {
    let fixture = fixture()?;
    let contexts = cross_country_contexts(&fixture, 700)?;
    let (mut book, _) = written(&fixture)?;
    let athletes = sheet(&mut book, "Athletes")?;
    let row = row_of(&athletes, fixture.julian.as_str())?;
    for header in ["Headline PR summary", "XC (s)"] {
        let summary = text(&athletes, row, column_of(&athletes, header)?);
        check!(summary.ends_with("; additional classified marks in PRs"));
        check!(summary.encode_utf16().count() <= 32_767);
    }
    let prs = sheet(&mut book, "PRs")?;
    check!(eq; prs.height(), 702);
    for (event, mark) in contexts {
        let pr = row_where(&prs, |row| text(&prs, row, 25) == event.as_str())?;
        check!(eq; text(&prs, pr, 9), mark);
        check!(eq; text(&prs, pr, 16), "https://wiaa.test/xc/results");
    }
    verified_publication(&fixture.store, 2)?;
    Ok(())
}

#[test]
fn complete_summary_that_fits_excel_does_not_reserve_an_overflow_marker() -> TestResult {
    let fixture = fixture()?;
    let contexts = cross_country_contexts(&fixture, 555)?;
    let (mut book, _) = written(&fixture)?;
    let athletes = sheet(&mut book, "Athletes")?;
    let row = row_of(&athletes, fixture.julian.as_str())?;
    let wide = text(&athletes, row, column_of(&athletes, "XC (s)")?);
    check!(eq; wide.encode_utf16().count(), 32_743);
    for (event, mark) in contexts {
        check!(wide.contains(&format!("XC {mark} [xc, na, unknown, event {event}]")));
    }
    verified_publication(&fixture.store, 2)?;
    Ok(())
}

fn frozen_publication(
    fixture: &Fixture,
) -> TestResult<(
    std::path::PathBuf,
    crate::export::ExportDataset,
    crate::workbook::Options,
)> {
    let options = crate::workbook::Options {
        school_year: Some(SchoolYear::new(2026).ok_or("invalid fixture season")?),
        ..crate::workbook::Options::default()
    };
    let published = crate::workbook::build(&fixture.store, &options)?;
    let generation = published.parent().ok_or("missing generation directory")?;
    let dataset =
        crate::export::ExportDataset::reopen_frozen(&generation.join("frozen-input.json"))?;
    Ok((published, dataset, options))
}

#[test]
fn frozen_readback_accepts_a_cross_country_surface_5000m_mark() -> TestResult {
    let fixture = fixture()?;
    five_k_performance(&fixture, Sport::CrossCountry)?;

    let (published, dataset, options) = frozen_publication(&fixture)?;
    crate::workbook::verify::verify_frozen(&published, &dataset, &options)?;
    Ok(())
}

fn forged_workbook(
    source: &std::path::Path,
    target: &std::path::Path,
    header: &str,
    value: &str,
) -> TestResult {
    let mut reader: Xlsx<_> = open_workbook(source)?;
    let mut writer = Workbook::new();
    for sheet_name in reader.sheet_names() {
        let range = reader.worksheet_range(&sheet_name)?;
        let selected = if sheet_name == "Athletes" {
            Some(column_of(&range, header)?)
        } else {
            None
        };
        let sheet = writer.add_worksheet();
        sheet.set_name(&sheet_name)?;
        for (row, cells) in range.rows().enumerate() {
            for (column, cell) in cells.iter().enumerate() {
                let r = u32::try_from(row)?;
                let c = u16::try_from(column)?;
                if row == 1 && selected == Some(column) {
                    sheet.write_string(r, c, value)?;
                    continue;
                }
                match cell {
                    Data::Empty => {}
                    Data::Float(number) => {
                        sheet.write_number(r, c, *number)?;
                    }
                    Data::Int(number) => {
                        sheet.write_number(r, c, *number as f64)?;
                    }
                    _ => {
                        sheet.write_string(r, c, cell.to_string())?;
                    }
                }
            }
        }
    }
    writer.save(target)?;
    Ok(())
}

#[test]
fn frozen_readback_rejects_a_forged_cross_country_cell() -> TestResult {
    let fixture = fixture()?;
    five_k_performance(&fixture, Sport::CrossCountry)?;
    let (published, dataset, options) = frozen_publication(&fixture)?;

    let target = fixture.dir.path().join("forged-xc.xlsx");
    forged_workbook(
        &published,
        &target,
        "XC (s)",
        "5000m 30:00.00 [xc, na, unknown]",
    )?;

    let error = match crate::workbook::verify::verify_frozen(&target, &dataset, &options) {
        Err(error) => error.to_string(),
        Ok(_) => return Err("forged cross-country cell accepted".into()),
    };
    check!(error.contains("5000m 30:00.00"), "{error}");
    Ok(())
}
