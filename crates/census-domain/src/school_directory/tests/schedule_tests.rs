use super::TestResult;
use crate::school_directory::{
    decide, next_due, Cadence, DueReason, Month, Parity, ScheduleLedger, ScheduleSource,
    UpdateDecision, YearMonth,
};

fn ym(year: i32, month: u8) -> Result<YearMonth, Box<dyn std::error::Error>> {
    Ok(YearMonth::new(year, Month::new(month)?)?)
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
fn months_and_year_months_are_validated_and_rendered() -> TestResult {
    check!(eq; Month::new(12)?.number(), 12);
    check!(eq; Month::new(9)?.to_string(), "09");
    check!(Month::new(0).is_err());
    check!(Month::new(13).is_err());

    check!(eq; ym(2026, 9)?.to_string(), "2026-09");
    check!(eq; ym(2026, 9)?.year(), 2026);
    check!(eq; ym(2026, 9)?.month(), Month::SEPTEMBER);
    check!(eq; "2026-09".parse::<YearMonth>()?, ym(2026, 9)?);
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
        check!(raw.parse::<YearMonth>().is_err(), "{raw:?} must not parse");
    }
    check!(YearMonth::new(1899, Month::JANUARY).is_err());
    check!(YearMonth::new(2201, Month::JANUARY).is_err());
    Ok(())
}

#[test]
fn windows_open_on_their_month_and_stay_open() -> TestResult {
    let yearly = Cadence::Yearly {
        month: Month::SEPTEMBER,
    };
    check!(eq; yearly.window_at_or_before(ym(2026, 12)?), ym(2026, 9)?);
    check!(eq; yearly.window_at_or_before(ym(2026, 9)?), ym(2026, 9)?);
    check!(eq; yearly.window_at_or_before(ym(2026, 8)?), ym(2025, 9)?);
    check!(eq; yearly.next_window_after(ym(2026, 9)?), ym(2027, 9)?);
    check!(eq; yearly.next_window_after(ym(2026, 8)?), ym(2026, 9)?);

    let semester = Cadence::Semester {
        months: [Month::JANUARY, Month::JULY],
    };
    check!(eq; semester.window_at_or_before(ym(2026, 3)?), ym(2026, 1)?);
    check!(eq; semester.window_at_or_before(ym(2026, 6)?), ym(2026, 1)?);
    check!(eq; semester.window_at_or_before(ym(2026, 7)?), ym(2026, 7)?);
    check!(eq; semester.next_window_after(ym(2026, 7)?), ym(2027, 1)?);
    check!(eq; semester.next_window_after(ym(2026, 1)?), ym(2026, 7)?);

    let quarterly = Cadence::Quarterly {
        months: [Month::JANUARY, Month::APRIL, Month::JULY, Month::OCTOBER],
    };
    check!(eq; quarterly.window_at_or_before(ym(2026, 11)?), ym(2026, 10)?);
    check!(eq; quarterly.window_at_or_before(ym(2026, 1)?), ym(2026, 1)?);
    check!(eq; quarterly.next_window_after(ym(2026, 10)?), ym(2027, 1)?);
    check!(eq; quarterly.next_window_after(ym(2026, 2)?), ym(2026, 4)?);
    Ok(())
}

#[test]
fn biennial_windows_follow_the_year_parity() -> TestResult {
    let even = Cadence::Biennial {
        parity: Parity::Even,
    };
    check!(eq; even.window_at_or_before(ym(2026, 6)?), ym(2026, 1)?);
    check!(eq; even.window_at_or_before(ym(2025, 6)?), ym(2024, 1)?);
    check!(eq; even.next_window_after(ym(2025, 6)?), ym(2026, 1)?);
    check!(eq; even.next_window_after(ym(2026, 6)?), ym(2028, 1)?);

    let odd = Cadence::Biennial {
        parity: Parity::Odd,
    };
    check!(eq; odd.window_at_or_before(ym(2026, 6)?), ym(2025, 1)?);
    check!(eq; odd.next_window_after(ym(2026, 1)?), ym(2027, 1)?);
    Ok(())
}

