use super::*;
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, GradYear,
    SourceIdentity, SourceNamespace, Sport,
};
use census_domain::UsJurisdiction;
#[test]
fn core_scope_keeps_only_non_athletic_net_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let (school, school_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford");
    store.append(Table::Schools, &school).unwrap();

    // One athlete reachable only through the AthleticLIVE mirror, one only through the
    // Athletic.net host adapter, and one through MileSplit.
    let mut mirrored =
        CanonicalAthlete::new(&school_id, "Mirror Only", GradYear::CO2027, Gender::Boys);
    mirrored.evidence.push(Evidence::parsed(
        census_domain::model::SourceRef::new("athleticlive_athletes", None),
        "2026-09-20",
    ));
    let mut host = CanonicalAthlete::new(&school_id, "Host Only", GradYear::CO2027, Gender::Boys);
    host.evidence.push(Evidence::parsed(
        census_domain::model::SourceRef::new("athleticnet", None),
        "2026-09-20",
    ));
    let mut core_athlete =
        CanonicalAthlete::new(&school_id, "Core Athlete", GradYear::CO2027, Gender::Boys);
    core_athlete.evidence.push(Evidence::parsed(
        census_domain::model::SourceRef::new("athleticlive_athletes", None),
        "2026-09-20",
    ));
    core_athlete.evidence.push(Evidence::parsed(
        census_domain::model::SourceRef::new("milesplit_roster", None),
        "2026-09-20",
    ));
    store.append(Table::Athletes, &mirrored).unwrap();
    store.append(Table::Athletes, &host).unwrap();
    store.append(Table::Athletes, &core_athlete).unwrap();

    let mut rows: Vec<CanonicalAthlete> = vec![mirrored, host, core_athlete];
    assert_eq!(retain_core(&mut rows), 2);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].canonical_name, "Core Athlete");
}

#[test]
fn census_counts_class_of_2027_with_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let (school, school_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford");
    store.append(Table::Schools, &school).unwrap();
    let mut athlete = CanonicalAthlete::new(
        &school_id,
        "Julian Aguilera",
        GradYear::CO2027,
        Gender::Boys,
    );
    athlete.source_identities.push(SourceIdentity::new(
        SourceNamespace::MilesplitAthlete,
        "14399169",
    ));
    athlete
        .public_profile_urls
        .push("https://wi.milesplit.com/athletes/14399169-julian-aguilera".to_string());
    athlete.sports.push(Sport::OutdoorTrack);
    store.append(Table::Athletes, &athlete).unwrap();
    let mut coach = CanonicalCoach::new(
        &school_id,
        "Dana Coach",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    coach.professional_email = Some("coach@example.org".to_string());
    store.append(Table::Coaches, &coach).unwrap();

    // No consolidation step: the census reads the entity tables through the store.
    let census = build_census(&store, Scope::AllSources).unwrap();
    assert_eq!(census.totals.class_of_2027, 1);
    assert_eq!(census.totals.class_of_2027_boys, 1);
    assert_eq!(census.totals.class_of_2027_with_profile_url, 1);
    assert_eq!(census.totals.class_of_2027_with_coach, 1);
    assert_eq!(census.totals.class_of_2027_with_coach_email, 1);
    assert_eq!(
        census.by_state[&JurisdictionBucket::from(UsJurisdiction::Wisconsin)].class_of_2027,
        1
    );
    assert_eq!(census.class_of_2027_sports.outdoor_only, 1);
    assert_eq!(
        census.providers.namespaces.get("milesplit_athlete"),
        Some(&1)
    );
    let (json_path, csv_path) = write_census(&store, &census, Scope::AllSources).unwrap();
    assert!(json_path.exists() && csv_path.exists());
}

#[test]
fn census_reads_merged_observations_without_consolidating() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let (school, school_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford");
    store.append(Table::Schools, &school).unwrap();
    let athlete = CanonicalAthlete::new(
        &school_id,
        "Julian Aguilera",
        GradYear::CO2027,
        Gender::Boys,
    );
    // The same entity appended twice and never consolidated: the report still sees one athlete.
    store.append(Table::Athletes, &athlete).unwrap();
    store.append(Table::Athletes, &athlete).unwrap();

    let census = build_census(&store, Scope::AllSources).unwrap();
    assert_eq!(census.totals.athletes, 1);
    assert_eq!(census.totals.class_of_2027, 1);
    assert_eq!(census.totals.schools, 1);
    // The workbook prints a state's school count off its `by_state` row, so the row has to carry
    // the count and not just the totals.
    assert_eq!(
        census.by_state[&JurisdictionBucket::from(UsJurisdiction::Wisconsin)].schools,
        1
    );
    assert!(!store.out_dir().join("athletes.jsonl").exists());
}

#[test]
fn every_jurisdiction_publishes_a_by_state_row() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    // An empty store still names every configured jurisdiction, plus the bucket for rows no school
    // placed: an omitted state reads as one nobody looked at.
    let empty = build_census(&store, Scope::AllSources).unwrap();
    let expected = UsJurisdiction::CENSUS_SCOPE.len().saturating_add(1);
    assert_eq!(empty.by_state.len(), expected);
    assert!(empty.by_state.contains_key(&JurisdictionBucket::Unplaced));
    assert!(empty
        .by_state
        .values()
        .all(|row| row.schools == 0 && row.athletes == 0));

    // A state with a school and no athlete is "covered, empty": its row carries the school count and
    // zero athletes rather than dropping out of `by_state`.
    let (school, _school_id) = CanonicalSchool::new(UsJurisdiction::Wyoming, "Laramie", "laramie");
    store.append(Table::Schools, &school).unwrap();
    let census = build_census(&store, Scope::AllSources).unwrap();
    assert_eq!(census.by_state.len(), expected);
    let wyoming = &census.by_state[&JurisdictionBucket::from(UsJurisdiction::Wyoming)];
    assert_eq!(wyoming.schools, 1);
    assert_eq!(wyoming.athletes, 0);
    assert_eq!(census.totals.schools, 1);

    // The published artifacts carry the same rows: the header, one line per jurisdiction whatever it
    // holds, and the totals line — a state cannot vanish from `report.json` or the CSV either.
    let (_json_path, csv_path) = write_census(&store, &census, Scope::AllSources).unwrap();
    let csv = std::fs::read_to_string(&csv_path).unwrap();
    assert_eq!(csv.lines().count(), expected.saturating_add(2));
    assert!(csv.contains("WY,1,0,"), "{csv}");
    assert!(csv.contains("UNKNOWN,"), "{csv}");
}

