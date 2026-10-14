use super::*;

fn multi_window_input() -> Result<(RosterOutcome, Vec<SourceRowLocator>), Box<dyn std::error::Error>>
{
    let mut body = format!(
        "é🏃<link rel=\"canonical\" href=\"{}/roster\"><ul id=\"rosterDataset\">",
        team().url,
    );
    let mut expected = Vec::new();
    for ordinal in 0u32..67 {
        let id = ordinal.checked_add(1).ok_or("fixture ID overflow")?;
        let year = match ordinal {
            31 => "invalid",
            value if value % 3 == 0 => "2027",
            _ => "2028",
        };
        let outer = row(
            &id.to_string(),
            &format!("Runner, Person{ordinal:02}"),
            year,
        )
        .replace(
            "<div data-season-id=\"2\">",
            "<div class=\"data-point\" data-season-id=\"1\">",
        )
        .replace("class=\"icon-yes\"", "class=\"icon icon-yes\"");
        let open = "<li class=\"athlete-row data-row\">";
        let inner = outer
            .strip_prefix(open)
            .and_then(|value| value.strip_suffix("</li>"))
            .ok_or("fixture row delimiters")?;
        if ordinal != 31 {
            expected.push(SourceRowLocator {
                ordinal,
                byte_offset: body.len().checked_add(open.len()).ok_or("fixture offset")?,
                byte_length: inner.len(),
            });
        }
        body.push_str(&outer);
    }
    body.push_str("</ul>");
    Ok((capture(body)?, expected))
}

#[test]
fn positive_roster_windows_and_records_borrow_the_same_global_members() -> TestResult {
    let (read, expected) = multi_window_input()?;
    let run = run()?;
    let roster = read.verdict.roster().ok_or("missing partial roster")?;
    check!(eq; roster.athletes.len(), 66);
    check!(eq; read.verdict.rejections().first().ok_or("missing rejection")?.row.ordinal, 31);
    let body = std::str::from_utf8(&read.capture.body)?;
    let mut counts = Counts::new()?;
    let mut next = 0usize;
    for (offset, length, first_ordinal) in [(0usize, 64usize, 0u32), (64, 2, 65)] {
        check!(eq; next, offset);
        let end = offset.checked_add(length).ok_or("window end overflow")?;
        let original = roster
            .athletes
            .get(offset..end)
            .ok_or("missing original slice")?;
        let records = {
            let window = Window::next(roster, offset, run.observed_on)?.ok_or("missing window")?;
            check!(std::ptr::eq(window.athletes(), original));
            let records = Records::of(&window, &run.site, run.school_year)?;
            check!(std::ptr::eq(records.members(), window.athletes()));
            records
        };
        check!(std::ptr::eq(records.members(), original));
        check!(eq; records.members().first().ok_or("empty members")?.locator().ordinal, first_ordinal);
        for (member, locator) in records
            .members()
            .iter()
            .zip(expected.get(offset..end).ok_or("expected slice")?)
        {
            check!(eq; member.locator(), *locator);
            let id = locator
                .ordinal
                .checked_add(1)
                .ok_or("expected provider ID overflow")?;
            check!(eq; member.athlete().athlete_id, id.to_string());
            let end = locator
                .byte_offset
                .checked_add(locator.byte_length)
                .ok_or("locator end")?;
            let span = body
                .get(locator.byte_offset..end)
                .ok_or("invalid UTF-8 span")?;
            check!(span.contains(&format!("/athletes/{id}-runner")));
        }
        counts.include(records.members(), records.teams())?;
        next = next
            .checked_add(records.members().len())
            .ok_or("cursor overflow")?;
    }
    check!(eq; next, roster.athletes.len());
    check!(Window::next(roster, next, run.observed_on)?.is_none());
    check!(eq; (counts.athletes, counts.co2027, counts.boys, counts.girls, counts.teams()), (66, 23, 23, 0, 1));
    Ok(())
}

#[tokio::test]
async fn actual_roster_apply_keeps_partial_window_counts_and_legacy_replay() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let (read, _) = multi_window_input()?;
    let run = run()?;
    apply(&store, &team(), &run, &read).await?;
    apply(&store, &team(), &run, &read).await?;
    let journal = prior(&store, &team(), &run)?.ok_or("missing partial journal")?;
    check!(eq; (journal.athletes, journal.co2027, journal.co2027_boys, journal.co2027_girls, journal.teams), (66, 23, 23, 0, 1));
    check!(eq; journal.disposition, census_crawl::CollectionDisposition::Partial);
    check!(eq; journal.rejected.len(), 1);
    check!(eq; journal.rejected.first().ok_or("missing rejection")?.row.ordinal, 31);
    check!(!journal.is_terminal());
    check!(eq; athletes(&store)?.len(), 66);
    Ok(())
}
