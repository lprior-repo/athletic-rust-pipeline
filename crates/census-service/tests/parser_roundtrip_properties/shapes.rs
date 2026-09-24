//! What a well-formed fixture must parse into: identities and marks, front-end agreement.

use super::*;
use census_domain::model::{CentiSeconds, EventKind, Gender, Grade, Mark};

#[test]
fn known_identities_and_marks_survive_the_seam() {
    let dash = parse(DASH_HTML, ArtifactFormat::HytekHtml, ARCHIVE_YEAR).expect("dash parses");
    let prelims = &dash.events[0];
    assert_eq!(prelims.kind, EventKind::Track100m);
    assert_eq!(prelims.gender, Gender::Boys);
    assert_eq!(prelims.division.as_deref(), Some("Division 1"));
    let winner = &prelims.rows[0];
    assert_eq!(winner.name, "Ben Lemirand");
    assert_eq!(winner.grade.map(Grade::get), Some(12));
    assert_eq!(winner.school, "West De Pere");
    assert_eq!(
        winner.mark,
        Mark::TimeSeconds(CentiSeconds::from_seconds_f64(10.56))
    );
    assert_eq!(winner.wind_mps, Some(0.4));

    let sections =
        parse(SECTIONS_HTML, ArtifactFormat::HytekHtml, ARCHIVE_YEAR).expect("sections parses");
    let shot = sections
        .events
        .iter()
        .find(|event| event.kind == EventKind::ShotPut)
        .expect("the sections fixture publishes the shot put");
    assert_eq!(shot.rows[0].name, "Hunter Sprangers");
    match &shot.rows[0].mark {
        Mark::FieldImperial { feet_mark, metres } => {
            assert_eq!(
                feet_mark, "61-03.50",
                "a field mark keeps the published notation through the seam"
            );
            assert!(
                (metres.as_metres_f64() - 18.68).abs() < 0.01,
                "feet convert once: {metres}"
            );
        }
        other => panic!("expected an imperial field mark, got {other:?}"),
    }

    let raceday = parse(RACEDAY_HTML, ArtifactFormat::RaceDay, 2023).expect("raceday parses");
    assert_eq!(raceday.events[0].kind, EventKind::CrossCountry);
    let finisher = &raceday.events[0].rows[0];
    assert_eq!(finisher.name, "Jack Hefty");
    assert_eq!(finisher.school, "Whitewater");
    assert_eq!(
        finisher.mark,
        Mark::TimeSeconds(CentiSeconds::from_seconds_f64(1033.69))
    );
}

#[test]
fn a_body_carrying_a_pre_block_is_read_from_that_block_alone() {
    // The front end reads either one `<pre>` report or the body's paragraphs, never both, so a
    // `<pre>` block appended to a paragraph report takes the body over.
    let mixed = format!("{SECTIONS_HTML}<pre>not the report</pre>");
    assert_eq!(parse(&mixed, ArtifactFormat::HytekHtml, ARCHIVE_YEAR), None);

    // The same report published as a `<pre>` release parses to the same meet as the text release.
    let pre_body = format!(
        "<html><body><pre>{}</pre></body></html>",
        DASH_TEXT.replace('\n', "<br>")
    );
    assert_eq!(
        parse(&pre_body, ArtifactFormat::HytekHtml, ARCHIVE_YEAR)
            .expect("a <pre> release of the dash report parses"),
        parse(DASH_TEXT, ArtifactFormat::HytekText, ARCHIVE_YEAR).expect("the text release parses")
    );
}
