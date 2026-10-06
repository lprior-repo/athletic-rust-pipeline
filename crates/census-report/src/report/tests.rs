use super::*;
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, GradYear,
    SourceIdentity, SourceNamespace, Sport,
};
use census_domain::UsJurisdiction;

mod cohort_admission;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn census_of(store: &Store, scope: Scope) -> TestResult<Census> {
    let dataset = crate::export::ExportDataset::load(store)?;
    Ok(build_census(
        &Derivation::of(&dataset, scope, None),
        &store.out_dir(),
    ))
}

fn fixture_source(id: &str) -> SourceIdentity {
    SourceIdentity::new(SourceNamespace::Other("fixture".to_string()), id)
}

#[test]
fn core_scope_keeps_only_non_athletic_net_evidence() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (school, school_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford", None);
    store.append(Table::Schools, &school)?;

    let mut mirrored = CanonicalAthlete::new(
        &school_id,
        "Mirror Only",
        GradYear::CO2027,
        Gender::Boys,
        fixture_source("mirror-only"),
    );
    mirrored.evidence.push(Evidence::parsed(
        census_domain::model::SourceRef::new("athleticlive_athletes", None),
        "2026-09-20",
    ));
    let mut host = CanonicalAthlete::new(
        &school_id,
        "Host Only",
        GradYear::CO2027,
        Gender::Boys,
        fixture_source("host-only"),
    );
    host.evidence.push(Evidence::parsed(
        census_domain::model::SourceRef::new("athleticnet", None),
        "2026-09-20",
    ));
    let mut core_athlete = CanonicalAthlete::new(
        &school_id,
        "Core Athlete",
        GradYear::CO2027,
        Gender::Boys,
        fixture_source("core-athlete"),
    );
    core_athlete.evidence.push(Evidence::parsed(
        census_domain::model::SourceRef::new("athleticlive_athletes", None),
        "2026-09-20",
    ));
    core_athlete.evidence.push(Evidence::parsed(
        census_domain::model::SourceRef::new("milesplit_roster", None),
        "2026-09-20",
    ));
    store.append(Table::Athletes, &mirrored)?;
    store.append(Table::Athletes, &host)?;
    store.append(Table::Athletes, &core_athlete)?;

    let mut rows: Vec<CanonicalAthlete> = vec![mirrored, host, core_athlete];
    check!(eq; retain_core(&mut rows), 2);
    check!(eq; rows.len(), 1);
    check!(eq; rows[0].canonical_name, "Core Athlete");
    Ok(())
}

