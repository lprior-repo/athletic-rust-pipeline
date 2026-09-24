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

use super::run::Run;
use super::{collect, collect_manifest, ManifestOptions, ResultOptions, StandingsCapture};
use crate::athleticlive::docs::{parse_event_document, parse_event_summary, EventDoc};
use crate::athleticlive::map::SOURCE_ID;
use crate::athleticlive::wire::{event_doc_url, event_summary_url};
use crate::athleticlive_athletes::{school_year_for_date, MeetTarget};
use crate::net::Fetcher;
use crate::{AdapterContext, AdapterReport};
use census_domain::model::CentiMetres;
use census_domain::model::CentiSeconds;
use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, EventKind, Gender, GradYear, Grade, Mark, SchoolYear,
    SourceAthleteObservation, SourceNamespace, SourceObservation,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::collections::{BTreeMap, HashMap};

const XC_STATE: &str =
    include_str!("../../../tests/fixtures/athleticlive_results/event-doc-2150205.json");
const HJ_MITS: &str =
    include_str!("../../../tests/fixtures/athleticlive_results/event-doc-2254280.json");
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
        crate::athleticlive::infer_level(name),
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
        school_year: SchoolYear::new(2026).expect("2026 is a season"),
        observed_on: OBSERVED_ON.to_string(),
        recording: None,
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

/// The route's journal entries, as the store holds them.
fn journal(store: &Store) -> std::collections::HashSet<String> {
    store.journal_keys(super::PHASE).expect("journal keys")
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
        Some(Mark::TimeSeconds(CentiSeconds(110070)))
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
            metres: CentiMetres(157),
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
        let seconds = crate::hytek::parse_time(published).expect("the time parses");
        let Some(Mark::TimeSeconds(minted)) = row.canonical_mark(&EventKind::CrossCountry) else {
            panic!("row {:?} mints no time mark", row.place());
        };
        assert!(
            (minted.0 - seconds.0).abs() < 1,
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
            (metres.0 - CentiMetres::from_metres_f64(micros / 1_000_000.0).0).abs() < 1,
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
    assert_eq!(winner.mark, Mark::TimeSeconds(CentiSeconds(110070)));
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
        school_year_for_date(
            &meets[0].date,
            SchoolYear::new(2026).expect("2026 is a season")
        ),
        SchoolYear::containing(2025, 10).expect("2025-10 is a season")
    );
    assert!(athletes
        .iter()
        .any(|athlete| athlete.grad_year == GradYear::new(2026).expect("2026 is a cohort")));
    assert!(athletes
        .iter()
        .any(|athlete| athlete.grad_year == GradYear::new(2029).expect("2029 is a cohort")));

    // Every performance whose athlete object published an Athletic.net id names that object, and the
    // identity it names is one its athlete row holds: the §31 key that lets a mis-merged athlete be
    // told apart from what the capture said, without reading the capture again. The capture's four
    // rows whose athlete object published no id stay unnamed rather than borrowing a canonical id.
    let by_id: HashMap<&str, &CanonicalAthlete> = athletes
        .iter()
        .map(|athlete| (athlete.id.as_str(), athlete))
        .collect();
    let named = performances
        .iter()
        .filter(|performance| performance.source_athlete.is_some())
        .count();
    assert_eq!(
        named, 132,
        "the {named} rows that name a source athlete are the capture's 132 that published one"
    );
    for performance in &performances {
        let Some(identity) = performance.source_athlete.as_ref() else {
            continue;
        };
        let athlete = by_id
            .get(performance.athlete.as_str())
            .unwrap_or_else(|| panic!("no athlete row for {}", performance.athlete.as_str()));
        assert!(
            athlete.source_identities.contains(identity),
            "the stamped identity is the athlete's own: {identity:?}"
        );
        assert!(
            !identity.id.is_empty(),
            "the identity carries the provider's own athlete id"
        );
    }
}

