//! The whole-meet walk, over the anonymous probe capture of meet 634313
//! (`research/sources/athleticnet/samples/anon-*.json`, copied byte-for-byte into
//! `tests/fixtures/athleticnet/`).
//!
//! Every number asserted here was measured from that capture, not chosen: 49 blocks, 758 published
//! rows of which 72 are relay squads, 288 relay legs, a 12-entry team list naming 10 schools, and a
//! metadata document declaring 36 events (24 track, 12 field, 4 of them hurdles).

use super::super::map::{Accumulator, Stats};
use super::super::parse::gender_of;
use super::super::{
    grade_of, jurisdiction_of, AllResults, EventDivisions, EventMetadata, MeetData,
};
use super::map::absorb_meet;
use crate::school_index::SchoolIndex;
use census_domain::model::{
    CompetitionLevel, EvidenceMethod, Gender, Mark, PerformanceId, SchoolYear, SourceNamespace,
    SourceRef, Sport, TimingMethod,
};
use census_domain::UsJurisdiction;
use std::collections::{BTreeMap, BTreeSet, HashMap};

const MEET_DATA: &str =
    include_str!("../../../../tests/fixtures/athleticnet/meet_634313_meetdata.json");
const ALL_RESULTS: &str =
    include_str!("../../../../tests/fixtures/athleticnet/meet_634313_allresults.json");
const EVENT_DIV: &str =
    include_str!("../../../../tests/fixtures/athleticnet/meet_634313_eventdiv.json");
/// The one derived document in the fixture set: the genuine meet capture with `Location.State`
/// dropped, so the refusal branch for a meet the payload does not place is reachable at all. The
/// results document is the genuine one and is never read on this path.
const MEET_DATA_WITHOUT_STATE: &str = include_str!(
    "../../../../tests/fixtures/athleticnet/meet_634313_meetdata_nostate.derived.json"
);
const OBSERVED_ON: &str = "2026-09-22";

/// What the walk made of the capture.
struct Walk {
    meet: MeetData,
    counts: super::count::MeetStats,
    accumulated: Accumulator,
    stats: Stats,
}

fn walk(metadata: bool) -> Walk {
    let meet: MeetData = serde_json::from_str(MEET_DATA).expect("the meet document decodes");
    let results: AllResults =
        serde_json::from_str(ALL_RESULTS).expect("the results document decodes");
    let document: EventDivisions =
        serde_json::from_str(EVENT_DIV).expect("the metadata document decodes");
    let metadata = metadata.then(|| EventMetadata::new(&document));
    absorb(&meet, &results, metadata.as_ref())
}

fn absorb(meet: &MeetData, results: &AllResults, metadata: Option<&EventMetadata>) -> Walk {
    let mut accumulated = Accumulator::default();
    let mut stats = Stats::default();
    let source = SourceRef::new("athleticnet", None);
    let (rows, counts) = absorb_meet(
        meet,
        results,
        metadata,
        &source,
        OBSERVED_ON,
        &SchoolIndex::from_schools(&[]),
        &mut HashMap::new(),
        &mut stats,
        &mut accumulated,
    );
    assert_eq!(rows, counts.stored(), "the walk reports the rows it stored");
    Walk {
        meet: meet.clone(),
        counts,
        accumulated,
        stats,
    }
}

/// Every performance the walk stored, in a stable order.
fn performances(walk: &Walk) -> Vec<&census_domain::model::CanonicalPerformance> {
    let mut rows: Vec<_> = walk.accumulated.performances.values().collect();
    rows.sort_by(|left, right| left.source_key.cmp(&right.source_key));
    rows
}

