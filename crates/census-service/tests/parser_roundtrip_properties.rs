//! Property tests for the `wiaa_results` parse seam.
//!
//! `census_crawl::wiaa_results::parse_result_body` is the one entry point every artifact
//! (HTML release, plain-text report, RaceDay export, PDF, unknown extension) goes through. These
//! properties pin what the census is allowed to rely on:
//!
//! * **Dispatch** — `artifact_format(extension, body)` and `parse_result_body` agree: every fixture
//!   is sniffed into the format that parses it, and that format yields the expected shape (events,
//!   rows, athlete identities, marks) — see [`shapes`].
//! * **Prefix stability** — a garbage tail appended to a well-formed body never changes the part
//!   already parsed, and a truncated body parses to a prefix of the full one — see [`prefix_laws`].
//! * **Total-ness** — arbitrary bytes, empty bodies and non-report markup return `None` or a meet,
//!   never a panic, and the same bytes always give the same answer — see [`edges`].
//!
//! Deterministic by construction: [`seam_config`] pins 64 cases on ChaCha with the fixed seed
//! `0x004D_4552_475F_4944`. Every fixture below is a verbatim slice of a published WIAA release.
//!
//! `ArtifactFormat::Pdf` shells out to `pdftotext`; the properties that touch it assert only what
//! holds with and without that tool installed (a body the tool cannot read is not a meet).

#![forbid(unsafe_code)]

use census_crawl::result_file::{ParsedEvent, ParsedMeet, ParsedRow};
use census_crawl::wiaa_results::{artifact_format, parse_result_body, ArtifactFormat};
use census_domain::model::{EventKind, SourceRef};
use proptest::prelude::*;
use proptest::test_runner::{RngAlgorithm, RngSeed};

#[path = "parser_roundtrip_properties/edges.rs"]
mod edges;
#[path = "parser_roundtrip_properties/prefix_laws.rs"]
mod prefix_laws;
#[path = "parser_roundtrip_properties/shapes.rs"]
mod shapes;


const DASH_HTML: &str =
    include_str!("../../census-crawl/tests/fixtures/wiaa_results/d1boysstateresults-dash.htm");
const DASH_TEXT: &str =
    include_str!("../../census-crawl/tests/fixtures/wiaa_results/d1boysstateresults-dash.txt");
const SECTIONS_HTML: &str =
    include_str!("../../census-crawl/tests/fixtures/wiaa_results/d1boysstateresults-sections.htm");
const SEED_HTML: &str =
    include_str!("../../census-crawl/tests/fixtures/wiaa_results/seed-column-regional.htm");
const TRACKSIDE_HTML: &str =
    include_str!("../../census-crawl/tests/fixtures/wiaa_results/trackside-regional.htm");
const RACEDAY_HTML: &str =
    include_str!("../../census-crawl/tests/fixtures/wiaa_results/racinesectionalb-finish-list.htm");
/// A page from the same archive that is not a result file: link markup, no meet.
const ARCHIVE_HTML: &str = "<html><body><h3>2025 Track &amp; Field State Results</h3>\
     <ul><li>Division 1 - <a href=\"/Portals/0/PDF/Results/Track/2025/d1boysstateresults.htm\">Boys</a>\
     </li></ul></body></html>";

const ARCHIVE_YEAR: i16 = 2025;
const FORMATS: [ArtifactFormat; 5] = [
    ArtifactFormat::HytekHtml,
    ArtifactFormat::HytekText,
    ArtifactFormat::RaceDay,
    ArtifactFormat::Pdf,
    ArtifactFormat::Unparsed,
];

