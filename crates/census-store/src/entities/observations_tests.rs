use crate::{Entity, Store, Table};
use census_domain::model::{
    normalize_name, AthleteId, CanonicalSchool, EventId, ExactSeconds, Mark, MeetId, RelayMember,
    RelayResult, SourceAthleteObservation, SourceIdentity, SourceNamespace, SourceObservation,
    SourceSchoolObservation, TeamId, TimingMethod, RELAY_MEMBER_NAME_FAMILY,
    RELAY_RESULT_SCALAR_FAMILY,
};
use census_domain::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn wiaa() -> SourceNamespace {
    SourceNamespace::AssociationSchool {
        association: "wiaa".to_string(),
    }
}

fn school_row(org_id: &str, name: &str) -> CanonicalSchool {
    let (mut school, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        name,
        normalize_name(name),
        Some("Abbotsford"),
    );
    school
        .source_identities
        .push(SourceIdentity::new(wiaa(), org_id).with_url(format!(
            "https://schools.wiaawi.org/Directory/School/GetDirectorySchool?orgID={org_id}"
        )));
    school
}

fn observed(school: &CanonicalSchool, observed_on: &str) -> TestResult<SourceSchoolObservation> {
    Ok(
        SourceSchoolObservation::of_school(&wiaa(), school, observed_on)
            .ok_or("a row carrying the source's identity has to mint an observation")?,
    )
}

fn athlete_row() -> SourceObservation {
    SourceObservation::Athlete(SourceAthleteObservation::new(
        SourceNamespace::MilesplitAthlete,
        "14399169",
        "roster:abbotsford:2025-26",
        "Jordan Blake",
        "2026-09-23",
    ))
}

#[test]
fn an_observation_is_keyed_by_the_providers_own_id() -> TestResult {
    let school = school_row("5151", "Abbotsford High School");
    let canonical = school.id.as_str().to_string();
    let row = observed(&school, "2026-09-23")?;
    check!(
        row.id.contains("5151"),
        "the provider's own object id is what the row is filed under: {}",
        row.id
    );
    check!(
        !row.id.contains(&canonical),
        "the canonical id must not appear in the key, or the row cannot outlive the merge: {}",
        row.id
    );
    check!(eq; row.observed_name, "Abbotsford High School");
    check!(eq; row.city.as_deref(), Some("Abbotsford"));
    check!(eq; row.association_id.as_deref(), Some("5151"), "an association namespace carries the association's own id for the school");
    check!(eq; row.source_row_key, "https://schools.wiaawi.org/Directory/School/GetDirectorySchool?orgID=5151", "the row keeps the page it was read from");
    Ok(())
}

#[test]
fn a_row_without_the_sources_identity_mints_nothing() {
    let school = school_row("5151", "Abbotsford High School");
    let elsewhere = SourceNamespace::AssociationSchool {
        association: "mshsl".to_string(),
    };
    assert!(
        SourceSchoolObservation::of_school(&elsewhere, &school, "2026-09-23").is_none(),
        "a namespace the row does not carry must not mint an observation"
    );
}

#[test]
fn two_sightings_of_one_school_share_one_row_and_keep_both_sequences() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let mut older = school_row("5151", "Abbotsford High School");
    older.school_website = None;
    let mut newer = school_row("5151", "Abbotsford HS");
    newer.school_website = Some("https://abbotsford.k12.wi.us".to_string());
    let rows = [
        SourceObservation::School(observed(&older, "2026-09-22")?),
        SourceObservation::School(observed(&newer, "2026-09-23")?),
    ];
    store.append_many(Table::SourceObservations, &rows)?;
    let held = store.count(Table::SourceObservations)?;
    check!(eq; held, 2, "both sightings are evidence and both are kept");
    let scanned = store.scan::<SourceObservation>(Table::SourceObservations)?;
    check!(eq; scanned.len(), 1, "one provider object reads back as one row, however often it was seen");
    let row = match &scanned[0] {
        SourceObservation::School(row) => row,
        other => {
            return Err(format!("a school observation has to read back as one: {other:?}").into())
        }
    };
    check!(eq; row.observed_name, "Abbotsford High School", "the first sighting keeps its identity: an upstream rename cannot move a stored result");
    check!(eq; row.observed_on, "2026-09-22", "the earliest day the object was seen is the one the row holds");
    check!(eq; row.official_url.as_deref(), Some("https://abbotsford.k12.wi.us"), "a field the later page publishes and the earlier one left blank is filled in");
    Ok(())
}

#[test]
fn both_object_kinds_live_in_one_table() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school = school_row("5151", "Abbotsford High School");
    let rows = [
        SourceObservation::School(observed(&school, "2026-09-23")?),
        athlete_row(),
    ];
    store.append_many(Table::SourceObservations, &rows)?;
    let scanned = store.scan::<SourceObservation>(Table::SourceObservations)?;
    check!(eq; scanned.len(), 2);
    check!(
        scanned
            .iter()
            .any(|row| row.object() == "school" && row.namespace() == &wiaa()),
        "the school row reads back under the source that published it"
    );
    check!(
        scanned.iter().any(|row| row.object() == "athlete"
            && row.id().contains("14399169")
            && row.observed_on() == "2026-09-23"),
        "the athlete row reads back keyed by its own provider id"
    );
    Ok(())
}