#[test]
fn the_probe_meet_reads_every_published_row() {
    let walk = walk(false);
    assert_eq!(walk.meet.meet.id, 634313);
    assert_eq!(walk.meet.meet.name, "Big 8 Conference");
    assert_eq!(walk.meet.meet.season_id, Some(2026));
    assert_eq!(walk.meet.sport2.as_deref(), Some("tfo"), "an outdoor meet");
    assert_eq!(
        walk.meet.divisions.len(),
        2,
        "`tfDivisions` names both divisions the payload declares"
    );
    assert_eq!(walk.counts.blocks, 49, "49 blocks: {}", walk.counts);
    assert_eq!(walk.counts.rows_seen, 758, "758 published rows");
    assert_eq!(walk.counts.relay_rows, 72, "72 of them are relay squads");
    assert_eq!(walk.counts.legs_seen, 288, "288 relay legs");
    assert_eq!(walk.counts.meets_pulled, 1);
    assert_eq!(walk.counts.meets_unplaced, 0);
    assert_eq!(walk.counts.meets_without_date, 0);
    assert_eq!(walk.counts.meets_without_season, 0);
    assert_eq!(
        walk.counts.blocks_without_division, 0,
        "every block publishes its division"
    );
    assert_eq!(
        walk.counts.blocks_gender_unknown, 0,
        "every block publishes M or F"
    );
    assert_eq!(
        walk.counts.rows_unknown_school, 0,
        "every row's TeamID resolves through the payload's own team list"
    );
    assert_eq!(
        walk.counts.relay_rows_without_legs, 0,
        "every squad row has legs keyed to it by ResultID"
    );
    assert_eq!(walk.counts.legs_no_name, 0, "every leg publishes a name");
    assert_eq!(
        walk.counts.legs_no_athlete, 0,
        "every leg publishes an athlete id"
    );
    assert_eq!(
        walk.counts.legs_no_grade, 0,
        "every leg's ShortDesc is a real grade"
    );
    assert_eq!(
        walk.counts.rows_stored, 623,
        "758 published rows minus the 72 relay squads minus the 63 individual rows whose only \
         mark is a no-mark word"
    );
    assert_eq!(
        walk.counts.rows_no_mark, 65,
        "63 individual rows and the two squads whose relay was a `DNS` and a `DQ`"
    );
    assert_eq!(
        walk.counts.legs_stored, 280,
        "288 published legs minus the 8 belonging to the two no-mark squads"
    );
    assert_eq!(walk.counts.stored(), 903);
    assert_eq!(walk.stats.fetches_failed, 0);
    assert_eq!(walk.counts.rows_no_grade, 0);
    assert_eq!(walk.counts.rows_no_name, 0);
    assert_eq!(walk.counts.rows_no_athlete, 0);
    assert_eq!(
        walk.counts.rows_unmapped_event, 0,
        "no row needed the metadata document to read its mark"
    );
}

#[test]
fn the_meet_row_carries_what_the_document_publishes_and_nothing_it_does_not() {
    let walk = walk(false);
    assert_eq!(walk.accumulated.meets.len(), 1);
    assert_eq!(walk.accumulated.schools.len(), 10);
    assert_eq!(walk.accumulated.athletes.len(), 490);
    assert_eq!(
        walk.accumulated.events.len(),
        49,
        "one event per block: this capture publishes prelims and finals of the same event four \
         times, and the round is part of the event identity"
    );
    let row = walk
        .accumulated
        .meets
        .values()
        .next()
        .expect("one meet row");
    assert_eq!(row.state, Some(UsJurisdiction::Wisconsin));
    assert_eq!(row.date, "2026-05-15");
    assert_eq!(
        row.end_date, None,
        "`EndDate` repeats the start date on this capture, so it is not published as a range"
    );
    assert_eq!(row.location.as_deref(), Some("Sun Prairie HS"));
    assert_eq!(row.sports, vec![Sport::OutdoorTrack]);
    assert_eq!(
        row.level,
        CompetitionLevel::Unknown,
        "the level stays what the bio path mints for the same meet: no capture maps the payload's \
         `LevelMask` onto the platform's levels"
    );
    assert_eq!(
        row.source_urls,
        vec!["https://www.athletic.net/TrackAndField/meet/634313/info".to_string()]
    );
    let identities: BTreeMap<&str, &str> = row
        .source_identities
        .iter()
        .map(|identity| {
            let kind = match &identity.namespace {
                SourceNamespace::AthleticNet { kind } => kind.as_str(),
                other => panic!("meet identity is not under this adapter's namespace: {other:?}"),
            };
            (kind, identity.id.as_str())
        })
        .collect();
    assert_eq!(
        identities.get("meet"),
        Some(&"634313"),
        "the published meet id"
    );
    assert_eq!(
        identities.get("live"),
        Some(&"73767"),
        "`LiveID` is persisted for the AthleticLIVE join"
    );
}