/// `(fixture name, body, the format the archive publishes it as)`.
const FIXTURES: [(&str, &str, ArtifactFormat); 6] = [
    (
        "d1boysstateresults-dash.htm",
        DASH_HTML,
        ArtifactFormat::HytekHtml,
    ),
    (
        "d1boysstateresults-dash.txt",
        DASH_TEXT,
        ArtifactFormat::HytekText,
    ),
    (
        "d1boysstateresults-sections.htm",
        SECTIONS_HTML,
        ArtifactFormat::HytekHtml,
    ),
    (
        "seed-column-regional.htm",
        SEED_HTML,
        ArtifactFormat::HytekHtml,
    ),
    (
        "trackside-regional.htm",
        TRACKSIDE_HTML,
        ArtifactFormat::HytekHtml,
    ),
    (
        "racinesectionalb-finish-list.htm",
        RACEDAY_HTML,
        ArtifactFormat::RaceDay,
    ),
];


fn source() -> SourceRef {
    SourceRef::new("wiaa_results", None)
}

fn parse(body: &str, format: ArtifactFormat, year: i16) -> Option<ParsedMeet> {
    parse_result_body(body.as_bytes(), format, source(), year)
}

fn seam_config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x004D_4552_475F_4944),
        ..ProptestConfig::default()
    }
}

fn extension(fixture: &str) -> &str {
    fixture.rsplit('.').next().unwrap_or_default()
}

/// Hostile-but-plausible tail text. `<` and `>` are excluded so a tail can never open a tag: the
/// `<pre>`-block rule is pinned by its own test, not by luck.
fn tail(max_len: usize) -> impl Strategy<Value = String> {
    const CHARS: &[u8] = b"abcxyz0123456789 .-!#@()/:;'\"&=+*[]{}|~\n\t";
    prop::collection::vec(prop::sample::select(CHARS), 1..=max_len)
        .prop_map(|bytes| bytes.into_iter().map(char::from).collect())
}

fn tail_blocks() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(tail(24), 1..=3)
}

/// The parse of a shorter body must survive as a prefix of the parse of a longer one: the meet
/// identity comes from the header, rows and events are only ever appended, and a later line can
/// add nothing to a section that is already closed.
///
/// Two things a longer body legitimately moves, and this predicate therefore ignores:
/// `round`, which a later marker line re-labels on the still-open section, and `legs`, which a
/// following `1) Name 11` line appends to the relay row the open section ends with.
fn prefix_survives(shorter: &ParsedMeet, longer: &ParsedMeet) -> Result<(), TestCaseError> {
    if (
        shorter.name.as_str(),
        shorter.date.as_str(),
        shorter.timer.as_ref(),
    ) != (
        longer.name.as_str(),
        longer.date.as_str(),
        longer.timer.as_ref(),
    ) {
        return Err(TestCaseError::fail(format!(
            "identity moved: {:?} became {:?}",
            (
                shorter.name.as_str(),
                shorter.date.as_str(),
                shorter.timer.as_ref()
            ),
            (
                longer.name.as_str(),
                longer.date.as_str(),
                longer.timer.as_ref()
            )
        )));
    }
    if longer.events.len() < shorter.events.len() || longer.rows_parsed < shorter.rows_parsed {
        return Err(TestCaseError::fail(format!(
            "counts shrank: {} events/{} rows became {} events/{} rows",
            shorter.events.len(),
            shorter.rows_parsed,
            longer.events.len(),
            longer.rows_parsed
        )));
    }
    let open = shorter.events.len().saturating_sub(1);
    for (index, event) in shorter.events.iter().enumerate() {
        let extended = &longer.events[index];
        if !same_event(event, extended) {
            return Err(TestCaseError::fail(format!(
                "event {index} changed: {event:?} became {extended:?}"
            )));
        }
        let rows = event.rows.len();
        if extended.rows.len() < rows {
            return Err(TestCaseError::fail(format!(
                "event {index} lost parsed rows: {} became {}",
                rows,
                extended.rows.len()
            )));
        }
        let frozen = if index == open {
            rows.saturating_sub(1)
        } else {
            rows
        };
        if extended.rows[..frozen] != event.rows[..frozen] {
            return Err(TestCaseError::fail(format!(
                "event {index} rewrote parsed rows: {:?} became {:?}",
                event.rows, extended.rows
            )));
        }
        if index == open && rows > 0 && !legs_grew(&event.rows[rows - 1], &extended.rows[rows - 1])
        {
            return Err(TestCaseError::fail(format!(
                "event {index} rewrote its open relay row: {:?} became {:?}",
                event.rows[rows - 1],
                extended.rows[rows - 1]
            )));
        }
    }
    Ok(())
}

