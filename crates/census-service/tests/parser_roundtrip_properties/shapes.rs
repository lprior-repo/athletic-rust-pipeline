use super::*;
use census_domain::model::{CentiSeconds, EventKind, Gender, Grade, Mark};

#[test]
fn known_identities_and_marks_survive_the_seam() -> Result<(), Box<dyn std::error::Error>> {
    let dash =
        parse(DASH_HTML, ArtifactFormat::HytekHtml, ARCHIVE_YEAR).ok_or("dash did not parse")?;
    let prelims = &dash.events[0];
    check!(eq; prelims.kind, EventKind::Track100m);
    check!(eq; prelims.gender, Gender::Boys);
    check!(eq; prelims.division.as_deref(), Some("Division 1"));
    let winner = &prelims.rows[0];
    check!(eq; winner.name, "Ben Lemirand");
    check!(eq; winner.grade.map(Grade::get), Some(12));
    check!(eq; winner.school, "West De Pere");
    check!(eq; winner.mark,
    Mark::TimeSeconds(CentiSeconds::try_from_seconds_f64(10.56).ok_or("invalid expected dash time")?));
    check!(eq; winner.wind_mps, Some(0.4));

    let sections = parse(SECTIONS_HTML, ArtifactFormat::HytekHtml, ARCHIVE_YEAR)
        .ok_or("sections did not parse")?;
    let shot = sections
        .events
        .iter()
        .find(|event| event.kind == EventKind::ShotPut)
        .ok_or("missing shot put")?;
    check!(eq; shot.rows[0].name, "Hunter Sprangers");
    match &shot.rows[0].mark {
        Mark::FieldImperial { feet_mark, metres } => {
            check!(eq; feet_mark, "61-03.50",
            "a field mark keeps the published notation through the seam");
            check!(
                (metres.as_metres_f64() - 18.68).abs() < 0.01,
                "feet convert once: {metres}"
            );
        }
        other => return Err(format!("expected an imperial field mark, got {other:?}").into()),
    }

    let raceday =
        parse(RACEDAY_HTML, ArtifactFormat::RaceDay, 2023).ok_or("raceday did not parse")?;
    check!(eq; raceday.events[0].kind, EventKind::CrossCountry);
    let finisher = &raceday.events[0].rows[0];
    check!(eq; finisher.name, "Jack Hefty");
    check!(eq; finisher.school, "Whitewater");
    check!(eq; finisher.mark,
    Mark::TimeSeconds(
        CentiSeconds::try_from_seconds_f64(1033.69).ok_or("invalid expected cross-country time")?
    ));
    Ok(())
}

#[test]
fn a_body_carrying_a_pre_block_is_read_from_that_block_alone(
) -> Result<(), Box<dyn std::error::Error>> {
    let mixed = format!("{SECTIONS_HTML}<pre>not the report</pre>");
    check!(eq; parse(&mixed, ArtifactFormat::HytekHtml, ARCHIVE_YEAR), None);

    let pre_body = format!(
        "<html><body><pre>{}</pre></body></html>",
        DASH_TEXT.replace('\n', "<br>")
    );
    check!(eq; parse(&pre_body, ArtifactFormat::HytekHtml, ARCHIVE_YEAR)
        .ok_or("pre release did not parse")?,
    parse(DASH_TEXT, ArtifactFormat::HytekText, ARCHIVE_YEAR).ok_or("text release did not parse")?);
    Ok(())
}