#[test]
fn every_published_team_id_mints_a_school_and_a_team() {
    let walk = walk(false);
    assert_eq!(
        walk.accumulated.schools.len(),
        10,
        "the capture's rows name 10 distinct TeamIDs"
    );
    assert_eq!(
        walk.accumulated.teams.len(),
        20,
        "10 schools, each with the boys and the girls team this meet published"
    );
    let team_ids: BTreeSet<&str> = walk
        .accumulated
        .teams
        .values()
        .map(|team| team.id.as_str())
        .collect();
    let performance_teams: BTreeSet<&str> = walk
        .accumulated
        .performances
        .values()
        .map(|row| row.team.as_str())
        .collect();
    assert_eq!(
        team_ids, performance_teams,
        "every performance hangs off a minted team"
    );
    assert_eq!(
        walk.stats.rows_unknown_school, 0,
        "no row fell through the school reader"
    );
}

#[test]
fn a_relay_squad_becomes_one_performance_per_leg_and_never_a_person() {
    let walk = walk(false);
    let rows = performances(&walk);
    let legs: Vec<_> = rows
        .iter()
        .filter(|row| leg_position(row).is_some())
        .collect();
    assert_eq!(
        legs.len(),
        280,
        "one performance per leg of a squad whose mark was readable"
    );
    assert_eq!(rows.len(), 903, "623 individual results plus 280 legs");
    assert!(
        legs.iter().all(|row| !row.source_key.contains("-0")),
        "a squad's own row is not a performance"
    );
    let positions: BTreeSet<u8> = legs.iter().filter_map(|row| leg_position(row)).collect();
    assert_eq!(
        positions,
        BTreeSet::from([1, 2, 3, 4]),
        "the leg's 1-based position in its squad is its published order"
    );
    for row in &legs {
        let note = row
            .evidence
            .first()
            .and_then(|evidence| evidence.note.as_deref());
        assert!(
            note.is_some_and(|note| note.starts_with("relay leg ")),
            "{}: a leg carries its own evidence note: {note:?}",
            row.source_key
        );
    }
    assert_eq!(
        walk.counts.rows_no_grade, 0,
        "the 71 squad rows publishing `-` never reach the grade reader: their legs are routed to \
         the relay reader by ResultID membership"
    );
    let names: BTreeSet<&str> = walk
        .accumulated
        .athletes
        .values()
        .map(|athlete| athlete.canonical_name.as_str())
        .collect();
    assert!(
        names.iter().all(|name| !name.contains("<BR>")),
        "a squad's `<BR>`-joined members are never minted as one athlete"
    );
    assert!(
        rows.iter().all(|row| !row.source_key.contains("<BR>")),
        "no performance is keyed on the squad's padded name"
    );
}

/// The leg position a performance's own evidence records, when it is a relay leg.
fn leg_position(row: &census_domain::model::CanonicalPerformance) -> Option<u8> {
    let note = row.evidence.first()?.note.as_deref()?;
    note.strip_prefix("relay leg ")?
        .split(';')
        .next()?
        .trim()
        .parse()
        .ok()
}