/// What the athlete observation log is for: a result pass files what the capture itself published
/// about each athlete — the provider's own athlete id, the name and the school beside it — next to
/// the canonical rows the same pass mints, so a canonical merge can be re-decided from this pass.
///
/// Every assertion below is against the capture: the ids, names and schools come from the fixture's
/// own `a` objects, and the assertions read the store's `SourceObservations` rows rather than any
/// count this run reported about itself.
#[tokio::test]
async fn a_result_pass_files_one_observation_per_athlete_id_the_capture_published() {
    let (dir, store, fetcher) = scratch();
    let doc = parse_event_document(&event_doc_url(2_150_205), XC_STATE).expect("parses");
    let labels = labelled_schools(&doc, UsJurisdiction::Iowa);
    write_schools(&store, &labels_of(&labels));
    let path = stage_capture(&dir, "event-doc-2150205.json", XC_STATE);
    let options = ResultOptions {
        documents: vec![path],
        ..ResultOptions::for_meet(state_meet(), OBSERVED_ON)
    };

    let report = collect(&context(&store, &fetcher), &options)
        .await
        .expect("the run completes");
    assert_eq!(report.errors, 0, "{}", joined(&report));

    // The capture's own ledger: one entry per row that publishes an Athletic.net athlete id, holding
    // the name, the school and the class token the capture printed beside that id.
    let grade_number = |token: &str| match token {
        "FR" => Some(9),
        "SO" => Some(10),
        "JR" => Some(11),
        "SR" => Some(12),
        other => other.parse::<u8>().ok(),
    };
    let published: BTreeMap<u64, (&str, &str, u8)> = doc
        .rows
        .iter()
        .filter_map(|row| {
            let athlete = row.athlete.as_ref()?;
            let id = athlete
                .an_athlete_id
                .as_ref()
                .and_then(|value| value.as_u64())?;
            let name = athlete.name.as_deref()?;
            let school = athlete.team.as_ref().and_then(|team| team.school_name())?;
            let grade = athlete
                .grade
                .as_ref()
                .and_then(|value| value.as_str())
                .and_then(grade_number)?;
            Some((id, (name, school, grade)))
        })
        .collect();
    assert_eq!(
        published.len(),
        132,
        "the capture publishes 132 rows each carrying its own athlete id"
    );

    let observations: Vec<SourceObservation> = store
        .scan(Table::SourceObservations)
        .expect("observation log");
    let filed: BTreeMap<u64, &SourceAthleteObservation> = observations
        .iter()
        .filter_map(|row| match row {
            SourceObservation::Athlete(athlete) => {
                Some((athlete.source_athlete_id.parse().ok()?, athlete))
            }
            SourceObservation::School(_) => None,
        })
        .collect();
    assert_eq!(
        filed.keys().copied().collect::<Vec<_>>(),
        published.keys().copied().collect::<Vec<_>>(),
        "one observation per athlete id the capture published, keyed by the provider's own id"
    );

    let captured: Vec<&str> = labels.iter().map(|(_, name)| name.as_str()).collect();
    for (id, (name, school, grade)) in &published {
        let seen = filed.get(id).expect("every published id is filed");
        assert_eq!(
            seen.namespace,
            SourceNamespace::LegacyAthleticNet {
                kind: "athlete".to_string()
            }
        );
        assert_eq!(seen.id, format!("legacy_athletic_net:athlete:{id}"));
        assert_eq!(
            seen.observed_name.as_str(),
            *name,
            "the name the capture spelled"
        );
        assert_eq!(
            seen.observed_school.as_deref(),
            Some(*school),
            "the school the capture placed the athlete at"
        );
        assert!(
            captured.contains(school),
            "the observed school is one the capture published: {school}"
        );
        assert_eq!(
            seen.observed_grade
                .as_ref()
                .map(|observed| observed.grade.get()),
            Some(*grade),
            "the class the capture's own token states"
        );
        assert_eq!(seen.gender, Gender::Girls);
        assert_eq!(
            seen.profile_url.as_deref(),
            Some(format!("https://www.athletic.net/athlete/{id}/track-and-field").as_str()),
            "the profile page Athletic.net's own athlete id derives"
        );
        assert_eq!(seen.observed_on, OBSERVED_ON);
        assert!(
            !seen.source_row_key.is_empty(),
            "the row states where it was read"
        );
    }
    let winner = filed
        .get(&17_327_390)
        .expect("the capture's first-placed row");
    assert_eq!(winner.observed_name, "McKenna Montgomery");
    assert_eq!(winner.observed_school.as_deref(), Some("Albia"));
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
        Mark::TimeSeconds(CentiSeconds(122570)),
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

/// The unit of work is the capture: its rows and the entry naming it commit together, so a walk that
/// stops before the append leaves the capture unread and the next run reads it again. The
/// discriminating assertions are the store's own rows: a run that resumed a capture whose rows never
/// landed writes the meet it files under and nothing else, which is what the resumed-run test above
/// expects of a capture that did land.
#[tokio::test]
async fn a_capture_whose_rows_never_landed_is_read_again_by_the_next_run() {
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

    // The interrupted walk: it reads its capture, then the run ends before the append.
    let mut walk =
        Run::new(&context(&store, &fetcher), &state_meet(), &options).expect("the walk opens");
    walk.read_captures(&options).expect("the capture is read");
    assert!(
        journal(&store).is_empty(),
        "the walk writes no entry of its own: {:?}",
        journal(&store)
    );
    assert!(
        store
            .scan::<CanonicalPerformance>(Table::Performances)
            .expect("performances read")
            .is_empty(),
        "the walk writes no row of its own"
    );
    drop(walk);

    // The next run: the same capture, read again, with its rows and its entry landing together.
    let report = collect(&context(&store, &fetcher), &options)
        .await
        .expect("the second run completes");
    assert_eq!(report.rows, 136, "the capture is a document of 136 rows");
    let performances: Vec<CanonicalPerformance> =
        store.scan(Table::Performances).expect("performances read");
    assert_eq!(
        performances.len(),
        136,
        "the capture's rows land on the second run, not skipped as read"
    );
    let meets: Vec<CanonicalMeet> = store.scan(Table::Meets).expect("meets read");
    assert_eq!(meets.len(), 1, "the meet the run files under");
    assert_eq!(
        journal(&store),
        std::collections::HashSet::from([path]),
        "one entry per capture read"
    );
}

#[tokio::test]
async fn the_event_summary_lists_individual_events_and_excludes_relays() {
    // Written to the shape `docs/events.rs` documents, because the summary capture (19,932 B) is
    // outside this lane's fixture budget.
    let summary = r#"{"a":{"i":2254280,"ec":"Individual","rui":"19-1","ab":"HJ","un":"High Jump","gl":"Girls"},
            "b":{"i":999001,"ec":"Relay","rui":"7-1","peb":"Relay","ab":"4x400m"},
            "c":{"i":999002,"ec":"Individual","ab":"Underwater Basket Weaving"}}"#.to_string();
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

#[tokio::test]
async fn a_manifest_imports_every_meet_it_names_and_lands_on_the_harvest_ids() {
    let (dir, store, fetcher) = scratch();
    let doc = parse_event_document(&event_doc_url(2_150_205), XC_STATE).expect("parses");
    write_schools(
        &store,
        &labels_of(&labelled_schools(&doc, UsJurisdiction::Iowa)),
    );
    let xc_path = stage_capture(&dir, "event-doc-2150205.json", XC_STATE);
    let hj_path = stage_capture(&dir, "event-doc-2254280.json", HJ_MITS);
    let manifest = serde_json::json!({
        "meets": [
            {
                "athleticlive_meet_id": STATE_MEET,
                "tenant": "live_results",
                "name": "Iowa High School State Championships",
                "state": "IA",
                "date": "2025-10-31",
                "documents": [xc_path],
            },
            {
                "athleticlive_meet_id": MITS_MEET,
                "tenant": "live_results",
                "name": "MITS 4",
                "state": "Michigan",
                "date": "2026-02-14",
                "documents": [hj_path],
            },
            {
                "athleticlive_meet_id": STATE_MEET,
                "tenant": "live_results",
                "name": "Iowa High School State Championships",
                "state": "IA",
                "date": "2025-10-31",
                "documents": [xc_path],
            }
        ]
    });
    let manifest_path = stage_capture(&dir, "manifest.json", &manifest.to_string());
    let options = ManifestOptions {
        input: Some(manifest_path),
        limit: None,
        observed_on: OBSERVED_ON.to_string(),
        states: Vec::new(),
    };

    let report = collect_manifest(&context(&store, &fetcher), &options)
        .await
        .expect("the import completes");
    assert_eq!(report.adapter, SOURCE_ID);
    assert_eq!(report.requests, 0, "the adapter fetches nothing");
    let notes = joined(&report);
    assert!(
        notes.contains("3 meets read, 1 repeated ids dropped, 2 in scope, 2 read"),
        "the manifest accounting is reported: {notes}"
    );
    for id in [STATE_MEET, MITS_MEET] {
        assert!(
            notes.contains(&format!("meet {id}:")),
            "meet {id} reports its own walk: {notes}"
        );
    }

    // The meet the results route minted is the one the harvest route already minted: same id, so
    // importing results never forks a second canonical meet for one published meet.
    let meets: Vec<CanonicalMeet> = store.scan(Table::Meets).expect("meets read");
    assert_eq!(meets.len(), 2, "one canonical meet per manifest entry");
    for expected in [state_meet().meet_id, mits_meet().meet_id] {
        assert!(
            meets.iter().any(|meet| meet.id.as_str() == expected),
            "the harvest id {expected} is the one results landed on"
        );
    }

    assert_eq!(
        report.rows, 136,
        "the state final maps 136 rows; the field event's club-labelled rows map none"
    );

    let performances: Vec<CanonicalPerformance> =
        store.scan(Table::Performances).expect("performances read");
    assert!(
        performances
            .iter()
            .any(|row| row.source_key.starts_with("athleticlive:2150205:")),
        "the state final's rows are keyed by its event"
    );
    assert!(
        notes.contains("rows read: 17 (mapped 0, skipped 17)"),
        "the field event's capture is read and its unlabelled rows are refused, not dropped: {notes}"
    );
    assert!(
        performances.len() >= 136,
        "the state final alone contributes 136 rows, saw {}",
        performances.len()
    );
}

#[tokio::test]
async fn a_manifest_that_places_no_jurisdiction_fails_by_name() {
    let body = r#"{"meets":[{"athleticlive_meet_id":1,"tenant":"t","name":"n","state":"Atlantis","date":"2026-01-01"}]}"#;
    let error = super::manifest::parse_manifest(body, OBSERVED_ON).expect_err("no jurisdiction");
    let text = error.to_string();
    assert!(
        text.contains("Atlantis") && text.contains("meet 1"),
        "the refusal names the entry: {text}"
    );
}

#[tokio::test]
async fn the_results_route_requires_a_manifest() {
    let (_dir, store, fetcher) = scratch();
    let error = collect_manifest(
        &context(&store, &fetcher),
        &ManifestOptions {
            input: None,
            limit: None,
            observed_on: OBSERVED_ON.to_string(),
            states: Vec::new(),
        },
    )
    .await
    .expect_err("a manifest is required");
    assert!(
        error.to_string().contains("requires --input"),
        "the refusal names the flag: {error}"
    );
}
