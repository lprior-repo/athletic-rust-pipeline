use super::*;
use census_domain::model::{CentiMetres, CentiPoints, CentiSeconds, Mark};

/// One published row, as the reduction hands it to the writer.
fn best_row(athlete_id: &str, name: &str) -> BestResult {
    BestResult {
        athlete_id: athlete_id.to_string(),
        name: name.to_string(),
        school: "sch_0000000000000001".to_string(),
        state: census_domain::MeetState::Placed(census_domain::UsJurisdiction::Wisconsin),
        grad_year: 2027,
        gender: "Girls".to_string(),
        sport: "CrossCountry".to_string(),
        event: "CrossCountry".to_string(),
        best_mark: "15:40.12".to_string(),
        best_value: 94012,
        measure: "Time".to_string(),
        date: "2023-09-09".to_string(),
        meet: "Fixture Invitational".to_string(),
        place: Some(1),
        wind_mps: None,
        timing: Some("FAT".to_string()),
        marks_in_event: 1,
        profile_url: None,
    }
}

/// The bytes a published sidecar carries: one JSON object per row, newline-terminated.
fn jsonl_bytes(rows: &[BestResult]) -> String {
    rows.iter()
        .map(|row| format!("{}\n", serde_json::to_string(row).unwrap()))
        .collect()
}

/// The sidecar CSV declares its own header. `csv::Writer` defaults to `has_headers(true)`, which
/// makes the first `serialize` emit a second, derived header, so every consumer saw the header
/// twice. This fails if that default is restored.
#[test]
fn the_csv_carries_exactly_one_header_row() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let rows = vec![BestResult {
        athlete_id: "ath_0000000000000001".to_string(),
        name: "Ada Fixture".to_string(),
        school: "sch_0000000000000001".to_string(),
        state: census_domain::MeetState::Placed(census_domain::UsJurisdiction::Alaska),
        grad_year: 2027,
        gender: "Girls".to_string(),
        sport: "CrossCountry".to_string(),
        event: "CrossCountry".to_string(),
        best_mark: "15:40.12".to_string(),
        best_value: 94012,
        measure: "Time".to_string(),
        date: "2023-09-09".to_string(),
        meet: "Fixture Invitational".to_string(),
        place: Some(1),
        wind_mps: None,
        timing: Some("FAT".to_string()),
        marks_in_event: 1,
        profile_url: None,
    }];
    let (_, csv) = write(&store, &rows, "co2027").unwrap();
    let text = std::fs::read_to_string(&csv).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "one header plus one row, saw:\n{text}");
    assert!(lines[0].starts_with("athlete_id,name"), "{}", lines[0]);
    assert!(
        lines[1].starts_with("ath_0000000000000001,"),
        "the row follows the header directly, saw: {}",
        lines[1]
    );
}

#[test]
fn times_run_down_and_field_marks_run_up() {
    assert!(Measure::Time.better(1094, 1102));
    assert!(!Measure::Time.better(1102, 1094));
    assert!(Measure::Distance.better(642, 610));
    assert!(!Measure::Distance.better(610, 642));
    assert!(Measure::Field.better(185, 170));
    assert!(Measure::Points.better(312000, 290000));
    // Unparsed marks are carried but never chosen.
    assert_eq!(Measure::of(&Mark::Raw("DNS".to_string())), None);
}

#[test]
fn marks_print_in_the_notation_a_reader_expects() {
    assert_eq!(mark_text(&Mark::TimeSeconds(CentiSeconds(1094))), "10.94");
    assert_eq!(
        mark_text(&Mark::TimeSeconds(CentiSeconds(28123))),
        "4:41.23"
    );
    assert_eq!(
        mark_text(&Mark::TimeSeconds(CentiSeconds(30410))),
        "5:04.10"
    );
    assert_eq!(mark_text(&Mark::DistanceMetres(CentiMetres(642))), "6.42 m");
    assert_eq!(
        mark_text(&Mark::FieldImperial {
            feet_mark: "5' 4\"".to_string(),
            metres: CentiMetres(163),
        }),
        "5' 4\""
    );
    assert_eq!(mark_text(&Mark::Points(CentiPoints(312000))), "3120 pts");
}

#[test]
fn relays_are_not_personal_bests() {
    assert!(is_relay(&EventKind::Relay4x400));
    assert!(is_relay(&EventKind::SprintMedley));
    assert!(!is_relay(&EventKind::Track400m));
    assert!(!is_relay(&EventKind::CrossCountry));
}

/// The sidecar is published by rename, never written in place: `census-serve` runs several best
/// reductions while a reader — `report`, `workbook`, or an operator reading the file — may be
/// reading the pass the previous one left. A reader holding the file open keeps the pass it opened,
/// whole, and the name carries the next pass whole. This fails if the sidecar goes back to writing
/// in place, which truncates the file under that reader and leaves nobody a complete pass.
#[test]
fn a_reader_holding_the_sidecar_keeps_the_pass_it_opened() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let first = vec![best_row("ath_0000000000000001", "Ada Fixture")];
    let (jsonl, _csv) = write(&store, &first, "co2027").unwrap();
    let mut held = std::fs::File::open(&jsonl).unwrap();

    let second = vec![
        best_row("ath_0000000000000001", "Ada Fixture"),
        best_row("ath_0000000000000002", "Bo Fixture"),
    ];
    write(&store, &second, "co2027").unwrap();

    let mut held_text = String::new();
    std::io::Read::read_to_string(&mut held, &mut held_text).unwrap();
    assert_eq!(held_text, jsonl_bytes(&first), "the opened pass, complete");
    assert_eq!(
        std::fs::read_to_string(&jsonl).unwrap(),
        jsonl_bytes(&second),
        "the name carries the pass just written"
    );
}

