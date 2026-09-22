//! Fixture tests for the tournament decoders. Every fixture is a genuine response copied byte for
//! byte from the lane's captures; each constant names the URL, the capture and the measured row
//! counts the test pins.
use super::parse::{
    class_token, date_part, event_date_range, finisher_grade, member_grade, newest_term,
    parse_error, parse_events, parse_grade, parse_mark, parse_meets, parse_qualifiers,
    parse_summary,
};
use super::wire::{EventSummary, QualifierAthlete, QualifiersEnvelope};
use census_domain::model::{Grade, Mark};

/// Provenance: `GET https://api.ihsa.org/v1/track-field/meets`, fetched 2026-09-19 23:15.
/// Capture: `tools/a13-ihsa/p_track-field_meets.json` (1,088 B). Measured: `count: 2`.
const FIXTURE_MEETS: &str =
    include_str!("../../../../tests/fixtures/ihsa_tournament/track_field_meets.json");

/// Provenance: `GET https://api.ihsa.org/v1/track-field/meets/2026/events?gender=Boys`,
/// fetched 2026-09-19 23:15. Capture: `tools/a13-ihsa/track-field_meets_2026_events_gender_Boys.json`
/// (200,022 B). Measured: `meetId: 74003`, `count: 97`.
const FIXTURE_EVENTS: &str =
    include_str!("../../../../tests/fixtures/ihsa_tournament/track_field_2026_boys_events.json");

/// Provenance: `GET https://api.ihsa.org/v1/track-field/events/2790204/summary`,
/// fetched 2026-09-19 23:15. Capture: `tools/a13-ihsa/tf_event_2790204.json` (110,259 B).
/// Measured: Boys High Jump 1A final, 20 finishers, each with `athlete.athleticNetId`.
const FIXTURE_HJ: &str =
    include_str!("../../../../tests/fixtures/ihsa_tournament/event_2790204_boys_hj_1a_finals.json");

/// Provenance: `GET https://api.ihsa.org/v1/track-field/events/500937/summary`,
/// fetched 2026-09-19 23:17. Capture: `tools/a13-ihsa/relay_500937.json` (148,085 B).
/// Measured: Boys 4x800m Relay 1A final, 12 teams, 4 legs each (48 athlete id sets).
const FIXTURE_RELAY: &str = include_str!(
    "../../../../tests/fixtures/ihsa_tournament/event_500937_boys_4x800_1a_finals.json"
);

/// Provenance: `GET https://api.ihsa.org/v1/2025-26/statefinal/cc-qualifiers?tournamentId=688`,
/// fetched 2026-09-19 23:16. Capture: `tools/a13-ihsa/ccq_2025_26.json` (87,567 B).
/// Measured: boys 1A - 398 box rows, 30 team qualifiers, 42 individual-qualifier schools.
const FIXTURE_XC_BOYS_1A: &str =
    include_str!("../../../../tests/fixtures/ihsa_tournament/cc_qualifiers_2025-26_688.json");

/// Provenance: `GET .../cc-qualifiers?tournamentId=689`, fetched 2026-09-19 23:20.
/// Capture: `tools/a13-ihsa/ccq_2025_26_689.json` (87,111 B). Measured: boys 2A, 396 athletes.
const FIXTURE_XC_BOYS_2A: &str =
    include_str!("../../../../tests/fixtures/ihsa_tournament/cc_qualifiers_2025-26_689.json");

/// Provenance: `GET .../cc-qualifiers?tournamentId=690`, fetched 2026-09-19 23:20.
/// Capture: `tools/a13-ihsa/ccq_2025_26_690.json` (93,690 B). Measured: boys 3A, 420 athletes.
const FIXTURE_XC_BOYS_3A: &str =
    include_str!("../../../../tests/fixtures/ihsa_tournament/cc_qualifiers_2025-26_690.json");

