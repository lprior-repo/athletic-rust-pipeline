use super::*;

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
        state: "AK".to_string(),
        grad_year: 2027,
        gender: "Girls".to_string(),
        sport: "CrossCountry".to_string(),
        event: "CrossCountry".to_string(),
        best_mark: "15:40.12".to_string(),
        best_value: 940.12,
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
    assert!(Measure::Time.better(10.94, 11.02));
    assert!(!Measure::Time.better(11.02, 10.94));
    assert!(Measure::Distance.better(6.42, 6.10));
    assert!(!Measure::Distance.better(6.10, 6.42));
    assert!(Measure::Field.better(1.85, 1.70));
    assert!(Measure::Points.better(3120.0, 2900.0));
    // Unparsed marks are carried but never chosen.
    assert_eq!(Measure::of(&Mark::Raw("DNS".to_string())), None);
}

#[test]
fn marks_print_in_the_notation_a_reader_expects() {
    assert_eq!(mark_text(&Mark::TimeSeconds(10.94)), "10.94");
    assert_eq!(mark_text(&Mark::TimeSeconds(281.23)), "4:41.23");
    assert_eq!(mark_text(&Mark::TimeSeconds(304.1)), "5:04.10");
    assert_eq!(mark_text(&Mark::DistanceMetres(6.4213)), "6.42 m");
    assert_eq!(
        mark_text(&Mark::FieldImperial {
            feet_mark: "5' 4\"".to_string(),
            metres: 1.63,
        }),
        "5' 4\""
    );
    assert_eq!(mark_text(&Mark::Points(3120.0)), "3120 pts");
}

#[test]
fn relays_are_not_personal_bests() {
    assert!(is_relay(&EventKind::Relay4x400));
    assert!(is_relay(&EventKind::SprintMedley));
    assert!(!is_relay(&EventKind::Track400m));
    assert!(!is_relay(&EventKind::CrossCountry));
}
