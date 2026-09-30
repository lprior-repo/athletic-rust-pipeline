use crate::school_directory::{
    decide, next_due, Cadence, DueReason, Month, Parity, ScheduleLedger, ScheduleSource,
    UpdateDecision, YearMonth,
};

fn ym(year: i32, month: u8) -> YearMonth {
    YearMonth::new(year, Month::new(month).expect("month number")).expect("year month")
}

fn cadences() -> Vec<Cadence> {
    vec![
        Cadence::Yearly {
            month: Month::SEPTEMBER,
        },
        Cadence::Biennial {
            parity: Parity::Even,
        },
        Cadence::Biennial {
            parity: Parity::Odd,
        },
        Cadence::Semester {
            months: [Month::JANUARY, Month::JULY],
        },
        Cadence::Quarterly {
            months: [Month::JANUARY, Month::APRIL, Month::JULY, Month::OCTOBER],
        },
    ]
}

#[test]
fn months_and_year_months_are_validated_and_rendered() {
    assert_eq!(Month::new(12).expect("december").number(), 12);
    assert_eq!(Month::new(9).expect("september").to_string(), "09");
    assert!(Month::new(0).is_err());
    assert!(Month::new(13).is_err());

    assert_eq!(ym(2026, 9).to_string(), "2026-09");
    assert_eq!(ym(2026, 9).year(), 2026);
    assert_eq!(ym(2026, 9).month(), Month::SEPTEMBER);
    assert_eq!("2026-09".parse::<YearMonth>().expect("parse"), ym(2026, 9));
    for raw in [
        "",
        "2026",
        "2026-13",
        "2026-00",
        "2026-9-1",
        "-2026-09",
        "twenty-09",
        "2026/09",
    ] {
        assert!(raw.parse::<YearMonth>().is_err(), "{raw:?} must not parse");
    }
    assert!(YearMonth::new(1899, Month::JANUARY).is_err());
    assert!(YearMonth::new(2201, Month::JANUARY).is_err());
}

#[test]
fn windows_open_on_their_month_and_stay_open() {
    let yearly = Cadence::Yearly {
        month: Month::SEPTEMBER,
    };
    assert_eq!(yearly.window_at_or_before(ym(2026, 12)), ym(2026, 9));
    assert_eq!(yearly.window_at_or_before(ym(2026, 9)), ym(2026, 9));
    assert_eq!(yearly.window_at_or_before(ym(2026, 8)), ym(2025, 9));
    assert_eq!(yearly.next_window_after(ym(2026, 9)), ym(2027, 9));
    assert_eq!(yearly.next_window_after(ym(2026, 8)), ym(2026, 9));

    let semester = Cadence::Semester {
        months: [Month::JANUARY, Month::JULY],
    };
    assert_eq!(semester.window_at_or_before(ym(2026, 3)), ym(2026, 1));
    assert_eq!(semester.window_at_or_before(ym(2026, 6)), ym(2026, 1));
    assert_eq!(semester.window_at_or_before(ym(2026, 7)), ym(2026, 7));
    assert_eq!(semester.next_window_after(ym(2026, 7)), ym(2027, 1));
    assert_eq!(semester.next_window_after(ym(2026, 1)), ym(2026, 7));

    let quarterly = Cadence::Quarterly {
        months: [Month::JANUARY, Month::APRIL, Month::JULY, Month::OCTOBER],
    };
    assert_eq!(quarterly.window_at_or_before(ym(2026, 11)), ym(2026, 10));
    assert_eq!(quarterly.window_at_or_before(ym(2026, 1)), ym(2026, 1));
    assert_eq!(quarterly.next_window_after(ym(2026, 10)), ym(2027, 1));
    assert_eq!(quarterly.next_window_after(ym(2026, 2)), ym(2026, 4));
}

#[test]
fn biennial_windows_follow_the_year_parity() {
    let even = Cadence::Biennial {
        parity: Parity::Even,
    };
    assert_eq!(even.window_at_or_before(ym(2026, 6)), ym(2026, 1));
    assert_eq!(even.window_at_or_before(ym(2025, 6)), ym(2024, 1));
    assert_eq!(even.next_window_after(ym(2025, 6)), ym(2026, 1));
    assert_eq!(even.next_window_after(ym(2026, 6)), ym(2028, 1));

    let odd = Cadence::Biennial {
        parity: Parity::Odd,
    };
    assert_eq!(odd.window_at_or_before(ym(2026, 6)), ym(2025, 1));
    assert_eq!(odd.next_window_after(ym(2026, 1)), ym(2027, 1));
}

