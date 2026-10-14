use super::TestResult;
use crate::bests::key::MarkOrdering;
use crate::bests::{should_replace, tie_break_preferred, Measure};

pub(super) mod fixture;
mod selection;
mod workbook;

const ORDERS: [[usize; 4]; 24] = [
    [0, 1, 2, 3],
    [0, 1, 3, 2],
    [0, 2, 1, 3],
    [0, 2, 3, 1],
    [0, 3, 1, 2],
    [0, 3, 2, 1],
    [1, 0, 2, 3],
    [1, 0, 3, 2],
    [1, 2, 0, 3],
    [1, 2, 3, 0],
    [1, 3, 0, 2],
    [1, 3, 2, 0],
    [2, 0, 1, 3],
    [2, 0, 3, 1],
    [2, 1, 0, 3],
    [2, 1, 3, 0],
    [2, 3, 0, 1],
    [2, 3, 1, 0],
    [3, 0, 1, 2],
    [3, 0, 2, 1],
    [3, 1, 0, 2],
    [3, 1, 2, 0],
    [3, 2, 0, 1],
    [3, 2, 1, 0],
];

fn winning_order<'a>(rows: &[MarkOrdering<'a>]) -> TestResult<MarkOrdering<'a>> {
    let (first, rest) = rows.split_first().ok_or("tie candidates missing")?;
    Ok(rest.iter().copied().fold(*first, |incumbent, candidate| {
        if tie_break_preferred(candidate, incumbent) {
            candidate
        } else {
            incumbent
        }
    }))
}

#[test]
fn exact_calendar_dates_are_known_including_gregorian_leap_days() -> TestResult {
    [
        "1900-02-28",
        "2000-02-29",
        "2024-02-29",
        "2025-04-30",
        "2025-12-31",
        "2100-02-28",
        "2400-02-29",
        "0001-01-01",
        "9999-12-31",
    ]
    .into_iter()
    .try_for_each(|date| -> TestResult {
        let known = MarkOrdering::new(date, "z");
        let unknown = MarkOrdering::new("", "a");
        check!(tie_break_preferred(known, unknown), "exact date {date}");
        check!(!tie_break_preferred(unknown, known), "exact date {date}");
        Ok(())
    })
}

#[test]
fn unknown_partial_malformed_and_non_ascii_dates_are_never_inferred() -> TestResult {
    [
        "",
        "2025",
        "0001",
        "9999",
        "2025-02",
        "2025-2-03",
        "2025-02-3",
        "2025/02/03",
        " 2025-02-03",
        "2025-02-03 ",
        "2025-02-03T00:00:00Z",
        "2025-02-03\n",
        "2025-02-03\0",
        "2025-00-01",
        "2025-13-01",
        "2025-01-00",
        "2025-01-32",
        "2025-04-31",
        "2023-02-29",
        "1900-02-29",
        "2100-02-29",
        "2024-02-30",
        "２０２５-02-03",
        "202５-02-03",
        "+2025-02-03",
        "-001-02-03",
    ]
    .into_iter()
    .try_for_each(|date| -> TestResult {
        let candidate = MarkOrdering::new(date, "z");
        let empty = MarkOrdering::new("", "a");
        let known = MarkOrdering::new("2025-02-03", "zz");
        check!(
            !tie_break_preferred(candidate, empty),
            "unknown date {date:?}"
        );
        check!(
            tie_break_preferred(empty, candidate),
            "unknown date {date:?}"
        );
        check!(
            tie_break_preferred(known, candidate),
            "unknown date {date:?}"
        );
        check!(
            !tie_break_preferred(candidate, known),
            "unknown date {date:?}"
        );
        Ok(())
    })
}

#[test]
fn earliest_exact_date_precedes_stable_id_and_same_date_prefers_ascending_id() -> TestResult {
    let earliest = MarkOrdering::new("2024-02-29", "z");
    let later = MarkOrdering::new("2024-03-01", "a");
    check!(tie_break_preferred(earliest, later));
    check!(!tie_break_preferred(later, earliest));
    let smaller = MarkOrdering::new("2024-02-29", "a");
    check!(tie_break_preferred(smaller, earliest));
    check!(!tie_break_preferred(earliest, smaller));
    [earliest, later, smaller, MarkOrdering::new("", "a")]
        .into_iter()
        .try_for_each(|row| -> TestResult {
            check!(!tie_break_preferred(row, row));
            check!(!should_replace(
                10,
                10,
                row,
                row,
                |candidate, incumbent| candidate < incumbent
            ));
            Ok(())
        })
}

