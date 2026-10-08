use super::*;

#[test]
fn team_blocks_preserve_all_published_fractional_digits_without_hiding_later_runners(
) -> Result<(), Box<dyn std::error::Error>> {
    let body = STATE.replace("15:50.2", "15:50.200000001");
    let meet = parse(&crate::hytek::lines_from_pdf_text(&body), source(), 2025)
        .ok_or("missing source meet")?;
    let event = meet.events.first().ok_or("missing source event")?;
    let runner = event
        .rows
        .iter()
        .find(|row| row.name == "Cooper Erickson")
        .ok_or("missing first runner")?;
    let Mark::TimeSeconds(time) = runner.mark else {
        return Err("first runner lost numeric mark".into());
    };
    check!(eq; (time.value(), time.precision(), time.to_string()), (950_200_000_001, 9, "950.200000001".to_string()));
    check!(eq; (meet.rows_parsed, meet.rows_skipped), (6, 0));
    let later = event
        .rows
        .iter()
        .find(|row| row.name == "Bennett Story")
        .ok_or("missing later runner")?;
    check!(eq; later.mark, Mark::TimeSeconds(ExactSeconds::parse("993.6")?));
    Ok(())
}

#[test]
fn padded_rows_preserve_nine_decimal_places_or_integer_precision(
) -> Result<(), Box<dyn std::error::Error>> {
    for (raw, nanos, precision) in [
        ("16:44.100000009", 1_004_100_000_009, 9),
        ("16:44", 1_004_000_000_000, 0),
    ] {
        let body = TABLE.replace("16:44.1", raw);
        let meet = parse(&crate::hytek::lines_from_pdf_text(&body), source(), 2025)
            .ok_or("missing source meet")?;
        let row = meet
            .events
            .first()
            .and_then(|event| event.rows.first())
            .ok_or("missing first source row")?;
        let Mark::TimeSeconds(time) = row.mark else {
            return Err("row lost numeric mark".into());
        };
        check!(eq; (time.value(), time.precision()), (nanos, precision));
        check!(eq; (meet.rows_parsed, meet.rows_skipped), (4, 0));
    }
    Ok(())
}

#[test]
fn rejected_block_time_counts_the_runner_without_hiding_later_valid_runners(
) -> Result<(), Box<dyn std::error::Error>> {
    let body = STATE.replace("15:50.2", "15:50.2000000001");
    let meet = parse(&crate::hytek::lines_from_pdf_text(&body), source(), 2025)
        .ok_or("missing source meet")?;
    let event = meet.events.first().ok_or("missing source event")?;
    check!(eq; (meet.rows_parsed, meet.rows_skipped), (5, 1));
    assert!(!event.rows.iter().any(|row| row.name == "Cooper Erickson"));
    let later = event
        .rows
        .iter()
        .find(|row| row.name == "Bennett Story")
        .ok_or("missing later runner")?;
    check!(eq; later.mark, Mark::TimeSeconds(ExactSeconds::parse("993.6")?));
    Ok(())
}

#[test]
fn published_divisions_keep_their_own_runners_and_marks() -> Result<(), Box<dyn std::error::Error>>
{
    let body = format!("{STATE}\nDivision 2\n    1. 10 Other High School (16:00.001 16:00.001 0:00.0)\n    1 1 Alex Runner 11 16:00.001\n");
    let meet =
        parse(&crate::hytek::lines_from_pdf_text(&body), source(), 2025).ok_or("missing meet")?;
    let first = meet
        .events
        .iter()
        .find(|event| event.division.as_deref() == Some("Division 1"))
        .ok_or("first division")?;
    let second = meet
        .events
        .iter()
        .find(|event| event.division.as_deref() == Some("Division 2"))
        .ok_or("second division")?;
    let runner = second
        .rows
        .iter()
        .find(|row| row.name == "Alex Runner")
        .ok_or("second division runner")?;
    check!(eq; runner.mark, Mark::TimeSeconds(ExactSeconds::parse("960.001")?));
    check!(first.rows.iter().any(|row| row.name == "Cooper Erickson"));
    check!(!first.rows.iter().any(|row| row.name == "Alex Runner"));
    check!(!second.rows.iter().any(|row| row.name == "Cooper Erickson"));
    Ok(())
}