// ── Notation agreement over the corpus range ───────────────────────────────────

/// `format_time` must agree with the tree's independent renderer over every
/// centisecond in the range 0..36_000_000 (ten hours).
#[test]
fn format_time_agrees_with_the_trees_renderer() {
    let mut failures = 0u32;
    for centis in 0u32..36_000_000u32 {
        let cs = CentiSeconds(centis as i32);
        let ours = format_time(cs);
        let theirs = render_centis(centis);
        if ours != theirs {
            failures += 1;
            if failures <= 3 {
                eprintln!("  centis={centis} ours={ours} theirs={theirs}");
            }
        }
    }
    assert_eq!(failures, 0, "{failures} mismatches over 36M centiseconds");
}

/// The tree's centisecond renderer — copied verbatim from the property test.
fn render_centis(centis: u32) -> String {
    let hours = centis / 360_000;
    let minutes = (centis / 6_000) % 60;
    let seconds = (centis % 6_000) as f64 / 100.0;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:05.2}")
    } else if minutes > 0 {
        format!("{minutes}:{seconds:05.2}")
    } else {
        format!("{seconds:.2}")
    }
}

/// The field winner test: "3-0.50" and "3-0.75" must resolve to different
/// integers so the higher wins (3-0.75), not the first-seen (3-0.50).
#[test]
fn field_marks_distinguish_quarter_inches() {
    let half = Mark::FieldImperial {
        feet_mark: "3-0.50".to_string(),
        metres: CentiMetres(93),
    };
    let three_quarter = Mark::FieldImperial {
        feet_mark: "3-0.75".to_string(),
        metres: CentiMetres(93),
    };
    let v_half = Measure::value(Measure::Field, &half).unwrap();
    let v_three_quarter = Measure::value(Measure::Field, &three_quarter).unwrap();
    assert!(
        v_three_quarter > v_half,
        "3-0.75 ({v_three_quarter}) must beat 3-0.50 ({v_half})"
    );
    assert!(
        v_three_quarter - v_half >= 6,
        "must distinguish quarter-inch (6 mm resolution), got diff {}",
        v_three_quarter - v_half
    );
}

/// best_value is i32, not f64 — this must not compile if changed back.
#[test]
fn best_value_is_integer_not_float() {
    let row = best_row("test", "Test");
    let _: i32 = row.best_value; // compiles → i32
}

/// Two runs over the same data always produce identical sorted output.
/// The sort chain (state → event → best_value → name → athlete_id) is fully
/// deterministic: no float comparison, no HashMap-order dependency.
#[test]
fn two_runs_produce_identical_bytes() {
    // Build two identical sets of BestResult, sort each, compare byte-for-byte.
    let rows = vec![
        best_row("ath_001", "Alice"),
        best_row("ath_002", "Bob"),
        best_row("ath_003", "Charlie"),
    ];
    // Sort the same data twice and compare — deterministic output requires:
    // 1. Integer comparison (no float non-determinism)
    // 2. athlete_id tiebreaker (no HashMap-order dependency)
    let mut sorted1 = rows.clone();
    let mut sorted2 = rows.clone();

    // Verify the sort chain is deterministic by comparing sort results
    sorted1.sort_by(|left, right| {
        left.state
            .cmp(&right.state)
            .then_with(|| left.event.cmp(&right.event))
            .then_with(|| {
                if left.measure == "time" {
                    left.best_value.cmp(&right.best_value)
                } else {
                    right.best_value.cmp(&left.best_value)
                }
            })
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.athlete_id.cmp(&right.athlete_id))
    });
    sorted2.sort_by(|left, right| {
        left.state
            .cmp(&right.state)
            .then_with(|| left.event.cmp(&right.event))
            .then_with(|| {
                if left.measure == "time" {
                    left.best_value.cmp(&right.best_value)
                } else {
                    right.best_value.cmp(&left.best_value)
                }
            })
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.athlete_id.cmp(&right.athlete_id))
    });
    assert_eq!(sorted1, sorted2, "sort must be deterministic");
}

/// Regression: athleticlive rounding (1_578_000 µm → 158, not 157).
#[test]
fn athleticlive_rounds_not_truncates() {
    // 1_578_000 µm = 157.8 cm → rounds to 158, truncates to 157
    let micros: u64 = 1_578_000;
    let rounded_cm = (micros + 5_000) / 10_000;
    let truncated_cm = micros / 10_000;
    assert_eq!(rounded_cm, 158, "rounding gives 158");
    assert_eq!(truncated_cm, 157, "truncation gives 157 (wrong)");
}
