//! What the captured result documents measure, and what the run does with them.
//!
//! The fixtures are the repository's own captures. `event-doc-2150205.json` is the 2A girls 5000 m
//! race of the 2025 Iowa state cross-country final: 136 rows, 41 school labels, `JR/SR/SO/FR`
//! grades, three splits on every row, `xc: true`. `event-doc-2254280.json` is the girls high jump of
//! MITS 4: 17 rows, club labels that name no school, four `NH` rows, one empty grade cell.
//!
//! The live-standings capture the wire document names is 234 KB and outside this lane's fixture
//! budget, so the standings tests write a payload to the shape `standings.rs` documents. What they
//! prove is the pairing and the mark rules, not the capture.

use super::{collect, ResultOptions, StandingsCapture};
use crate::net::Fetcher;
use crate::sources::athleticlive::docs::{parse_event_document, parse_event_summary, EventDoc};
use crate::sources::athleticlive::map::SOURCE_ID;
use crate::sources::athleticlive::wire::{event_doc_url, event_summary_url};
use crate::sources::athleticlive_athletes::{school_year_for_date, MeetTarget};
use crate::sources::{AdapterContext, AdapterReport};
use crate::store::{Store, Table};
use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, EventKind, GradYear, Grade, Mark, SchoolYear, SourceNamespace,
};
use census_domain::UsJurisdiction;

const XC_STATE: &str =
    include_str!("../../../../tests/fixtures/athleticlive_results/event-doc-2150205.json");
const HJ_MITS: &str =
    include_str!("../../../../tests/fixtures/athleticlive_results/event-doc-2254280.json");
const OBSERVED_ON: &str = "2026-09-22";
/// The two meets the harvest CSV publishes for the fixtures' captures.
const STATE_MEET: u64 = 58_504;
const MITS_MEET: u64 = 61_710;

/// The meet target for `(id, tenant, name, state, date)`, with the canonical id the harvest route
/// mints for the same row, so the test can prove the results route lands on that id.
fn target(id: u64, tenant: &str, name: &str, state: UsJurisdiction, date: &str) -> MeetTarget {
    let meet = CanonicalMeet::new(
        Some(state),
        name,
        date,
        crate::sources::athleticlive::infer_level(name),
    );
    MeetTarget {
        athleticlive_meet_id: id,
        meet_id: meet.id.as_str().to_string(),
        tenant: tenant.to_string(),
        name: name.to_string(),
        state,
        date: date.to_string(),
    }
}

/// The 2025 Iowa state cross-country final, as the harvest CSV publishes it.
fn state_meet() -> MeetTarget {
    target(
        STATE_MEET,
        "live_results",
        "Iowa High School State Championships",
        UsJurisdiction::Iowa,
        "2025-10-31",
    )
}

/// MITS 4, as the harvest CSV publishes it.
fn mits_meet() -> MeetTarget {
    target(
        MITS_MEET,
        "live_results",
        "MITS 4",
        UsJurisdiction::Michigan,
        "2026-02-14",
    )
}

/// A store and fetcher pair over a fresh directory.
fn scratch() -> (tempfile::TempDir, Store, Fetcher) {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = Store::open(dir.path().join("store")).expect("store");
    let fetcher = Fetcher::new(
        dir.path().join("http"),
        None,
        std::time::Duration::from_millis(1),
        std::collections::HashMap::new(),
        Vec::new(),
    )
    .expect("fetcher");
    (dir, store, fetcher)
}

fn context<'a>(store: &'a Store, fetcher: &'a Fetcher) -> AdapterContext<'a> {
    AdapterContext {
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear(2026),
        observed_on: OBSERVED_ON.to_string(),
    }
}

/// Write a capture where the options can name it, and return its path.
fn stage_capture(dir: &tempfile::TempDir, name: &str, body: &str) -> String {
    let path = dir.path().join(name);
    std::fs::write(&path, body).expect("capture staged");
    path.to_string_lossy().into_owned()
}