#[test]
fn an_athlete_observation_keeps_the_cohort_evidence_it_was_minted_with() -> TestResult {
    let athlete = SourceAthleteObservation::new(
        SourceNamespace::MilesplitAthlete,
        "14399169",
        "roster:abbotsford:2025-26",
        "Jordan Blake",
        "2026-09-23",
    )
    .with_school(Some("Abbotsford High School".to_string()));
    let row = SourceObservation::Athlete(athlete);
    let encoded = serde_json::to_string(&row)?;
    check!(
        encoded.contains("\"id\":\"milesplit_athlete:14399169\""),
        "the id stays at the top level where the key encoder reads it: {encoded}"
    );
    check!(encoded.contains("\"observed_school\":\"Abbotsford High School\""));
    Ok(())
}

fn relay_leg(order: u32, name: &str) -> RelayResult {
    RelayResult::new(
        &TeamId::mint("team", &["a"]),
        &EventId::mint("event", &["b"]),
        &MeetId::mint("meet", &["c"]),
        Mark::Raw("raw".to_string()),
        "test:key",
    )
    .with_member(RelayMember::new(order, name))
}

#[test]
fn a_second_capture_with_a_different_member_name_is_retained_as_a_conflict() -> TestResult {
    let mut held = relay_leg(1, "Jordan Blake");
    held.merge(relay_leg(1, "Jordan Black"));
    check!(eq; held.members.len(), 1, "one order still reads back as one slot");
    check!(eq; held.members[0].order, 1);
    check!(
        eq; held.members[0].name_as_published, "Jordan Blake",
        "the first published spelling keeps the display slot"
    );
    check!(eq; held.retained_conflicts.len(), 1, "the contradiction is retained, not dropped");
    let conflict = &held.retained_conflicts[0];
    check!(eq; conflict.family, RELAY_MEMBER_NAME_FAMILY);
    check!(eq; conflict.subject_id, held.id.as_str());
    check!(
        conflict.detail.contains("Jordan Blake") && conflict.detail.contains("Jordan Black"),
        "both spellings stay recoverable from the retained detail: {}",
        conflict.detail
    );
    Ok(())
}

#[test]
fn an_identical_relay_repeat_merges_idempotently() -> TestResult {
    let mut held = relay_leg(2, "Jordan Blake");
    held.merge(relay_leg(2, "Jordan Blake"));
    held.merge(relay_leg(2, "Jordan Blake"));
    check!(eq; held.members.len(), 1);
    check!(
        held.retained_conflicts.is_empty(),
        "a repeat sighting mints no conflict"
    );
    Ok(())
}

#[test]
fn three_spellings_for_one_slot_keep_two_conflicts() -> TestResult {
    let mut held = relay_leg(3, "Jordan Blake");
    held.merge(relay_leg(3, "Jordan Black"));
    held.merge(relay_leg(3, "Jorden Blake"));
    check!(eq; held.members.len(), 1, "one order still reads back as one slot");
    check!(eq; held.members[0].name_as_published, "Jordan Blake");
    check!(
        eq; held.retained_conflicts.len(), 2,
        "each distinct later spelling retains its own contradiction"
    );
    Ok(())
}

fn relay_leg_linked(order: u32, name: &str, athlete: &str) -> RelayResult {
    RelayResult::new(
        &TeamId::mint("team", &["a"]),
        &EventId::mint("event", &["b"]),
        &MeetId::mint("meet", &["c"]),
        Mark::Raw("raw".to_string()),
        "test:key",
    )
    .with_member(RelayMember::new(order, name).with_athlete(AthleteId::mint("athlete", &[athlete])))
}

#[test]
fn a_later_athlete_link_fills_an_unlinked_slot_without_conflict() -> TestResult {
    let mut held = relay_leg(4, "Jordan Blake");
    held.merge(relay_leg_linked(4, "Jordan Blake", "7"));
    check!(eq; held.members.len(), 1);
    check!(
        held.members[0].athlete.is_some(),
        "the same-spelling link fills the empty athlete slot"
    );
    check!(
        held.retained_conflicts.is_empty(),
        "linking the same spelling mints no conflict"
    );
    Ok(())
}

#[test]
fn two_athlete_links_for_one_spelling_retain_the_contradiction() -> TestResult {
    let mut held = relay_leg_linked(5, "Jordan Blake", "7");
    held.merge(relay_leg_linked(5, "Jordan Blake", "8"));
    check!(eq; held.members.len(), 1);
    check!(
        eq; held.retained_conflicts.len(), 1,
        "conflicting athlete links for one spelling are retained"
    );
    Ok(())
}

