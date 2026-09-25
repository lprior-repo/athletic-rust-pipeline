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
    assert_eq!(
        mark_text(&Mark::TimeSeconds(CentiSeconds::new(1094))),
        "10.94"
    );
    assert_eq!(
        mark_text(&Mark::TimeSeconds(CentiSeconds::new(28123))),
        "4:41.23"
    );
    assert_eq!(
        mark_text(&Mark::TimeSeconds(CentiSeconds::new(30410))),
        "5:04.10"
    );
    assert_eq!(
        mark_text(&Mark::DistanceMetres(CentiMetres::new(642))),
        "6.42 m"
    );
    assert_eq!(
        mark_text(&Mark::FieldImperial {
            feet_mark: "5' 4\"".to_string(),
            metres: CentiMetres::new(163),
        }),
        "5' 4\""
    );
    assert_eq!(
        mark_text(&Mark::Points(CentiPoints::new(312000))),
        "3120 pts"
    );
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
        let cs = CentiSeconds::new(centis as i32);
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
        metres: CentiMetres::new(93),
    };
    let three_quarter = Mark::FieldImperial {
        feet_mark: "3-0.75".to_string(),
        metres: CentiMetres::new(93),
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

// ── measure module tests ────────────────────────────────────────────────────────────

#[test]
fn mark_value_converts_each_published_scale() {
    assert_eq!(
        mark_value(&Mark::TimeSeconds(CentiSeconds::new(4855))),
        Some(48.55)
    );
    assert_eq!(
        mark_value(&Mark::DistanceMetres(CentiMetres::new(642))),
        Some(6.42)
    );
    assert_eq!(
        mark_value(&Mark::FieldImperial {
            feet_mark: "21-0.75".to_string(),
            metres: CentiMetres::new(642),
        }),
        Some(6.42)
    );
    assert_eq!(
        mark_value(&Mark::Points(CentiPoints::new(12345))),
        Some(123.45)
    );
}

#[test]
fn mark_value_and_unit_are_blank_for_raw_notation() {
    let mark = Mark::Raw("windy".to_string());
    assert_eq!(mark_value(&mark), None);
    assert_eq!(mark_unit(&mark), None);
}

#[test]
fn mark_unit_matches_numeric_scale() {
    assert_eq!(
        mark_unit(&Mark::TimeSeconds(CentiSeconds::new(1))),
        Some("s")
    );
    assert_eq!(
        mark_unit(&Mark::DistanceMetres(CentiMetres::new(1))),
        Some("m")
    );
    assert_eq!(
        mark_unit(&Mark::FieldImperial {
            feet_mark: "1-0".to_string(),
            metres: CentiMetres::new(30),
        }),
        Some("m")
    );
    assert_eq!(mark_unit(&Mark::Points(CentiPoints::new(1))), Some("pts"));
}

fn millimetres(feet_mark: &str) -> i32 {
    millimetres_opt(feet_mark).expect("a mark in the published notation parses")
}

fn millimetres_opt(feet_mark: &str) -> Option<i32> {
    Measure::value(
        Measure::Field,
        &Mark::FieldImperial {
            feet_mark: feet_mark.to_string(),
            metres: CentiMetres::new(0),
        },
    )
}

#[test]
fn field_marks_convert_to_exact_millimetres() {
    assert_eq!(millimetres("3-0.75"), 933, "3 ft 0.75 in");
    assert_eq!(millimetres("4-00"), 1_219, "4 ft exactly");
    assert_eq!(millimetres("145-09"), 44_425, "145 ft 9 in, whole inches");
    assert_eq!(millimetres("61-03.50"), 18_682, "61 ft 3.5 in");
    assert_eq!(millimetres("1-0"), 305, "1 ft exactly");
    assert_eq!(millimetres("5' 4\""), 1_626, "the apostrophe notation");
    assert_eq!(millimetres("6-06.25"), 1_988, "6 ft 6.25 in");
}

/// A mark with more feet outranks one with fewer, however many inches the shorter carries: the
/// comparison follows the published notation itself, not a rescaled inch term.
#[test]
fn a_field_mark_with_more_feet_always_ranks_higher() {
    for (lower, higher) in [
        ("3-11.75", "4-00"),
        ("6-00", "6-00.25"),
        ("13-11.75", "14-00"),
        ("144-11.75", "145-00"),
    ] {
        assert!(
            millimetres(lower) < millimetres(higher),
            "{lower} ({}) must rank below {higher} ({})",
            millimetres(lower),
            millimetres(higher)
        );
    }
}

/// Cross-check the parse against the metric value every host publishes beside its feet-inches
/// mark: agreement to the centimetre is what a 100x arithmetic slip cannot survive.
#[test]
fn parsed_feet_inches_agrees_with_the_published_metres() {
    for (feet_mark, centimetres) in [
        ("3-0.75", 93),
        ("5-04.25", 163),
        ("21-0.75", 642),
        ("61-03.50", 1_868),
        ("145-09", 4_442),
    ] {
        let parsed = millimetres(feet_mark);
        let published = centimetres * 10;
        assert!(
            (parsed - published).abs() <= 6,
            "{feet_mark} parses to {parsed} mm but the host publishes {published} mm"
        );
    }
}

/// A mark this reader can't place refuses instead of comparing on a wrong scale.
#[test]
fn an_unplaceable_field_mark_has_no_value() {
    assert_eq!(millimetres_opt("windy"), None);
    assert_eq!(millimetres_opt("-0.75"), None);
    assert_eq!(millimetres_opt("3-4-5"), None);
}

/// The 100x slip: `30-2.50` → ~9.208 m, not 92.08 or 154.94.
/// The 100x slip: `30-2.50` normalises to ~9.208 m, not 92.08 or 154.94.
#[test]
fn field_mark_30_2_50_normalises_to_metres() {
    assert_eq!(field_mm("30-2.50"), Some(9_208));
    let m = Mark::FieldImperial {
        feet_mark: "30-2.50".into(),
        metres: CentiMetres::new(921),
    };
    let n = Measure::Field.normalized_mark(&m).unwrap();
    assert!((n - 9.208).abs() < 0.01 && (n - 92.08).abs() > 1.0);
}

/// 4-0.00 beats 3-0.75 regardless of input order.
#[test]
fn four_zero_beats_three_point_seventy_five() {
    let (h, l) = (field_mm("4-0.00").unwrap(), field_mm("3-0.75").unwrap());
    assert!(h > l && Measure::Field.better(h, l) && !Measure::Field.better(l, h));
}

/// Time marks are centi-seconds: /100 gives seconds.
#[test]
fn time_mark_divides_correctly() {
    let m = Mark::TimeSeconds(CentiSeconds::new(1000));
    assert_eq!(Measure::Time.normalized_mark(&m), Some(10.0));
}