/// Write the consolidated schools a run resolves labels against, where the adapter reads them.
fn write_schools(store: &Store, schools: &[(UsJurisdiction, &str)]) {
    let mut lines = String::new();
    for (state, name) in schools {
        let (school, _) = CanonicalSchool::new(*state, *name, normalize_name(name));
        lines.push_str(&serde_json::to_string(&school).expect("school serializes"));
        lines.push('\n');
    }
    std::fs::create_dir_all(store.out_dir()).expect("out dir");
    std::fs::write(store.out_dir().join("schools.jsonl"), lines).expect("schools written");
}

/// The distinct school labels a capture publishes, as schools of `state`.
///
/// The consolidated index is where a school's existence is decided, so a mapping test has to give
/// the run the schools the capture's own labels name; the labels here are Iowa and Michigan schools.
fn labelled_schools(doc: &EventDoc, state: UsJurisdiction) -> Vec<(UsJurisdiction, String)> {
    let mut labels: Vec<String> = doc
        .rows
        .iter()
        .filter_map(|row| row.athlete.as_ref())
        .filter_map(|athlete| athlete.team.as_ref())
        .filter_map(|team| team.school_name().map(str::to_string))
        .collect();
    labels.sort();
    labels.dedup();
    labels.into_iter().map(|label| (state, label)).collect()
}

fn labels_of(schools: &[(UsJurisdiction, String)]) -> Vec<(UsJurisdiction, &str)> {
    schools
        .iter()
        .map(|(state, label)| (*state, label.as_str()))
        .collect()
}

fn joined(report: &AdapterReport) -> String {
    report.notes.join("\n")
}

#[test]
fn event_document_rows_carry_the_published_shapes() {
    let xc = parse_event_document(&event_doc_url(2_150_205), XC_STATE)
        .expect("the state-final document parses");
    assert_eq!(xc.event_id(), Some(2_150_205));
    assert_eq!(xc.meet_id(), Some(STATE_MEET));
    assert_eq!(xc.rows.len(), 136);
    // `xc: true` outranks the published label, and the label is still read as published.
    assert_eq!(xc.label(), Some("Run"));
    assert_eq!(xc.kind(), EventKind::CrossCountry);
    assert_eq!(xc.gender_group.as_deref(), Some("Girls"));
    assert_eq!(xc.run_id(), Some("1-1"));
    assert_eq!(xc.division_name(), None, "`dv` is null on this capture");
    assert_eq!(xc.rows[0].place(), Some(1));
    assert_eq!(xc.rows[0].mark.as_deref(), Some("18:20.7"));
    assert_eq!(
        xc.rows[0].canonical_mark(&EventKind::CrossCountry),
        Some(Mark::TimeSeconds(1100.7))
    );
    assert_eq!(xc.rows[0].splits.len(), 3, "the cross-country split list");

    let hj = parse_event_document(&event_doc_url(2_254_280), HJ_MITS)
        .expect("the high-jump document parses");
    assert_eq!(hj.event_id(), Some(2_254_280));
    assert_eq!(hj.meet_id(), Some(MITS_MEET));
    assert_eq!(hj.rows.len(), 17);
    assert_eq!(hj.label(), Some("HJ"));
    assert_eq!(hj.kind(), EventKind::HighJump);
    assert_eq!(hj.division_name(), Some("MITS"));
    assert_eq!(hj.run_id(), Some("19-1"));
    assert_eq!(
        hj.rows[0].splits.len(),
        0,
        "a field event publishes no splits"
    );
    // A field mark keeps the published notation, and its metres are the integer channel's.
    assert_eq!(
        hj.rows[0].canonical_mark(&EventKind::HighJump),
        Some(Mark::FieldImperial {
            feet_mark: "5-02.00".to_string(),
            metres: (5.0 * 12.0 + 2.0) * 0.0254,
        })
    );
    // `NH` publishes `im: 0`: the athlete competed, and there is no mark to mint.
    let no_height = &hj.rows[13];
    assert_eq!(no_height.mark.as_deref(), Some("NH"));
    assert_eq!(no_height.canonical_mark(&EventKind::HighJump), None);
    assert_eq!(no_height.place(), None, "an unplaced row publishes `--`");
    // The grade cell is published empty on one row: an empty token is no grade, not grade zero.
    let blank_grade = &hj.rows[10];
    assert_eq!(blank_grade.mark.as_deref(), Some("4-06.00"));
    assert_eq!(
        blank_grade
            .athlete
            .as_ref()
            .and_then(|athlete| athlete.grade.as_ref())
            .and_then(|grade| grade.as_str()),
        Some("")
    );
}