#[test]
fn grades_are_read_only_where_the_payload_publishes_one() {
    assert_eq!(grade_of(Some("9")).map(|grade| grade.get()), Some(9));
    assert_eq!(grade_of(Some("12")).map(|grade| grade.get()), Some(12));
    assert_eq!(
        grade_of(Some("99")),
        None,
        "the placeholder the rankings document publishes on masked rows (`blurred: true`, \
         `AthleteID: 0`, 96 rows on the anonymous capture) is never a grade"
    );
    assert_eq!(
        grade_of(Some("-")),
        None,
        "the placeholder this meet's 71 squad rows publish is never a grade"
    );
    assert_eq!(grade_of(Some("")), None);
    assert_eq!(grade_of(None), None);
    let walk = walk(false);
    let grades: BTreeSet<u8> = performances(&walk)
        .iter()
        .filter_map(|row| row.observed_grade.map(|grade| grade.get()))
        .collect();
    assert_eq!(
        grades,
        BTreeSet::from([9, 10, 11, 12]),
        "all four classes compete in this meet, and no row stores a placeholder"
    );
    assert_eq!(
        performances(&walk)
            .iter()
            .filter(|row| row.observed_grade.is_none())
            .count(),
        0,
        "every stored row published a grade"
    );
    let squads = walk
        .accumulated
        .athletes
        .values()
        .filter(|athlete| {
            athlete
                .observed_grades
                .iter()
                .any(|grade| grade.school_year == SchoolYear(2025))
        })
        .count();
    assert!(
        squads > 0,
        "the athletes carry the school year the meet's May date resolves to"
    );
}

#[test]
fn a_jurisdiction_is_read_only_from_a_published_state_code() {
    assert_eq!(jurisdiction_of(Some("WI")), Some(UsJurisdiction::Wisconsin));
    assert_eq!(
        jurisdiction_of(Some("XX")),
        None,
        "the masked rows the rankings document publishes carry `State: \"XX\"`, not a state"
    );
    assert_eq!(
        jurisdiction_of(None),
        None,
        "the out-of-scope `Overseas` region publishes no state code at all \
         (`samples/atn-probe-divchildren-168416.json`: `StateName: null`, `DivDivName: \"Overseas\"`)"
    );
    assert_eq!(jurisdiction_of(Some("")), None);
    assert_eq!(jurisdiction_of(Some("wi")), Some(UsJurisdiction::Wisconsin));
}

#[test]
fn a_meet_the_payload_does_not_place_is_dropped_whole() {
    let meet: MeetData =
        serde_json::from_str(MEET_DATA_WITHOUT_STATE).expect("the derived document decodes");
    let results: AllResults =
        serde_json::from_str(ALL_RESULTS).expect("the results document decodes");
    let walk = absorb(&meet, &results, None);
    assert_eq!(walk.counts.meets_unplaced, 1);
    assert_eq!(
        walk.counts.rows_seen, 0,
        "no block of a refused meet is walked"
    );
    assert_eq!(walk.counts.meets_pulled, 0);
    assert!(walk.accumulated.meets.is_empty(), "no meet row is minted");
    assert!(
        walk.accumulated.performances.is_empty(),
        "no performance is minted for an unplaced meet"
    );
}

#[test]
fn the_third_request_settles_the_marks_the_labels_already_settle() {
    let without = walk(false);
    let with = walk(true);
    assert_eq!(
        without.counts.rows_unmapped_event, 0,
        "every one of the capture's 49 block labels maps to a platform kind on its own, so the \
         third request is not needed to read this meet"
    );
    assert_eq!(
        with.counts.stored(),
        without.counts.stored(),
        "the metadata document never changes a row the label already settles"
    );
    assert_eq!(with.counts.blocks_event_type_mismatch, 0);
    assert_eq!(
        with.counts.blocks_metadata_absent, 0,
        "the metadata document lists every event the blocks name"
    );
    assert_eq!(
        performances(&with)
            .iter()
            .map(|row| row.mark.clone())
            .collect::<Vec<_>>(),
        performances(&without)
            .iter()
            .map(|row| row.mark.clone())
            .collect::<Vec<_>>()
    );
    let metadata = EventMetadata::new(
        &serde_json::from_str::<EventDivisions>(EVENT_DIV).expect("the metadata document decodes"),
    );
    assert_eq!(metadata.len(), 36);
    assert_eq!(metadata.field_events(), 12);
    assert_eq!(metadata.hurdles(), 4);
    assert_eq!(metadata.event_type(1), Some("T"), "`100 Meters` is track");
    assert_eq!(metadata.event_type(13), Some("F"), "`Discus` is field");
    assert_eq!(
        metadata.is_hurdle(28),
        Some(true),
        "`100m Hurdles` is a hurdle"
    );
    assert_eq!(metadata.is_hurdle(1), Some(false));
    assert_eq!(metadata.event_type(9999), None, "an unlisted event id");
}