/// Provenance: `GET .../cc-qualifiers?tournamentId=691`, recorded 2026-09-20 09:06.
/// Capture: `research/midwest/evidence/gaps/38/cc-qualifiers-2025-26-691.json` (79,632 B).
/// Measured: girls 1A, 357 athletes.
const FIXTURE_XC_GIRLS_1A: &str =
    include_str!("../../../../tests/fixtures/ihsa_tournament/cc_qualifiers_2025-26_691.json");

/// Provenance: `GET .../cc-qualifiers?tournamentId=688` for term 2024-25, fetched 2026-09-19 23:18.
/// Capture: `tools/a13-ihsa/ccq_2024_25.json` (88 B). Measured: the archive holds an empty envelope
/// for that tournament id - the term predates the lists, the id itself still answers.
const FIXTURE_XC_EMPTY: &str =
    include_str!("../../../../tests/fixtures/ihsa_tournament/cc_qualifiers_2024-25_688.json");

/// Provenance: `GET .../cc-qualifiers?tournamentId=691` for term 2026-27, fetched 2026-09-19 23:19.
/// Capture: `tools/a13-ihsa/ccq_2026_27.json` (50 B). Measured: the archive answers an error body.
const FIXTURE_XC_ERROR: &str =
    include_str!("../../../../tests/fixtures/ihsa_tournament/cc_qualifiers_archive_error.json");

/// Provenance: `GET https://api.ihsa.org/v1/terms`, fetched 2026-09-19 23:16.
/// Capture: `tools/a13-ihsa/terms.json` (131 B). Measured: `currentTerm` 2026-27, newest term 2025-26.
const FIXTURE_TERMS: &str = include_str!("../../../../tests/fixtures/ihsa_tournament/terms.json");

// ── Meet and event indexes ─────────────────────────────────────────────

#[test]
fn meets_index_pins_both_state_finals_and_their_athletic_net_live_ids() {
    let meets = parse_meets(FIXTURE_MEETS).expect("fixture must parse");

    assert_eq!(meets.len(), 2, "the T&F API holds exactly two meets");
    let boys = &meets[0];
    assert_eq!(boys.meet_id, 74003, "= live.athletic.net/meets/74003");
    assert_eq!(boys.year, "2026");
    assert_eq!(boys.title, "2026 IHSA Boys State Track & Field");
    assert_eq!(boys.gender, "Boys");
    assert_eq!(
        boys.last_refreshed_at.as_deref(),
        Some("2026-05-31T17:51:14.655Z"),
        "the coarse change signal the journal keys on"
    );

    let girls = &meets[1];
    assert_eq!(girls.meet_id, 74002, "= live.athletic.net/meets/74002");
    assert_eq!(girls.gender, "Girls");
    assert_eq!(girls.title, "2026 IHSA Girls State Track & Field");
}

#[test]
fn events_index_pins_97_event_rows_with_round_and_class_splits() {
    let envelope = parse_events(FIXTURE_EVENTS).expect("fixture must parse");
    assert_eq!(envelope.meet_id, 74003);
    assert_eq!(envelope.count, 97);
    assert_eq!(envelope.data.len(), 97);

    let mut ids = std::collections::BTreeSet::new();
    let mut prelims = 0;
    let mut finals = 0;
    let mut unsuffixed = 0;
    let mut classes = std::collections::BTreeMap::new();
    let mut relays = 0;
    for row in &envelope.data {
        assert!(ids.insert(row.event_id.clone()), "event ids are unique");
        assert_eq!(row.gender, "M", "the boys index publishes only M rows");
        assert_eq!(row.status.as_deref(), Some("final"));
        assert!(row.has_results, "every captured event has results");
        assert!(row.metadata_fetched_at.is_some());
        *classes.entry(row.class_division.clone()).or_insert(0usize) += 1;
        if row.event_type == "RelayEvent" {
            relays += 1;
        }
        match row.round.as_deref() {
            Some("P") => prelims += 1,
            Some("F") => finals += 1,
            _ => unsuffixed += 1,
        }
    }
    assert_eq!(ids.len(), 97);
    assert_eq!((prelims, finals, unsuffixed), (39, 39, 19));
    assert_eq!(classes.get("1A"), Some(&31));
    assert_eq!(classes.get("2A"), Some(&31));
    assert_eq!(classes.get("3A"), Some(&31));
    assert_eq!(classes.get("WD"), Some(&4));
    assert_eq!(relays, 24, "8 relay events per class");

    let range = event_date_range(&envelope.data);
    assert_eq!(
        (range.0.as_deref(), range.1.as_deref()),
        (Some("2026-05-28"), Some("2026-05-30"))
    );
}