#[test]
fn both_mark_channels_agree_on_every_captured_row() {
    let xc = parse_event_document(&event_doc_url(2_150_205), XC_STATE).expect("parses");
    let mut time_rows = 0usize;
    for row in &xc.rows {
        let published = row.mark.as_deref().expect("every row publishes a mark");
        let seconds = crate::sources::hytek::parse_time(published).expect("the time parses");
        let Some(Mark::TimeSeconds(minted)) = row.canonical_mark(&EventKind::CrossCountry) else {
            panic!("row {:?} mints no time mark", row.place());
        };
        assert!(
            (minted - seconds).abs() < 1e-9,
            "{published} parsed {seconds} but the integer channel minted {minted}"
        );
        time_rows += 1;
    }
    assert_eq!(time_rows, 136);

    let hj = parse_event_document(&event_doc_url(2_254_280), HJ_MITS).expect("parses");
    let mut field_rows = 0usize;
    for row in &hj.rows {
        let Some(Mark::FieldImperial { metres, .. }) = row.canonical_mark(&EventKind::HighJump)
        else {
            assert_eq!(row.mark.as_deref(), Some("NH"), "only `NH` mints no mark");
            continue;
        };
        let micros = row
            .mark_int
            .as_ref()
            .and_then(|value| value.as_f64())
            .expect("im publishes");
        assert!(
            (metres - micros / 1_000_000.0).abs() < 1e-9,
            "{} published {metres} m against {micros} µm",
            row.mark.as_deref().unwrap_or_default()
        );
        field_rows += 1;
    }
    assert_eq!(field_rows, 13, "four of the seventeen rows are `NH`");
}

#[tokio::test]
async fn collect_maps_a_captured_state_final_into_the_canonical_tables() {
    let (dir, store, fetcher) = scratch();
    let doc = parse_event_document(&event_doc_url(2_150_205), XC_STATE).expect("parses");
    write_schools(
        &store,
        &labels_of(&labelled_schools(&doc, UsJurisdiction::Iowa)),
    );
    let path = stage_capture(&dir, "event-doc-2150205.json", XC_STATE);
    let options = ResultOptions {
        documents: vec![path.clone()],
        ..ResultOptions::for_meet(state_meet(), OBSERVED_ON)
    };

    let report = collect(&context(&store, &fetcher), &options)
        .await
        .expect("the run completes");
    assert_eq!(report.adapter, SOURCE_ID);
    assert_eq!(report.rows, 136);
    assert_eq!(
        report.errors,
        0,
        "every row of this capture maps: {}",
        joined(&report)
    );
    let notes = joined(&report);
    for expected in [
        "rows read: 136 (mapped 136, skipped 0)",
        "performances: 136 written, 0 rows published no mark",
        "identity channels: athletic.net athlete ids 132, athleticlive team ids 136, \
         athletic.net team ids 136, legacy ids 0, short team keys 0",
        "splits: 136 rows carrying 408 splits; seeds 0, wind 0, heats 0, unplaced 0",
        "documents: 1 event documents, 0 standings; events listed 0 (relay 0, unmapped 0, unfetched 0)",
        "canonical entities: meets 1 events 1 teams 41 athletes 136 performances 136",
    ] {
        assert!(notes.contains(expected), "missing `{expected}` in:\n{notes}");
    }
    assert_eq!(report.requests, 0, "the adapter fetches nothing");

    let performances: Vec<CanonicalPerformance> =
        store.scan(Table::Performances).expect("performances read");
    assert_eq!(performances.len(), 136);
    let winner = performances
        .iter()
        .find(|row| row.place == Some(1))
        .expect("one row is placed first");
    assert_eq!(winner.mark, Mark::TimeSeconds(1100.7));
    assert!(
        winner.source_key.starts_with("athleticlive:2150205:"),
        "the performance key is the event's: {}",
        winner.source_key
    );
    assert_eq!(
        winner
            .evidence
            .iter()
            .map(|row| row.source.url.clone())
            .collect::<Vec<_>>(),
        vec![Some(event_doc_url(2_150_205))]
    );
    assert_eq!(winner.observed_grade.map(Grade::get), Some(12));

    let meets: Vec<CanonicalMeet> = store.scan(Table::Meets).expect("meets read");
    assert_eq!(meets.len(), 1);
    assert_eq!(
        meets[0].id.as_str(),
        state_meet().meet_id,
        "the harvest route's meet id"
    );
    assert_eq!(meets[0].date, "2025-10-31");
    assert_eq!(meets[0].state, Some(UsJurisdiction::Iowa));
    assert!(meets[0].source_identities.iter().any(|identity| {
        identity.id == STATE_MEET.to_string()
            && identity.namespace
                == SourceNamespace::TimerMeet {
                    provider: "live_results".to_string(),
                }
    }));

    let events: Vec<CanonicalEvent> = store.scan(Table::Events).expect("events read");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::CrossCountry);
    assert_eq!(
        events[0].round.as_deref(),
        Some("finals"),
        "`Finals` maps to the round marker"
    );
    assert_eq!(events[0].division, None);
    assert!(events[0]
        .source_labels
        .iter()
        .any(|label| label.label == "Run"));

    let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes).expect("athletes read");
    assert_eq!(athletes.len(), 136);
    // The meet is a fall one, so the school year is 2025-26: a senior is the class of 2026.
    assert_eq!(
        school_year_for_date(&meets[0].date, SchoolYear(2026)),
        SchoolYear::containing(2025, 10)
    );
    assert!(athletes
        .iter()
        .any(|athlete| athlete.grad_year == GradYear(2026)));
    assert!(athletes
        .iter()
        .any(|athlete| athlete.grad_year == GradYear(2029)));
}