#[test]
fn census_counts_class_of_2027_with_evidence() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (school, school_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford", None);
    store.append(Table::Schools, &school)?;
    let mut athlete = CanonicalAthlete::new(
        &school_id,
        "Julian Aguilera",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169"),
    );
    athlete
        .public_profile_urls
        .push("https://wi.milesplit.com/athletes/14399169-julian-aguilera".to_string());
    athlete.sports.push(Sport::OutdoorTrack);
    store.append(Table::Athletes, &athlete)?;
    let mut coach = CanonicalCoach::new(
        &school_id,
        "Dana Coach",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    coach.professional_email = Some("coach@example.org".to_string());
    store.append(Table::Coaches, &coach)?;

    let census = census_of(&store, Scope::AllSources)?;
    check!(eq; census.totals.class_of_2027, 1);
    check!(eq; census.totals.class_of_2027_boys, 1);
    check!(eq; census.totals.class_of_2027_with_profile_url, 1);
    check!(eq; census.totals.class_of_2027_with_coach, 1);
    check!(eq; census.totals.class_of_2027_with_coach_email, 1);
    check!(eq; census.by_state[&JurisdictionBucket::from(UsJurisdiction::Wisconsin)].class_of_2027,
    1);
    check!(eq; census.class_of_2027_sports.outdoor_only, 1);
    check!(eq; census.providers.namespaces.get("milesplit_athlete"),
    Some(&1));
    let (json_path, csv_path) = write_census(&store, &census, Scope::AllSources)?;
    check!(json_path.exists() && csv_path.exists());
    Ok(())
}

#[test]
fn census_reads_merged_observations_without_consolidating() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (school, school_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford", None);
    store.append(Table::Schools, &school)?;
    let athlete = CanonicalAthlete::new(
        &school_id,
        "Julian Aguilera",
        GradYear::CO2027,
        Gender::Boys,
        fixture_source("julian-aguilera"),
    );
    store.append(Table::Athletes, &athlete)?;
    store.append(Table::Athletes, &athlete)?;

    let census = census_of(&store, Scope::AllSources)?;
    check!(eq; census.totals.athletes, 1);
    check!(eq; census.totals.class_of_2027, 1);
    check!(eq; census.totals.schools, 1);
    check!(eq; census.by_state[&JurisdictionBucket::from(UsJurisdiction::Wisconsin)].schools,
    1);
    check!(!store.out_dir().join("athletes.jsonl").exists());
    Ok(())
}

#[test]
fn every_jurisdiction_publishes_a_by_state_row() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;

    let empty = census_of(&store, Scope::AllSources)?;
    let expected = UsJurisdiction::CENSUS_SCOPE.len().saturating_add(1);
    check!(eq; empty.by_state.len(), expected);
    check!(empty.by_state.contains_key(&JurisdictionBucket::Unplaced));
    check!(empty
        .by_state
        .values()
        .all(|row| row.schools == 0 && row.athletes == 0));

    let (school, _school_id) =
        CanonicalSchool::new(UsJurisdiction::Wyoming, "Laramie", "laramie", None);
    store.append(Table::Schools, &school)?;
    let census = census_of(&store, Scope::AllSources)?;
    check!(eq; census.by_state.len(), expected);
    let wyoming = &census.by_state[&JurisdictionBucket::from(UsJurisdiction::Wyoming)];
    check!(eq; wyoming.schools, 1);
    check!(eq; wyoming.athletes, 0);
    check!(eq; census.totals.schools, 1);

    let (_json_path, csv_path) = write_census(&store, &census, Scope::AllSources)?;
    let csv = std::fs::read_to_string(&csv_path)?;
    check!(eq; csv.lines().count(), expected.saturating_add(2));
    check!(csv.contains("WY,1,0,"), "{csv}");
    check!(csv.contains("UNKNOWN,"), "{csv}");
    Ok(())
}

#[test]
fn out_of_scope_jurisdiction_is_named_in_notes_not_counted() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;

    let (in_scope, in_scope_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford", None);
    let (out_of_scope, out_of_scope_id) = CanonicalSchool::new(
        UsJurisdiction::Hawaii,
        "Honolulu Prep",
        "honolulu-prep",
        None,
    );
    store.append(Table::Schools, &in_scope)?;
    store.append(Table::Schools, &out_of_scope)?;
    let in_scope_athlete = CanonicalAthlete::new(
        &in_scope_id,
        "In Scope",
        GradYear::CO2027,
        Gender::Boys,
        fixture_source("in-scope"),
    );
    let out_of_scope_athlete = CanonicalAthlete::new(
        &out_of_scope_id,
        "Out Of Scope",
        GradYear::CO2027,
        Gender::Boys,
        fixture_source("out-of-scope"),
    );
    store.append(Table::Athletes, &in_scope_athlete)?;
    store.append(Table::Athletes, &out_of_scope_athlete)?;

    let census = census_of(&store, Scope::AllSources)?;

    check!(eq; census.by_state.len(),
    UsJurisdiction::CENSUS_SCOPE.len().saturating_add(1));
    check!(!census
        .by_state
        .contains_key(&JurisdictionBucket::from(UsJurisdiction::Hawaii)));
    check!(eq; census.totals.schools, 1);
    check!(eq; census.totals.athletes, 1);
    check!(eq; census.totals.class_of_2027, 1);
    let wisconsin = census
        .by_state
        .get(&JurisdictionBucket::from(UsJurisdiction::Wisconsin));
    check!(eq; wisconsin.map(|row| row.schools), Some(1));
    check!(eq; wisconsin.map(|row| row.class_of_2027), Some(1));

    let note = census
        .notes
        .iter()
        .find(|note| note.contains("Hawaii"))
        .ok_or("missing out-of-scope jurisdiction note")?;
    check!(
        note.contains("Hawaii (HI) is outside the census run scope"),
        "{note}"
    );
    check!(note.contains("schools=1"), "{note}");
    check!(note.contains("athletes=1"), "{note}");
    check!(note.contains("class_of_2027=1"), "{note}");

    let core = census_of(&store, Scope::Core)?;
    check!(eq; core.by_state.keys().collect::<Vec<_>>(),
    census.by_state.keys().collect::<Vec<_>>());
    Ok(())
}