// ── Individual event summary ───────────────────────────────────────────

/// The published grade distribution of the captured 1A high jump final.
fn hj_grade_counts(finishers: &[super::wire::FinisherRow]) -> Vec<(u8, usize)> {
    let mut counts: std::collections::BTreeMap<u8, usize> = std::collections::BTreeMap::new();
    for row in finishers {
        if let Some(grade) = finisher_grade(row) {
            *counts.entry(grade.get()).or_insert(0) += 1;
        }
    }
    counts.into_iter().collect()
}

#[test]
fn hj_summary_pins_net_and_live_ids_on_every_finisher() {
    let summary = parse_summary(FIXTURE_HJ).expect("fixture must parse");
    assert_eq!(summary.event_id, "2790204");
    assert_eq!(summary.meet_id, 74003);
    assert_eq!(summary.class_division, "1A");
    assert_eq!(summary.event_type, "IndividualEvent");
    assert_eq!(summary.finishers.len(), 20);

    for row in &summary.finishers {
        let athlete = row
            .athlete
            .as_ref()
            .expect("individual rows publish athlete");
        assert!(athlete.athletic_net_id.is_some(), "Athletic.net AthleteID");
        assert!(athlete.athletic_live_id.is_some(), "Athletic.net Live id");
        assert!(
            row.ihsa_school_id.is_some(),
            "the school resolution channel"
        );
        assert!(row
            .team
            .as_ref()
            .is_some_and(|team| team.athletic_net_id.is_some()));
        assert!(row.mark.is_some());
        assert!(finisher_grade(row).is_some());
        assert_eq!(
            row.year, athlete.year,
            "the row grade and the nested athlete grade agree"
        );
    }

    let winner = &summary.finishers[0];
    assert_eq!(winner.place, Some(1));
    assert_eq!(winner.athlete_name.as_deref(), Some("Kehlin Crawford"));
    assert_eq!(winner.ihsa_school_id.as_deref(), Some("0611"));
    assert_eq!(
        winner.mark.as_deref().and_then(parse_mark),
        Some(Mark::DistanceMetres(2.02))
    );
    let athlete = winner.athlete.as_ref().expect("checked above");
    assert_eq!(athlete.athletic_net_id, Some(27_740_691));
    assert_eq!(athlete.athletic_live_id, Some(49_752_378));
    assert_eq!(athlete.year.as_deref(), Some("11"));
    assert_eq!(
        winner.team.as_ref().and_then(|t| t.athletic_net_id),
        Some(16_352)
    );

    assert_eq!(
        hj_grade_counts(&summary.finishers),
        vec![(9, 1), (10, 5), (11, 10), (12, 4)],
        "the measured grade distribution of the captured final"
    );
}

#[test]
fn summary_round_label_is_derived_from_the_published_code() {
    let hj = parse_summary(FIXTURE_HJ).expect("fixture must parse");
    assert_eq!(hj.round.as_deref(), Some("F"));
    assert_eq!(hj.round_label.as_deref(), Some("Finals"));
    assert_eq!(
        super::parse::round_label(hj.round.as_deref()),
        Some("Finals")
    );
    assert_eq!(
        hj.round_label.as_deref(),
        super::parse::round_label(hj.round.as_deref()),
        "the derivation and the published label agree, so the index path needs no label"
    );
}

// ── Relay event summary ────────────────────────────────────────────────