#[test]
fn published_row_tokens_are_read_in_the_forms_this_capture_uses() {
    assert_eq!(gender_of("M"), Some(Gender::Boys));
    assert_eq!(gender_of("F"), Some(Gender::Girls));
    assert_eq!(gender_of("X"), None, "an unpublished gender is refused");
    let results: AllResults =
        serde_json::from_str(ALL_RESULTS).expect("the results document decodes");
    let squad = results
        .blocks
        .iter()
        .flat_map(|block| block.results.iter())
        .find(|row| row.last_name.is_none())
        .expect("this capture publishes relay squad rows");
    assert!(
        squad
            .first_name
            .as_deref()
            .is_some_and(|name| name.contains("<BR>")),
        "a squad row's FirstName holds its four legs as `<BR>`-joined markup: {:?}",
        squad.first_name
    );
    assert_eq!(squad.grade.as_deref(), Some("-"));
    assert!(squad.athlete_id.is_some());
    assert!(
        results
            .legs
            .iter()
            .any(|leg| leg.result_id == squad.result_id),
        "the squad's legs arrive separately, keyed by ResultID"
    );
    let walk = walk(false);
    let timings: Vec<Option<TimingMethod>> =
        performances(&walk).iter().map(|row| row.timing).collect();
    assert_eq!(
        timings
            .iter()
            .filter(|timing| **timing == Some(TimingMethod::Fat))
            .count(),
        658,
        "the 378 individual rows publishing a trailing `a` plus the 280 legs inheriting their \
         squad's automatic mark"
    );
    assert_eq!(
        timings
            .iter()
            .filter(|timing| **timing == Some(TimingMethod::Hand))
            .count(),
        245,
        "every other stored row is hand-or-unstated"
    );
    assert!(
        performances(&walk).iter().all(|row| matches!(
            row.mark,
            Mark::TimeSeconds(_)
                | Mark::DistanceMetres(_)
                | Mark::FieldImperial { .. }
                | Mark::Points(_)
        )),
        "every stored mark is one of the four published forms"
    );
}

#[test]
fn every_performance_is_keyed_by_the_published_result_it_came_from() {
    let walk = walk(false);
    assert_eq!(
        performances(&walk).len(),
        903,
        "the 623 individual results plus the 280 legs; the square rows are not performances"
    );
    let keys: BTreeSet<&str> = performances(&walk)
        .iter()
        .map(|row| row.source_key.as_str())
        .collect();
    assert_eq!(keys.len(), 903, "every row is keyed exactly once");
    let leg_keys = keys.iter().filter(|key| key.contains(":leg")).count();
    assert_eq!(leg_keys, 280, "each stored leg carries its own position");
    for row in performances(&walk) {
        assert_eq!(
            row.evidence.len(),
            1,
            "one parsed-evidence entry per row: {}",
            row.source_key
        );
        let evidence = row.evidence.first().expect("one evidence entry");
        assert_eq!(evidence.method, EvidenceMethod::Parsed);
        assert_eq!(evidence.observed_on, OBSERVED_ON);
        assert_eq!(evidence.source.id, "athleticnet");
        let _: &PerformanceId = &row.id;
    }
}