#[test]
fn all_unknown_date_spellings_tie_only_by_ascending_stable_id() -> TestResult {
    let spellings = ["", "2025", "2024-02", "not-a-date", "2023-02-29"];
    spellings
        .into_iter()
        .try_for_each(|candidate_date| -> TestResult {
            spellings
                .into_iter()
                .try_for_each(|incumbent_date| -> TestResult {
                    check!(tie_break_preferred(
                        MarkOrdering::new(candidate_date, "a"),
                        MarkOrdering::new(incumbent_date, "z")
                    ));
                    check!(!tie_break_preferred(
                        MarkOrdering::new(candidate_date, "z"),
                        MarkOrdering::new(incumbent_date, "a")
                    ));
                    check!(!tie_break_preferred(
                        MarkOrdering::new(candidate_date, "same"),
                        MarkOrdering::new(incumbent_date, "same")
                    ));
                    Ok(())
                })
        })
}

#[test]
fn tie_preference_is_strict_and_transitive_across_known_and_unknown_dates() -> TestResult {
    let ordered = [
        MarkOrdering::new("2000-02-29", "z"),
        MarkOrdering::new("2000-03-01", "a"),
        MarkOrdering::new("2000-03-01", "z"),
        MarkOrdering::new("2025", "a"),
        MarkOrdering::new("1900-02-29", "z"),
    ];
    ordered
        .iter()
        .copied()
        .enumerate()
        .try_for_each(|(left_index, left)| -> TestResult {
            ordered
                .iter()
                .copied()
                .enumerate()
                .try_for_each(|(right_index, right)| -> TestResult {
                    check!(eq; tie_break_preferred(left, right), left_index < right_index);
                    ordered.iter().copied().try_for_each(|third| -> TestResult {
                        if tie_break_preferred(left, right) && tie_break_preferred(right, third) {
                            check!(tie_break_preferred(left, third));
                        }
                        Ok(())
                    })
                })
        })
}

#[test]
fn all_equal_mark_permutations_and_replays_choose_the_same_date_and_id() -> TestResult {
    let candidates = [
        MarkOrdering::new("2025-06-01", "0"),
        MarkOrdering::new("2025-03-01", "z"),
        MarkOrdering::new("2025-03-01", "a"),
        MarkOrdering::new("2024", "0"),
    ];
    ORDERS.into_iter().try_for_each(|order| -> TestResult {
        let rows = order.into_iter().map(|index| candidates.get(index).copied()
            .ok_or("permutation index escaped candidates")).collect::<Result<Vec<_>, _>>()?;
        let winner = winning_order(&rows)?;
        check!(eq; (winner.date, winner.performance_id), ("2025-03-01", "a"));
        let replayed: Vec<_> = rows.iter().chain(rows.iter().rev()).chain(rows.iter()).copied().collect();
        let replayed_winner = winning_order(&replayed)?;
        check!(eq; (replayed_winner.date, replayed_winner.performance_id), (winner.date, winner.performance_id));
        let numeric = replayed.iter().copied().reduce(|incumbent, candidate| {
            if should_replace(10, 10, candidate, incumbent, |left, right| left < right) {
                candidate
            } else {
                incumbent
            }
        }).ok_or("numeric tie candidates missing")?;
        check!(eq; (numeric.date, numeric.performance_id), (winner.date, winner.performance_id));
        Ok(())
    })
}

#[test]
fn time_distance_and_points_rank_numeric_quality_before_every_tie_preference() -> TestResult {
    [
        (Measure::Time, 10_800_000_000, 10_940_000_000),
        (Measure::Distance, 45_720_000, 44_000_000),
        (Measure::Points, 70_000, 69_000),
    ]
    .into_iter()
    .try_for_each(|(measure, better, worse)| -> TestResult {
        let preferred = MarkOrdering::new("2024-01-01", "a");
        let disfavored = MarkOrdering::new("", "z");
        check!(should_replace(
            better,
            worse,
            disfavored,
            preferred,
            |candidate, incumbent| measure.better(candidate, incumbent)
        ));
        check!(!should_replace(
            worse,
            better,
            preferred,
            disfavored,
            |candidate, incumbent| measure.better(candidate, incumbent)
        ));
        check!(should_replace(
            better,
            better,
            preferred,
            disfavored,
            |candidate, incumbent| measure.better(candidate, incumbent)
        ));
        check!(!should_replace(
            better,
            better,
            disfavored,
            preferred,
            |candidate, incumbent| measure.better(candidate, incumbent)
        ));
        Ok(())
    })
}
