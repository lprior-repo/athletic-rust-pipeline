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

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const XC_STATE: &str =
    include_str!("../../../../tests/fixtures/athleticlive_results/event-doc-2150205.json");
const HJ_MITS: &str =
    include_str!("../../../../tests/fixtures/athleticlive_results/event-doc-2254280.json");
const OBSERVED_ON: &str = "2026-09-22";
const STATE_MEET: u64 = 58_504;
const MITS_MEET: u64 = 61_710;

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

fn state_meet() -> MeetTarget {
    target(
        STATE_MEET,
        "live_results",
        "Iowa High School State Championships",
        UsJurisdiction::Iowa,
        "2025-10-31",
    )
}

fn mits_meet() -> MeetTarget {
    target(
        MITS_MEET,
        "live_results",
        "MITS 4",
        UsJurisdiction::Michigan,
        "2026-02-14",
    )
}

fn scratch() -> TestResult<(tempfile::TempDir, Store, Fetcher)> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let fetcher = Fetcher::new(
        dir.path().join("http"),
        None,
        std::time::Duration::from_millis(1),
        std::collections::HashMap::new(),
        Vec::new(),
    )?;
    Ok((dir, store, fetcher))
}

fn context<'a>(store: &'a Store, fetcher: &'a Fetcher) -> TestResult<AdapterContext<'a>> {
    Ok(AdapterContext {
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2026).ok_or("2026 is a season")?,
        observed_on: OBSERVED_ON.to_string(),
        recording: None,
    })
}

fn stage_capture(dir: &tempfile::TempDir, name: &str, body: &str) -> TestResult<String> {
    let path = dir.path().join(name);
    std::fs::write(&path, body)?;
    Ok(path.to_string_lossy().into_owned())
}

fn write_schools(store: &Store, schools: &[(UsJurisdiction, &str)]) -> TestResult {
    let mut lines = String::new();
    for (state, name) in schools {
        let (school, _) = CanonicalSchool::new(*state, *name, normalize_name(name), None);
        lines.push_str(&serde_json::to_string(&school)?);
        lines.push('\n');
    }
    std::fs::create_dir_all(store.out_dir())?;
    std::fs::write(store.out_dir().join("schools.jsonl"), lines)?;
    Ok(())
}

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

fn journal(store: &Store) -> TestResult<std::collections::HashSet<String>> {
    Ok(store.journal_keys(super::PHASE)?)
}

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

#[test]
fn event_document_rows_carry_the_published_shapes() -> TestResult {
    let xc = parse_event_document(&event_doc_url(2_150_205), XC_STATE)?;
    check!(eq; xc.event_id(), Some(2_150_205));
    check!(eq; xc.meet_id(), Some(STATE_MEET));
    check!(eq; xc.rows.len(), 136);
    check!(eq; xc.label(), Some("Run"));
    check!(eq; xc.kind(), EventKind::CrossCountry);
    check!(eq; xc.gender_group.as_deref(), Some("Girls"));
    check!(eq; xc.run_id(), Some("1-1"));
    check!(eq; xc.division_name(), None, "`dv` is null on this capture");
    check!(eq; xc.rows[0].place(), Some(1));
    check!(eq; xc.rows[0].mark.as_deref(), Some("18:20.7"));
    check!(eq;
        xc.rows[0].canonical_mark(&EventKind::CrossCountry),
        Some(Mark::TimeSeconds(CentiSeconds::new(110070)))
    );
    check!(eq; xc.rows[0].splits.len(), 3, "the cross-country split list");

    let hj = parse_event_document(&event_doc_url(2_254_280), HJ_MITS)?;
    check!(eq; hj.event_id(), Some(2_254_280));
    check!(eq; hj.meet_id(), Some(MITS_MEET));
    check!(eq; hj.rows.len(), 17);
    check!(eq; hj.label(), Some("HJ"));
    check!(eq; hj.kind(), EventKind::HighJump);
    check!(eq; hj.division_name(), Some("MITS"));
    check!(eq; hj.run_id(), Some("19-1"));
    check!(eq;
        hj.rows[0].splits.len(),
        0,
        "a field event publishes no splits"
    );
    check!(eq;
        hj.rows[0].canonical_mark(&EventKind::HighJump),
        Some(Mark::FieldImperial {
            feet_mark: "5-02.00".to_string(),
            metres: CentiMetres::new(157),
        })
    );
    let no_height = &hj.rows[13];
    check!(eq; no_height.mark.as_deref(), Some("NH"));
    check!(eq; no_height.canonical_mark(&EventKind::HighJump), None);
    check!(eq; no_height.place(), None, "an unplaced row publishes `--`");
    let blank_grade = &hj.rows[10];
    check!(eq; blank_grade.mark.as_deref(), Some("4-06.00"));
    check!(eq;
        blank_grade
            .athlete
            .as_ref()
            .and_then(|athlete| athlete.grade.as_ref())
            .and_then(|grade| grade.as_str()),
        Some("")
    );
    Ok(())
}

#[test]
fn both_mark_channels_agree_on_every_captured_row() -> TestResult {
    let xc = parse_event_document(&event_doc_url(2_150_205), XC_STATE)?;
    let mut time_rows = 0usize;
    for row in &xc.rows {
        let published = row.mark.as_deref().ok_or("every row publishes a mark")?;
        let seconds = crate::hytek::parse_time(published).ok_or("the time parses")?;
        let Some(Mark::TimeSeconds(minted)) = row.canonical_mark(&EventKind::CrossCountry) else {
            return Err(format!("row {:?} mints no time mark", row.place()).into());
        };
        check!(
            (minted.value() - seconds.value()).abs() < 1,
            "{published} parsed {seconds} but the integer channel minted {minted}"
        );
        time_rows += 1;
    }
    check!(eq; time_rows, 136);

    let hj = parse_event_document(&event_doc_url(2_254_280), HJ_MITS)?;
    let mut field_rows = 0usize;
    for row in &hj.rows {
        let Some(Mark::FieldImperial { metres, .. }) = row.canonical_mark(&EventKind::HighJump)
        else {
            check!(eq; row.mark.as_deref(), Some("NH"), "only `NH` mints no mark");
            continue;
        };
        let micros = row
            .mark_int
            .as_ref()
            .and_then(|value| value.as_f64())
            .ok_or("im publishes")?;
        check!(
            (metres.value()
                - CentiMetres::try_from_metres_f64(micros / 1_000_000.0)
                    .ok_or("in range")?
                    .value())
            .abs()
                < 1,
            "{} published {metres} m against {micros} µm",
            row.mark
                .as_deref()
                .map_or(Default::default(), core::convert::identity)
        );
        field_rows += 1;
    }
    check!(eq; field_rows, 13, "four of the seventeen rows are `NH`");
    Ok(())
}
mod integration;
mod integration2;
mod integration3;