#[tokio::test]
async fn club_labels_and_an_empty_grade_cell_are_refused_not_invented() {
    let (dir, store, fetcher) = scratch();
    // The index holds one genuine Michigan school; not one of the capture's club labels names it.
    write_schools(
        &store,
        &[(UsJurisdiction::Michigan, "East Kentwood High School")],
    );
    let path = stage_capture(&dir, "event-doc-2254280.json", HJ_MITS);
    let options = ResultOptions {
        documents: vec![path],
        ..ResultOptions::for_meet(mits_meet(), OBSERVED_ON)
    };

    let report = collect(&context(&store, &fetcher), &options)
        .await
        .expect("the run completes");
    assert_eq!(report.rows, 0, "no row names a consolidated school");
    let notes = joined(&report);
    for expected in [
        "rows read: 17 (mapped 0, skipped 17)",
        "skipped: no name 0, no school label 0, unresolved school 16, below high school 0, no grade 1",
        "performances: 0 written, 0 rows published no mark",
        "documents: 1 event documents, 0 standings; events listed 0 (relay 0, unmapped 0, unfetched 0)",
        "unresolved school labels (up to 10): Cardinal Track Club x1, Donie Track Club x1, GR FIRE x1, \
         Jackson Crushers x1, Kentwood Track Club x1, Tiger Track x1, Unattached x8, sturgis Track Club x2",
        "canonical entities: meets 1 events 1 teams 0 athletes 0 performances 0",
    ] {
        assert!(notes.contains(expected), "missing `{expected}` in:\n{notes}");
    }

    // The meet and its event are still minted: the capture placed the race even though it placed no
    // school.
    let meets: Vec<CanonicalMeet> = store.scan(Table::Meets).expect("meets read");
    assert_eq!(meets.len(), 1);
    assert_eq!(meets[0].id.as_str(), mits_meet().meet_id);
    assert_eq!(meets[0].date, "2026-02-14");
    assert_eq!(meets[0].state, Some(UsJurisdiction::Michigan));
    let events: Vec<CanonicalEvent> = store.scan(Table::Events).expect("events read");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::HighJump);
    assert_eq!(events[0].division.as_deref(), Some("MITS"));
    assert!(store
        .scan::<CanonicalAthlete>(Table::Athletes)
        .expect("athletes read")
        .is_empty());
}