#[test]
fn decisions_follow_the_last_recorded_update() -> TestResult {
    let semester = Cadence::Semester {
        months: [Month::JANUARY, Month::JULY],
    };
    check!(eq; decide(semester, None, ym(2026, 3)?),
    UpdateDecision::Due {
        reason: DueReason::NeverUpdated
    });
    check!(eq; decide(semester, Some(ym(2026, 1)?), ym(2026, 3)?),
    UpdateDecision::NotDue { due: ym(2026, 7)? });
    check!(eq; decide(semester, Some(ym(2026, 1)?), ym(2026, 7)?),
    UpdateDecision::Due {
        reason: DueReason::WindowOpened
    });
    check!(eq; decide(semester, Some(ym(2026, 3)?), ym(2026, 7)?),
    UpdateDecision::Due {
        reason: DueReason::WindowOpened
    });
    check!(eq; decide(semester, Some(ym(2026, 12)?), ym(2026, 7)?),
    UpdateDecision::NotDue { due: ym(2027, 1)? });

    let yearly = Cadence::Yearly {
        month: Month::SEPTEMBER,
    };
    check!(eq; decide(yearly, Some(ym(2025, 9)?), ym(2026, 8)?),
    UpdateDecision::NotDue { due: ym(2026, 9)? });
    check!(eq; decide(yearly, Some(ym(2025, 9)?), ym(2026, 9)?),
    UpdateDecision::Due {
        reason: DueReason::WindowOpened
    });
    Ok(())
}

#[test]
fn due_decisions_and_next_due_dates_always_agree() -> TestResult {
    for cadence in cadences() {
        for last_year in [2023, 2024, 2025, 2026] {
            for last_month in 1..=12 {
                for now_year in [2025, 2026] {
                    for now_month in 1..=12 {
                        let last = Some(ym(last_year, last_month)?);
                        let now = ym(now_year, now_month)?;
                        let due = next_due(cadence, last, now);
                        match decide(cadence, last, now) {
                            UpdateDecision::Due { .. } => {
                                check!(
                                    due <= now,
                                    "{cadence:?} reported due at {now} with a due date of {due}"
                                );
                            }
                            UpdateDecision::NotDue { due: reported } => {
                                check!(eq; reported, due, "{cadence:?} at {now}");
                                check!(due > now,
                                "{cadence:?} reported not due at {now} with a due date of {due}");
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn the_ledger_records_the_last_run_per_source() -> TestResult {
    let mut ledger = ScheduleLedger::new();
    check!(ledger.is_empty());
    check!(eq; ledger.last(ScheduleSource::Ccd), None);

    ledger.record(ScheduleSource::Ccd, ym(2025, 9)?);
    ledger.record(ScheduleSource::PrivateAssociation, ym(2026, 1)?);
    ledger.record(ScheduleSource::Ccd, ym(2026, 9)?);
    check!(eq; ledger.len(), 2);
    check!(eq; ledger.last(ScheduleSource::Ccd), Some(ym(2026, 9)?));
    check!(eq; ledger.last(ScheduleSource::Pss), None);
    let entries = ledger.entries();
    check!(eq; entries.len(), 2);
    check!(eq; entries.first(), Some(&(ScheduleSource::Ccd, ym(2026, 9)?)));
    Ok(())
}

#[test]
fn each_published_source_carries_its_own_cadence() -> Result<(), Box<dyn std::error::Error>> {
    check!(eq; ScheduleSource::Ccd.cadence(),
    Some(Cadence::Yearly {
        month: Month::SEPTEMBER
    }));
    check!(eq; ScheduleSource::Pss.cadence(),
    Some(Cadence::Biennial {
        parity: Parity::Even
    }));
    check!(eq; ScheduleSource::StateEducationAgency.cadence(),
    Some(Cadence::Semester {
        months: [Month::JANUARY, Month::JULY]
    }));
    check!(eq; ScheduleSource::PrivateAssociation.cadence(),
    Some(Cadence::Quarterly {
        months: [Month::JANUARY, Month::APRIL, Month::JULY, Month::OCTOBER]
    }));
    check!(eq; ScheduleSource::AthleticAssociation.cadence(), None);
    check!(eq; ScheduleSource::Ccd.label(), "nces-ccd");
    check!(eq; ScheduleSource::ALL.len(), 5);
    Ok(())
}

#[test]
fn schedule_state_survives_json_round_trips() -> TestResult {
    let month = Month::SEPTEMBER;
    check!(eq; serde_json::to_string(&month)?, "9");
    let year_month = ym(2026, 9)?;
    check!(eq; serde_json::to_string(&year_month)?, "\"2026-09\"");
    check!(eq; serde_json::from_str::<YearMonth>("\"2026-09\"")?,
    year_month);
    check!(serde_json::from_str::<YearMonth>("\"2026-13\"").is_err());
    check!(serde_json::from_str::<Month>("13").is_err());

    let cadence = Cadence::Quarterly {
        months: [Month::JANUARY, Month::APRIL, Month::JULY, Month::OCTOBER],
    };
    let json = serde_json::to_string(&cadence)?;
    check!(eq; serde_json::from_str::<Cadence>(&json)?, cadence);

    let mut ledger = ScheduleLedger::new();
    ledger.record(ScheduleSource::StateEducationAgency, ym(2026, 7)?);
    let json = serde_json::to_string(&ledger)?;
    check!(eq; serde_json::from_str::<ScheduleLedger>(&json)?, ledger);
    Ok(())
}