/// Everything about an event that a later line cannot move: `round` is deliberately absent.
fn same_event(shorter: &ParsedEvent, longer: &ParsedEvent) -> bool {
    shorter.label == longer.label
        && shorter.kind == longer.kind
        && shorter.gender == longer.gender
        && shorter.division == longer.division
}

/// The row a still-open section ends with: every published column is fixed, and only relay legs
/// may have been appended.
fn legs_grew(shorter: &ParsedRow, longer: &ParsedRow) -> bool {
    longer.place == shorter.place
        && longer.name == shorter.name
        && longer.grade == shorter.grade
        && longer.school == shorter.school
        && longer.mark == shorter.mark
        && longer.wind_mps == shorter.wind_mps
        && longer.heat == shorter.heat
        && longer.points == shorter.points
        && longer.legs.starts_with(&shorter.legs)
}


#[test]
fn every_fixture_dispatches_to_the_format_that_parses_it() {
    for (name, body, format) in FIXTURES {
        assert_eq!(
            artifact_format(extension(name), Some(body)),
            format,
            "{name} is sniffed into the format the archive publishes it as"
        );
        let meet = parse(body, format, ARCHIVE_YEAR)
            .unwrap_or_else(|| panic!("{name} parses as {format:?}"));
        assert!(!meet.name.is_empty(), "{name} publishes a meet name");
        assert!(!meet.date.is_empty(), "{name} publishes a date");
        assert!(!meet.events.is_empty(), "{name} publishes events");
        let rows: usize = meet.events.iter().map(|event| event.rows.len()).sum();
        assert!(rows > 0, "{name} publishes result rows");
        assert_eq!(meet.rows_parsed, rows, "{name} counts the rows it parsed");
        for event in &meet.events {
            for row in &event.rows {
                assert!(
                    !row.school.is_empty(),
                    "{name}: a result row names a school: {row:?}"
                );
                assert!(
                    row.place.is_none() || !row.name.is_empty() || !row.legs.is_empty(),
                    "{name}: a placed row names an athlete or lists relay legs: {row:?}"
                );
            }
        }
    }
}

#[test]
fn the_two_hytek_front_ends_agree_on_the_same_report() {
    let from_html = parse(DASH_HTML, ArtifactFormat::HytekHtml, ARCHIVE_YEAR);
    let from_text = parse(DASH_TEXT, ArtifactFormat::HytekText, ARCHIVE_YEAR);
    assert_eq!(
        from_html.expect("the HTML release parses"),
        from_text.expect("the plain-text release parses"),
        "one report, two releases: the parsed meets must be identical"
    );
}

/// The prefix predicate is what the tail and truncation properties assert through, so it has to
/// reject an edited parse rather than accept everything.
#[test]
fn the_prefix_predicate_rejects_an_edited_parse() {
    let full =
        parse(SECTIONS_HTML, ArtifactFormat::HytekHtml, ARCHIVE_YEAR).expect("sections parses");

    let mut renamed = full.clone();
    renamed.name = "Some Other Meet".to_string();
    assert!(prefix_survives(&full, &renamed).is_err(), "identity moved");

    let mut relabelled = full.clone();
    relabelled.events[1].kind = EventKind::Track200m;
    assert!(
        prefix_survives(&full, &relabelled).is_err(),
        "an event's ontology changed"
    );

    let mut edited = full.clone();
    edited.events[2].rows[0].school = "Elsewhere".to_string();
    assert!(
        prefix_survives(&full, &edited).is_err(),
        "a parsed row was rewritten"
    );
    assert!(
        prefix_survives(&full, &full).is_ok(),
        "the predicate accepts an unchanged parse"
    );
}