#[test]
fn decisions_follow_the_last_recorded_update() {
    let semester = Cadence::Semester {
        months: [Month::JANUARY, Month::JULY],
    };
    assert_eq!(
        decide(semester, None, ym(2026, 3)),
        UpdateDecision::Due {
            reason: DueReason::NeverUpdated
        }
    );
    assert_eq!(
        decide(semester, Some(ym(2026, 1)), ym(2026, 3)),
        UpdateDecision::NotDue { due: ym(2026, 7) }
    );
    assert_eq!(
        decide(semester, Some(ym(2026, 1)), ym(2026, 7)),
        UpdateDecision::Due {
            reason: DueReason::WindowOpened
        }
    );
    assert_eq!(
        decide(semester, Some(ym(2026, 3)), ym(2026, 7)),
        UpdateDecision::Due {
            reason: DueReason::WindowOpened
        }
    );
    assert_eq!(
        decide(semester, Some(ym(2026, 12)), ym(2026, 7)),
        UpdateDecision::NotDue { due: ym(2027, 1) }
    );

    let yearly = Cadence::Yearly {
        month: Month::SEPTEMBER,
    };
    assert_eq!(
        decide(yearly, Some(ym(2025, 9)), ym(2026, 8)),
        UpdateDecision::NotDue { due: ym(2026, 9) }
    );
    assert_eq!(
        decide(yearly, Some(ym(2025, 9)), ym(2026, 9)),
        UpdateDecision::Due {
            reason: DueReason::WindowOpened
        }
    );
}

#[test]
fn due_decisions_and_next_due_dates_always_agree() {
    for cadence in cadences() {
        for last_year in [2023, 2024, 2025, 2026] {
            for last_month in 1..=12 {
                for now_year in [2025, 2026] {
                    for now_month in 1..=12 {
                        let last = Some(ym(last_year, last_month));
                        let now = ym(now_year, now_month);
                        let due = next_due(cadence, last, now);
                        match decide(cadence, last, now) {
                            UpdateDecision::Due { .. } => {
                                assert!(
                                    due <= now,
                                    "{cadence:?} reported due at {now} with a due date of {due}"
                                );
                            }
                            UpdateDecision::NotDue { due: reported } => {
                                assert_eq!(reported, due, "{cadence:?} at {now}");
                                assert!(
                                    due > now,
                                    "{cadence:?} reported not due at {now} with a due date of {due}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn the_ledger_records_the_last_run_per_source() {
    let mut ledger = ScheduleLedger::new();
    assert!(ledger.is_empty());
    assert_eq!(ledger.last(ScheduleSource::Ccd), None);

    ledger.record(ScheduleSource::Ccd, ym(2025, 9));
    ledger.record(ScheduleSource::PrivateAssociation, ym(2026, 1));
    ledger.record(ScheduleSource::Ccd, ym(2026, 9));
    assert_eq!(ledger.len(), 2);
    assert_eq!(ledger.last(ScheduleSource::Ccd), Some(ym(2026, 9)));
    assert_eq!(ledger.last(ScheduleSource::Pss), None);
    let entries = ledger.entries();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries.first(), Some(&(ScheduleSource::Ccd, ym(2026, 9))));
}

#[test]
fn each_published_source_carries_its_own_cadence() {
    assert_eq!(
        ScheduleSource::Ccd.cadence(),
        Some(Cadence::Yearly {
            month: Month::SEPTEMBER
        })
    );
    assert_eq!(
        ScheduleSource::Pss.cadence(),
        Some(Cadence::Biennial {
            parity: Parity::Even
        })
    );
    assert_eq!(
        ScheduleSource::StateEducationAgency.cadence(),
        Some(Cadence::Semester {
            months: [Month::JANUARY, Month::JULY]
        })
    );
    assert_eq!(
        ScheduleSource::PrivateAssociation.cadence(),
        Some(Cadence::Quarterly {
            months: [Month::JANUARY, Month::APRIL, Month::JULY, Month::OCTOBER]
        })
    );
    assert_eq!(ScheduleSource::AthleticAssociation.cadence(), None);
    assert_eq!(ScheduleSource::Ccd.label(), "nces-ccd");
    assert_eq!(ScheduleSource::ALL.len(), 5);
}

#[test]
fn schedule_state_survives_json_round_trips() {
    let month = Month::SEPTEMBER;
    assert_eq!(serde_json::to_string(&month).expect("month json"), "9");
    let year_month = ym(2026, 9);
    assert_eq!(
        serde_json::to_string(&year_month).expect("year month json"),
        "\"2026-09\""
    );
    assert_eq!(
        serde_json::from_str::<YearMonth>("\"2026-09\"").expect("year month parses"),
        year_month
    );
    assert!(serde_json::from_str::<YearMonth>("\"2026-13\"").is_err());
    assert!(serde_json::from_str::<Month>("13").is_err());

    let cadence = Cadence::Quarterly {
        months: [Month::JANUARY, Month::APRIL, Month::JULY, Month::OCTOBER],
    };
    let json = serde_json::to_string(&cadence).expect("cadence json");
    assert_eq!(
        serde_json::from_str::<Cadence>(&json).expect("cadence parses"),
        cadence
    );

    let mut ledger = ScheduleLedger::new();
    ledger.record(ScheduleSource::StateEducationAgency, ym(2026, 7));
    let json = serde_json::to_string(&ledger).expect("ledger json");
    assert_eq!(
        serde_json::from_str::<ScheduleLedger>(&json).expect("ledger parses"),
        ledger
    );
}