#[test]
fn relay_summary_pins_four_legs_per_team_with_ids_and_grades() {
    let summary = parse_summary(FIXTURE_RELAY).expect("fixture must parse");
    assert_eq!(summary.event_id, "500937");
    assert_eq!(summary.event_type, "RelayEvent");
    assert_eq!(summary.finishers.len(), 12, "12 teams in the 1A final");

    let mut legs = 0;
    for row in &summary.finishers {
        assert!(row.athlete.is_none(), "relay rows carry no single athlete");
        assert!(row.year.is_none(), "relay rows carry no row-level grade");
        assert_eq!(row.members.len(), 4, "a 4x800m team fields four legs");
        for member in &row.members {
            let athlete = member.athlete.as_ref().expect("legs publish athlete");
            assert!(athlete.athletic_net_id.is_some());
            assert!(athlete.athletic_live_id.is_some());
            assert!(member_grade(member).is_some());
            legs += 1;
        }
    }
    assert_eq!(legs, 48, "12 teams x 4 legs");

    let winner = &summary.finishers[0];
    assert_eq!(winner.ihsa_school_id.as_deref(), Some("1835"));
    assert_eq!(
        winner.mark.as_deref().and_then(parse_mark),
        Some(Mark::TimeSeconds(471.37)),
        "7:51.37 in seconds"
    );
    assert_eq!(
        winner.team.as_ref().and_then(|t| t.athletic_net_id),
        Some(16_665)
    );
    let lead = winner.members[0].athlete.as_ref().expect("leg 1");
    assert_eq!(lead.name.as_deref(), Some("Joel White"));
    assert_eq!(lead.athletic_net_id, Some(20_992_451));
    assert_eq!(lead.year.as_deref(), Some("12"));
}

// ── Cross-country qualifier lists ──────────────────────────────────────

fn athletes(envelope: &QualifiersEnvelope) -> Vec<&QualifierAthlete> {
    envelope
        .team_qualifiers
        .iter()
        .chain(envelope.individual_qualifiers.iter())
        .flat_map(|team| team.athletes.iter())
        .collect()
}

fn grade_eleven(envelope: &QualifiersEnvelope) -> usize {
    athletes(envelope)
        .into_iter()
        .filter(|athlete| parse_grade(athlete.year_in_school.as_deref()) == Grade::new(11))
        .count()
}

fn graded(envelope: &QualifiersEnvelope) -> usize {
    athletes(envelope)
        .into_iter()
        .filter(|athlete| parse_grade(athlete.year_in_school.as_deref()).is_some())
        .count()
}

#[test]
fn xc_boys_1a_pins_ids_grades_and_sector_equality() {
    let envelope = parse_qualifiers(FIXTURE_XC_BOYS_1A).expect("fixture must parse");
    assert_eq!(envelope.tournament_id, "688");
    assert_eq!(envelope.box_assignments.len(), 398);
    assert_eq!(envelope.team_qualifiers.len(), 30);
    assert_eq!(envelope.individual_qualifiers.len(), 42);

    let team_legs: usize = envelope
        .team_qualifiers
        .iter()
        .map(|t| t.athletes.len())
        .sum();
    let individual_legs: usize = envelope
        .individual_qualifiers
        .iter()
        .map(|t| t.athletes.len())
        .sum();
    assert_eq!((team_legs, individual_legs), (348, 50));

    // The sector list carries one row per qualifying athlete and no grade; the graded lists cover it
    // exactly, in both totals and entry type.
    let sectors: usize = envelope.box_assignments.len();
    assert_eq!(
        sectors,
        team_legs + individual_legs,
        "no athlete is lost by skipping boxes"
    );
    let typed = |kind: &str| {
        envelope
            .box_assignments
            .iter()
            .filter(|row| row.entry_type.as_deref() == Some(kind))
            .count()
    };
    assert_eq!(typed("T"), team_legs);
    assert_eq!(typed("I"), individual_legs);

    for team in envelope
        .team_qualifiers
        .iter()
        .chain(envelope.individual_qualifiers.iter())
    {
        assert!(
            team.ihsa_school_id.is_some(),
            "every qualifier school publishes its own id"
        );
        assert!(team
            .school_name
            .as_ref()
            .is_some_and(|name| !name.is_empty()));
    }

    assert_eq!(athletes(&envelope).len(), 398);
    assert_eq!(
        graded(&envelope),
        397,
        "one captured row publishes no grade"
    );
    assert_eq!(grade_eleven(&envelope), 104);
}

