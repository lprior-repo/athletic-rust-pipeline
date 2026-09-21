use super::*;
use crate::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalSchool, CoachRole, Gender, GradYear, SourceIdentity,
    SourceNamespace, Sport,
};
#[test]
fn core_scope_keeps_only_non_athletic_net_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let (school, school_id) = CanonicalSchool::new("WI", "Abbotsford", "abbotsford");
    store.append(Table::Schools, &school).unwrap();

    // One athlete reachable only through the AthleticLIVE mirror, one only through the
    // Athletic.net host adapter, and one through MileSplit.
    let mut mirrored =
        CanonicalAthlete::new(&school_id, "Mirror Only", GradYear::CO2027, Gender::Boys);
    mirrored.evidence.push(Evidence::parsed(
        crate::model::SourceRef::new("athleticlive_athletes", None),
        "2026-09-20",
    ));
    let mut host = CanonicalAthlete::new(&school_id, "Host Only", GradYear::CO2027, Gender::Boys);
    host.evidence.push(Evidence::parsed(
        crate::model::SourceRef::new("athleticnet", None),
        "2026-09-20",
    ));
    let mut core_athlete =
        CanonicalAthlete::new(&school_id, "Core Athlete", GradYear::CO2027, Gender::Boys);
    core_athlete.evidence.push(Evidence::parsed(
        crate::model::SourceRef::new("athleticlive_athletes", None),
        "2026-09-20",
    ));
    core_athlete.evidence.push(Evidence::parsed(
        crate::model::SourceRef::new("milesplit_roster", None),
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
    let (school, school_id) = CanonicalSchool::new("WI", "Abbotsford", "abbotsford");
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
    assert_eq!(census.by_state.get("WI").unwrap().class_of_2027, 1);
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
    let (school, school_id) = CanonicalSchool::new("WI", "Abbotsford", "abbotsford");
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
    assert_eq!(census.by_state["WI"].schools, 1);
    assert!(!store.out_dir().join("athletes.jsonl").exists());
}