/// A standings payload in the shape `standings.rs` documents, for one row of the state final.
///
/// `rtm` is the raw timing channel the reader prefers for a time, `m` the rounded display column,
/// `anli` the second Athletic.net-derived id channel (counted, never minted) and `sp` the split
/// copies, `split_final` among them.
fn standings_payload(name: &str, grade: &str, team: &str) -> String {
    serde_json::json!({
        "zx91": {
            "i": "1501",
            "cm": "1501",
            "n": name,
            "g": "F",
            "y": grade,
            "tn": team,
            "ti": "2eW0TN",
            "p": "26",
            "rtm": "20:25.700",
            "m": "20:25.7",
            "anli": 42660317,
            "sp": {
                "0": {"sp": "6:30.0", "cs": "6:30.0"},
                "1": {"sp": "13:20.0", "cs": "13:20.0"},
                "split_final": {"sp": "20:25.7"}
            }
        }
    })
    .to_string()
}

#[tokio::test]
async fn a_standings_capture_folds_into_the_event_that_published_its_run_key() {
    let (dir, store, fetcher) = scratch();
    write_schools(&store, &[(UsJurisdiction::Iowa, "Waukon")]);
    let doc_path = stage_capture(&dir, "event-doc-2150205.json", XC_STATE);
    let standings_path = stage_capture(
        &dir,
        "live-run-standings-1-1.json",
        &standings_payload("Miriam Downing", "SO", "Waukon"),
    );
    let options = ResultOptions {
        documents: vec![doc_path.clone()],
        standings: vec![StandingsCapture {
            run_id: "1-1".to_string(),
            path: standings_path,
        }],
        ..ResultOptions::for_meet(state_meet(), OBSERVED_ON)
    };

    let report = collect(&context(&store, &fetcher), &options)
        .await
        .expect("the run completes");
    let notes = joined(&report);
    // The document published run key `1-1`, so the capture is filed under that race.
    assert_eq!(report.errors, 0, "{notes}");
    assert!(
        notes.contains("documents: 1 event documents, 1 standings"),
        "{notes}"
    );
    assert!(
        notes.contains("legacy ids 1, short team keys 1"),
        "the `anli` and `ti` channels are read and counted: {notes}"
    );
    // Seven Waukon rows map; the standings row is the twenty-sixth runner, whose name, grade, school
    // and event key the document already minted, so both routes land on one performance.
    assert!(
        notes.contains("rows read: 137 (mapped 8, skipped 129)"),
        "{notes}"
    );
    assert!(notes.contains("athletes 7 performances 7"), "{notes}");

    let performances: Vec<CanonicalPerformance> =
        store.scan(Table::Performances).expect("performances read");
    assert_eq!(performances.len(), 7);
    let miriam = performances
        .iter()
        .find(|row| row.place == Some(26))
        .expect("the standings row is the one the payload placed 26th");
    assert_eq!(
        miriam.mark,
        Mark::TimeSeconds(1225.7),
        "for times the reader takes the raw channel, `20:25.700`"
    );
    assert_eq!(
        miriam.evidence.first().map(|row| row.source.url.clone()),
        Some(Some(event_doc_url(2_150_205))),
        "the document was folded first, so its evidence is the one kept"
    );
    // `anli` is counted, never minted: no entity carries it as an identity.
    let identities: Vec<String> = store
        .scan::<CanonicalAthlete>(Table::Athletes)
        .expect("athletes read")
        .into_iter()
        .flat_map(|athlete| athlete.source_identities)
        .map(|identity| identity.id)
        .collect();
    assert!(
        !identities.contains(&"42660317".to_string()),
        "{identities:?}"
    );
}