#[test]
fn xc_boys_three_classes_total_the_measured_1214_athletes_and_358_juniors() {
    let one_a = parse_qualifiers(FIXTURE_XC_BOYS_1A).expect("fixture must parse");
    let two_a = parse_qualifiers(FIXTURE_XC_BOYS_2A).expect("fixture must parse");
    let three_a = parse_qualifiers(FIXTURE_XC_BOYS_3A).expect("fixture must parse");
    assert_eq!(
        (
            one_a.tournament_id.as_str(),
            two_a.tournament_id.as_str(),
            three_a.tournament_id.as_str()
        ),
        ("688", "689", "690")
    );

    assert_eq!(athletes(&two_a).len(), 396);
    assert_eq!(athletes(&three_a).len(), 420);
    assert_eq!(
        athletes(&one_a).len() + athletes(&two_a).len() + athletes(&three_a).len(),
        1_214,
        "the report's measured boys qualifier total"
    );
    assert_eq!(
        grade_eleven(&one_a) + grade_eleven(&two_a) + grade_eleven(&three_a),
        358,
        "the report's measured grade-11 slice of the boys qualifiers"
    );
}

#[test]
fn xc_girls_list_publishes_the_same_shape() {
    let envelope = parse_qualifiers(FIXTURE_XC_GIRLS_1A).expect("fixture must parse");
    assert_eq!(envelope.tournament_id, "691");
    assert_eq!(athletes(&envelope).len(), 357);
    assert_eq!(
        graded(&envelope),
        356,
        "one captured row publishes no grade"
    );
    assert_eq!(grade_eleven(&envelope), 97);
    assert_eq!(envelope.box_assignments.len(), 357);
}

#[test]
fn xc_empty_archive_decodes_to_an_empty_envelope() {
    let envelope = parse_qualifiers(FIXTURE_XC_EMPTY).expect("fixture must parse");
    assert_eq!(envelope.tournament_id, "688");
    assert!(envelope.box_assignments.is_empty());
    assert!(envelope.team_qualifiers.is_empty());
    assert!(envelope.individual_qualifiers.is_empty());
    assert_eq!(athletes(&envelope).len(), 0);
}

#[test]
fn xc_missing_archive_is_an_error_envelope_not_a_qualifier_payload() {
    assert_eq!(
        parse_error(FIXTURE_XC_ERROR).as_deref(),
        Some("Archive not available for term 2026-27")
    );
    assert!(
        parse_qualifiers(FIXTURE_XC_ERROR).is_err(),
        "the error body must not decode as a qualifier list"
    );
}

// ── Terms ──────────────────────────────────────────────────────────────

#[test]
fn terms_pin_the_newest_completed_school_year() {
    let envelope = super::parse::parse_terms(FIXTURE_TERMS).expect("fixture must parse");
    assert_eq!(envelope.current_term, "2026-27");
    assert_eq!(envelope.terms.len(), 3);
    assert_eq!(
        newest_term(FIXTURE_TERMS)
            .expect("fixture must parse")
            .as_deref(),
        Some("2025-26")
    );
}

// ── Field parsers ──────────────────────────────────────────────────────