#[test]
fn out_of_scope_jurisdiction_is_named_in_notes_not_counted() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    // Wisconsin is a jurisdiction a census run covers; Hawaii is modelled, never acquired, and a
    // store can still hold rows for it.
    let (in_scope, in_scope_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford");
    let (out_of_scope, out_of_scope_id) =
        CanonicalSchool::new(UsJurisdiction::Hawaii, "Honolulu Prep", "honolulu-prep");
    store.append(Table::Schools, &in_scope).unwrap();
    store.append(Table::Schools, &out_of_scope).unwrap();
    let in_scope_athlete =
        CanonicalAthlete::new(&in_scope_id, "In Scope", GradYear::CO2027, Gender::Boys);
    let out_of_scope_athlete = CanonicalAthlete::new(
        &out_of_scope_id,
        "Out Of Scope",
        GradYear::CO2027,
        Gender::Boys,
    );
    store.append(Table::Athletes, &in_scope_athlete).unwrap();
    store
        .append(Table::Athletes, &out_of_scope_athlete)
        .unwrap();

    let census = build_census(&store, Scope::AllSources).unwrap();

    // The run scope decides which jurisdictions publish at all: one row per run-scope jurisdiction
    // plus the unplaced row, and no row for the jurisdiction the run does not cover.
    assert_eq!(
        census.by_state.len(),
        UsJurisdiction::CENSUS_SCOPE.len().saturating_add(1)
    );
    assert!(!census
        .by_state
        .contains_key(&JurisdictionBucket::from(UsJurisdiction::Hawaii)));
    // Nothing the out-of-scope side holds reaches a row or a total.
    assert_eq!(census.totals.schools, 1);
    assert_eq!(census.totals.athletes, 1);
    assert_eq!(census.totals.class_of_2027, 1);
    let wisconsin = census
        .by_state
        .get(&JurisdictionBucket::from(UsJurisdiction::Wisconsin));
    assert_eq!(wisconsin.map(|row| row.schools), Some(1));
    assert_eq!(wisconsin.map(|row| row.class_of_2027), Some(1));

    // The work is named rather than dropped: a note carries the jurisdiction and the row it would
    // have contributed.
    let note = census
        .notes
        .iter()
        .find(|note| note.contains("Hawaii"))
        .expect("a note names the jurisdiction the run scope leaves out");
    assert!(
        note.contains("Hawaii (HI) is outside the census run scope"),
        "{note}"
    );
    assert!(note.contains("schools=1"), "{note}");
    assert!(note.contains("athletes=1"), "{note}");
    assert!(note.contains("class_of_2027=1"), "{note}");

    // The evidence scope never changes which jurisdictions publish: a core report and an all-sources
    // report differ only in the rows they admit.
    let core = build_census(&store, Scope::Core).unwrap();
    assert_eq!(
        core.by_state.keys().collect::<Vec<_>>(),
        census.by_state.keys().collect::<Vec<_>>()
    );
}