#[tokio::test]
async fn a_standings_capture_whose_run_key_no_document_published_is_refused() {
    let (dir, store, fetcher) = scratch();
    write_schools(
        &store,
        &[(UsJurisdiction::Michigan, "East Kentwood High School")],
    );
    let doc_path = stage_capture(&dir, "event-doc-2254280.json", HJ_MITS);
    let standings_path = stage_capture(
        &dir,
        "live-run-standings-9-9.json",
        &standings_payload("Miriam Downing", "SO", "East Kentwood High School"),
    );
    let options = ResultOptions {
        documents: vec![doc_path.clone()],
        standings: vec![StandingsCapture {
            run_id: "9-9".to_string(),
            path: standings_path.clone(),
        }],
        ..ResultOptions::for_meet(mits_meet(), OBSERVED_ON)
    };

    let report = collect(&context(&store, &fetcher), &options)
        .await
        .expect("the run completes");
    assert_eq!(report.errors, 1, "{}", joined(&report));
    assert!(
        joined(&report).contains(&format!(
            "capture refused: {standings_path}: no event document in this run published run key `9-9`"
        )),
        "{}",
        joined(&report)
    );
}

#[tokio::test]
async fn a_second_run_resumes_the_capture_the_first_journaled() {
    let (dir, store, fetcher) = scratch();
    let doc = parse_event_document(&event_doc_url(2_150_205), XC_STATE).expect("parses");
    write_schools(
        &store,
        &labels_of(&labelled_schools(&doc, UsJurisdiction::Iowa)),
    );
    let path = stage_capture(&dir, "event-doc-2150205.json", XC_STATE);
    let options = ResultOptions {
        documents: vec![path],
        ..ResultOptions::for_meet(state_meet(), OBSERVED_ON)
    };

    let first = collect(&context(&store, &fetcher), &options)
        .await
        .expect("the first run completes");
    assert_eq!(first.rows, 136);
    let second = collect(&context(&store, &fetcher), &options)
        .await
        .expect("the second run completes");
    assert_eq!(second.rows, 0, "the capture is not read twice");
    let notes = joined(&second);
    assert!(
        notes.contains("captures already journaled by an earlier run: 1"),
        "{notes}"
    );
    assert!(
        notes.contains("canonical entities: meets 1 events 0 teams 0 athletes 0 performances 0"),
        "a resumed run rewrites only the meet it files under, and reads no capture: {notes}"
    );
    let performances: Vec<CanonicalPerformance> =
        store.scan(Table::Performances).expect("performances read");
    assert_eq!(performances.len(), 136, "the tables are not appended twice");
}

#[tokio::test]
async fn the_event_summary_lists_individual_events_and_excludes_relays() {
    // Written to the shape `docs/events.rs` documents, because the summary capture (19,932 B) is
    // outside this lane's fixture budget.
    let summary = format!(
        r#"{{"a":{{"i":2254280,"ec":"Individual","rui":"19-1","ab":"HJ","un":"High Jump","gl":"Girls"}},
            "b":{{"i":999001,"ec":"Relay","rui":"7-1","peb":"Relay","ab":"4x400m"}},
            "c":{{"i":999002,"ec":"Individual","ab":"Underwater Basket Weaving"}}}}"#
    );
    let events =
        parse_event_summary(&event_summary_url(MITS_MEET), &summary).expect("the summary parses");
    assert_eq!(events.len(), 3);
    assert_eq!(events.iter().filter(|event| event.is_relay()).count(), 1);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event.kind(), EventKind::Unmapped { .. }))
            .count(),
        1
    );
    assert_eq!(events[0].event_id(), Some(2_254_280));

    // One summary payload the run reads through, with one of its two individual events supplied.
    let (dir, store, fetcher) = scratch();
    write_schools(
        &store,
        &[(UsJurisdiction::Michigan, "East Kentwood High School")],
    );
    let summary_path = stage_capture(&dir, "event-summary-61710.json", &summary);
    let doc_path = stage_capture(&dir, "event-doc-2254280.json", HJ_MITS);
    let options = ResultOptions {
        summary: Some(summary_path),
        documents: vec![doc_path],
        ..ResultOptions::for_meet(mits_meet(), OBSERVED_ON)
    };
    let report = collect(&context(&store, &fetcher), &options)
        .await
        .expect("the run completes");
    let notes = joined(&report);
    assert!(
        notes.contains("events listed 3 (relay 1, unmapped 1, unfetched 1)"),
        "the relay is counted and never fetched, and the event with no document is reported: {notes}"
    );
}