#[test]
fn mark_forms_seen_in_the_corpus_parse_to_canonical_marks() {
    assert_eq!(parse_mark("2.02m"), Some(Mark::DistanceMetres(2.02)));
    assert_eq!(parse_mark("1.88mq"), Some(Mark::DistanceMetres(1.88)));
    assert_eq!(parse_mark("7:51.37"), Some(Mark::TimeSeconds(471.37)));
    assert_eq!(parse_mark("10.94"), Some(Mark::TimeSeconds(10.94)));
    assert_eq!(parse_mark("10.94Q"), Some(Mark::TimeSeconds(10.94)));
    assert_eq!(parse_mark("  2.02m "), Some(Mark::DistanceMetres(2.02)));

    assert_eq!(parse_mark(""), None, "an absent mark is never invented");
    assert_eq!(parse_mark("NH"), None, "no height");
    assert_eq!(parse_mark("NM"), None, "no mark");
    assert_eq!(parse_mark("FOUL"), None);
    assert_eq!(parse_mark("DNF"), None);
}

/// Every `mark` string anywhere in a real summary payload is either parsed or one of the alphabetic
/// no-mark tokens - the parser has no unhandled numeric form in the captured corpus.
///
/// `expects_no_mark_token` records what the payload itself publishes: the high jump summary's
/// `relatedRounds` carry the prelims' `"NH"` rows, while the 4x800m relay summary publishes none.
fn assert_mark_corpus(name: &str, body: &str, minimum: usize, expects_no_mark_token: bool) {
    fn walk(value: &serde_json::Value, out: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, child) in map {
                    if key == "mark" {
                        if let serde_json::Value::String(text) = child {
                            out.push(text.clone());
                        }
                    }
                    walk(child, out);
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    walk(item, out);
                }
            }
            _ => {}
        }
    }

    let document: serde_json::Value = serde_json::from_str(body).expect("fixture must parse");
    let mut marks = Vec::new();
    walk(&document, &mut marks);
    assert!(
        marks.len() >= minimum,
        "{name}: expected at least {minimum} marks, saw {}",
        marks.len()
    );

    let mut refused = 0;
    for mark in &marks {
        match parse_mark(mark) {
            Some(_) => {}
            None => {
                refused += 1;
                assert!(
                    mark.is_empty() || mark.chars().all(|c| c.is_ascii_alphabetic()),
                    "{name}: unhandled mark form {mark:?}"
                );
            }
        }
    }
    assert!(
        refused > 0 || !expects_no_mark_token,
        "{name}: the corpus publishes no-mark tokens (NH/NM) in its unread prelim rounds"
    );
}

#[test]
fn summaries_contain_no_unhandled_mark_form() {
    assert_mark_corpus("high jump 1A final", FIXTURE_HJ, 20, true);
    assert_mark_corpus("4x800m relay 1A final", FIXTURE_RELAY, 12, false);
}

#[test]
fn date_and_class_helpers_read_what_the_index_publishes() {
    assert_eq!(date_part("2026-05-28T00:00:00.000Z"), Some("2026-05-28"));
    assert_eq!(
        date_part("2026-05-28"),
        None,
        "a bare date has no T to split on"
    );
    assert_eq!(class_token("1A"), Some("1A"));
    assert_eq!(class_token("WD"), Some("WD"));
    assert_eq!(class_token("  "), None);
}

#[test]
fn grade_parser_rejects_what_is_not_a_high_school_grade() {
    assert_eq!(parse_grade(Some("11")), Grade::new(11));
    assert_eq!(parse_grade(Some("9")), Grade::new(9));
    assert_eq!(parse_grade(Some("12")), Grade::new(12));
    assert_eq!(parse_grade(Some("")), None);
    assert_eq!(parse_grade(Some("13")), None);
    assert_eq!(parse_grade(Some("0")), None);
    assert_eq!(parse_grade(Some("Fr.")), None);
    assert_eq!(parse_grade(None), None);
}

/// One summary's published title is used to sanity-check that the two fixtures are different events.
#[test]
fn captain_summaries_are_distinct_events() {
    let hj: EventSummary = parse_summary(FIXTURE_HJ).expect("fixture must parse");
    let relay: EventSummary = parse_summary(FIXTURE_RELAY).expect("fixture must parse");
    assert_ne!(hj.event_id, relay.event_id);
    assert!(hj.event_name.contains("High Jump"));
    assert!(relay.event_name.contains("4x800m Relay"));
}