fn relay_team(place: Option<u16>) -> RelayResult {
    RelayResult::new(
        &TeamId::mint("team", &["a"]),
        &EventId::mint("event", &["b"]),
        &MeetId::mint("meet", &["c"]),
        Mark::Raw("raw".to_string()),
        "test:key",
    )
    .with_place(place)
}

#[test]
fn a_matching_scalar_repeat_mints_no_conflict() -> TestResult {
    let mut held = relay_team(Some(2))
        .with_round(Some("preliminaries".to_string()))
        .with_timing(Some(TimingMethod::Hand));
    held.merge(
        relay_team(Some(2))
            .with_round(Some("preliminaries".to_string()))
            .with_timing(Some(TimingMethod::Hand)),
    );
    check!(eq; held.place, Some(2));
    check!(eq; held.round.as_deref(), Some("preliminaries"));
    check!(eq; held.timing, Some(TimingMethod::Hand));
    check!(
        held.retained_conflicts.is_empty(),
        "a repeat sighting of the same scalars mints no conflict: {:?}",
        held.retained_conflicts
    );
    Ok(())
}

#[test]
fn a_contradictory_place_keeps_the_first_and_retains_the_scalar_conflict() -> TestResult {
    let mut held = relay_team(Some(2));
    held.merge(relay_team(Some(5)));
    check!(eq; held.place, Some(2), "the first sighting keeps the field");
    check!(eq; held.retained_conflicts.len(), 1, "the dropped place is retained");
    let conflict = &held.retained_conflicts[0];
    check!(eq; conflict.family, RELAY_RESULT_SCALAR_FAMILY);
    check!(eq; conflict.subject, "place");
    check!(eq; conflict.subject_id, held.id.as_str());
    check!(
        conflict.detail.contains("kept Some(2)") && conflict.detail.contains("dropped Some(5)"),
        "both places stay recoverable from the retained detail: {}",
        conflict.detail
    );
    Ok(())
}

#[test]
fn a_contradictory_round_retains_its_own_scalar_conflict() -> TestResult {
    let mut held = relay_team(Some(1)).with_round(Some("preliminaries".to_string()));
    held.merge(relay_team(Some(1)).with_round(Some("finals".to_string())));
    check!(eq; held.round.as_deref(), Some("preliminaries"));
    check!(eq; held.retained_conflicts.len(), 1);
    check!(eq; held.retained_conflicts[0].subject, "round");
    check!(
        held.retained_conflicts[0].detail.contains("finals"),
        "the dropped round stays recoverable: {}",
        held.retained_conflicts[0].detail
    );
    Ok(())
}

#[test]
fn a_contradictory_timing_retains_its_own_scalar_conflict() -> TestResult {
    let mut held = relay_team(Some(1)).with_timing(Some(TimingMethod::Hand));
    held.merge(relay_team(Some(1)).with_timing(Some(TimingMethod::Fat)));
    check!(eq; held.timing, Some(TimingMethod::Hand));
    check!(eq; held.retained_conflicts.len(), 1);
    check!(eq; held.retained_conflicts[0].subject, "timing");
    Ok(())
}

#[test]
fn a_later_scalar_fills_an_empty_field_without_conflict() -> TestResult {
    let mut held = relay_team(None);
    held.merge(relay_team(Some(3)).with_timing(Some(TimingMethod::Fat)));
    check!(eq; held.place, Some(3), "an empty scalar is filled, not contradicted");
    check!(eq; held.timing, Some(TimingMethod::Fat));
    check!(held.retained_conflicts.is_empty());
    Ok(())
}

#[test]
fn a_raw_mark_is_refined_by_a_known_mark_without_conflict() -> TestResult {
    let mut held = relay_leg(1, "Jordan Blake");
    let timed = RelayResult::new(
        &TeamId::mint("team", &["a"]),
        &EventId::mint("event", &["b"]),
        &MeetId::mint("meet", &["c"]),
        Mark::TimeSeconds(ExactSeconds::parse("41.86")?),
        "test:key",
    );
    held.merge(timed);
    check!(
        eq; held.mark, Mark::TimeSeconds(ExactSeconds::parse("41.86")?),
        "the raw mark is refined by the timed capture"
    );
    check!(
        held.retained_conflicts.is_empty(),
        "refining a raw mark is not a contradiction"
    );
    Ok(())
}

#[test]
fn a_later_raw_mark_cannot_displace_a_known_mark() -> TestResult {
    let mut held = RelayResult::new(
        &TeamId::mint("team", &["a"]),
        &EventId::mint("event", &["b"]),
        &MeetId::mint("meet", &["c"]),
        Mark::TimeSeconds(ExactSeconds::parse("41.86")?),
        "test:key",
    );
    held.merge(relay_leg(1, "Jordan Blake"));
    check!(
        eq; held.mark, Mark::TimeSeconds(ExactSeconds::parse("41.86")?),
        "a later raw capture cannot displace a known mark"
    );
    check!(
        held.retained_conflicts.is_empty(),
        "a raw later capture is not a contradiction"
    );
    Ok(())
}
