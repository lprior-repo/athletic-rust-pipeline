use census_domain::model::*;
use census_domain::UsJurisdiction;

use super::*;
use census_store::Entity;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn season() -> TestResult<census_domain::model::SchoolYear> {
    census_domain::model::SchoolYear::new(2026).ok_or_else(|| "invalid fixture season".into())
}

fn school() -> CanonicalSchool {
    let mut school =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford", None).0;
    school.source_identities.push(
        SourceIdentity::new(SourceNamespace::MilesplitSchool, "1234")
            .with_url("https://wi.milesplit.com/teams/1234"),
    );
    school
}

fn athlete(school: &CanonicalSchool) -> CanonicalAthlete {
    let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169");
    let mut athlete = CanonicalAthlete::new(
        &school.id,
        "Jane Doe",
        GradYear::CO2027,
        Gender::Girls,
        source,
    );
    athlete.add_identity(SourceIdentity::new(
        SourceNamespace::AthleticNet {
            kind: "athlete".to_string(),
        },
        "987654",
    ));
    athlete
}

fn unplaced_meet() -> CanonicalMeet {
    CanonicalMeet::new(
        None,
        "Unplaced Invite",
        "2026-04-01",
        CompetitionLevel::Invitational,
    )
}

#[test]
fn source_object_identities_keep_provider_ids_verbatim_and_name_their_table() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school = school();
    store.append(Table::Schools, &school)?;
    store.append(Table::Athletes, &athlete(&school))?;
    let dataset = census_report::export::ExportDataset::load(&store)?;
    let rows = canonical_pass(&dataset).identities;
    let ids: Vec<&str> = rows.iter().map(|row| row.id.as_str()).collect();
    check!(ids.contains(&"milesplit_school:schools:1234"), "{ids:?}");
    check!(
        ids.contains(&"athleticnet:athlete:athletes:987654"),
        "{ids:?}"
    );
    let milesplit = rows
        .iter()
        .find(|row| row.entity == SourceEntityKind::Schools)
        .ok_or("the school row")?;
    check!(eq; milesplit.canonical_id, school.id.as_str());
    check!(eq; milesplit.url.as_deref(), Some("https://wi.milesplit.com/teams/1234"));
    let athletic_net = rows
        .iter()
        .find(|row| row.entity == SourceEntityKind::Athletes)
        .ok_or("the athlete row")?;
    check!(athletic_net.url.is_none(), "no URL was observed");
    Ok(())
}

#[test]
fn the_jurisdiction_row_carries_its_measured_denominators() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school = school();
    store.append(Table::Schools, &school)?;
    store.append(Table::Athletes, &athlete(&school))?;
    let dataset = census_report::export::ExportDataset::load(&store)?;
    let identities = canonical_pass(&dataset).identities;
    let rows = coverage_rows(&dataset, &identities)?;
    let wisconsin = rows
        .iter()
        .find(|row| row.id == "jurisdiction:WI")
        .ok_or("a Wisconsin row")?;
    check!(eq; wisconsin.metrics.get("schools"), Some(&1));
    check!(eq; wisconsin.metrics.get("cohort_athletes"), Some(&1));
    check!(eq; wisconsin.metrics.get("girls"), Some(&1));
    check!(eq; rows.iter().filter(|row| row.id.starts_with("jurisdiction:")).count(),
        UsJurisdiction::CENSUS_SCOPE.len().saturating_add(1), "every jurisdiction in the run scope and the unplaced row publish a denominator");
    Ok(())
}

#[test]
fn source_rows_count_what_each_namespace_contributes_per_table() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school = school();
    store.append(Table::Schools, &school)?;
    store.append(Table::Athletes, &athlete(&school))?;
    let dataset = census_report::export::ExportDataset::load(&store)?;
    let identities = canonical_pass(&dataset).identities;
    let rows = super::coverage::source_coverage(&identities);
    let milesplit = rows
        .iter()
        .find(|row| row.id == "source:milesplit_school")
        .ok_or("a MileSplit row")?;
    check!(eq; milesplit.scope, CoverageScope::Source);
    check!(eq; milesplit.metrics.get("identities"), Some(&1));
    check!(eq; milesplit.metrics.get("schools"), Some(&1));
    let primary = rows
        .iter()
        .find(|row| row.id == "source:milesplit_athlete")
        .ok_or("the primary athlete source contributes its own coverage row")?;
    check!(eq; primary.metrics.get("athletes"), Some(&1));
    let athletic_net = rows
        .iter()
        .find(|row| row.id == "source:athleticnet:athlete")
        .ok_or("an Athletic.net row")?;
    check!(eq; athletic_net.metrics.get("athletes"), Some(&1));
    check!(eq; athletic_net.metrics.get("schools"), None);
    Ok(())
}

#[test]
fn a_pass_appends_every_index_and_a_snapshot_of_the_store() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school = school();
    store.append(Table::Schools, &school)?;
    store.append(Table::Athletes, &athlete(&school))?;
    let meet = unplaced_meet();
    store.append(Table::Meets, &meet)?;
    let report = derive(&store, "index", "2026-09-22", season()?)?;
    check!(eq; report.source_identities, 3);
    check!(eq; report.snapshots, 1);
    check!(
        report.reviews >= 1,
        "the unplaced venue is a retained review finding"
    );
    let snapshots: Vec<CollectionSnapshot> = store.scan(Table::Snapshots)?;
    check!(eq; snapshots.len(), 1);
    check!(eq; snapshots[0].id, "index:2026-09-22");
    check!(eq; snapshots[0].observations.get("schools"), Some(&1));
    check!(eq; snapshots[0].observations.get("meets"), Some(&1));
    let coverage: Vec<CoverageRow> = store.scan(Table::Coverage)?;
    check!(coverage.iter().any(|row| row.id == "jurisdiction:WI"));
    check!(coverage.iter().any(|row| row.id.starts_with("source:")));
    let cases: Vec<ReviewCase> = store.scan(Table::ReviewCases)?;
    let venue = cases
        .iter()
        .find(|case| case.subject_id == meet.id.as_str())
        .ok_or("the unplaced venue is a case")?;
    check!(eq; venue.state, ReviewState::Pending);
    check!(eq; venue.family, "Meet venue unresolved");
    Ok(())
}

#[test]
fn a_repeated_pass_reuses_every_id_and_keeps_one_snapshot_a_day() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school = school();
    store.append(Table::Schools, &school)?;
    derive(&store, "index", "2026-09-22", season()?)?;
    let first_ids: Vec<_> = store
        .scan::<SourceObjectIdentity>(Table::SourceIdentities)?
        .into_iter()
        .map(|identity| identity.id)
        .collect();
    derive(&store, "index", "2026-09-22", season()?)?;
    let identities: Vec<SourceObjectIdentity> = store.scan(Table::SourceIdentities)?;
    check!(eq; identities.len(), 1, "the second pass reuses the row id, so a read sees one identity");
    check!(eq; identities.into_iter().map(|identity| identity.id).collect::<Vec<_>>(), first_ids,
        "rederivation preserves provider identity row IDs");
    let snapshots: Vec<CollectionSnapshot> = store.scan(Table::Snapshots)?;
    check!(eq; snapshots.len(), 1, "one snapshot per phase per day");
    check!(eq; snapshots[0].observations.get("source_identities"), Some(&0),
        "the counter counts appended observations, and a derived table is replaced rather than \
         appended, so re-deriving it never moves the counter");
    Ok(())
}

#[test]
fn a_review_decision_survives_the_next_derivation() {
    let decided = {
        let mut case =
            ReviewCase::pending("Meet venue unresolved", "meet:m1", "Invite", "no venue");
        case.state = ReviewState::Resolved;
        case
    };
    let incoming = ReviewCase::pending(
        "Meet venue unresolved",
        "meet:m1",
        "Invite",
        "still no venue",
    );
    let mut merged = decided.clone();
    merged.merge(incoming);
    assert_eq!(
        merged.state,
        ReviewState::Resolved,
        "a verdict is not reset"
    );
    assert_eq!(
        merged.detail, "still no venue",
        "the finding itself still updates"
    );
    let mut pending = ReviewCase::pending("Meet venue unresolved", "meet:m1", "Invite", "no venue");
    pending.merge(ReviewCase::pending(
        "Meet venue unresolved",
        "meet:m1",
        "Invite",
        "still no venue",
    ));
    assert_eq!(pending.state, ReviewState::Pending);
}

fn other_subject_under_one_id(school: &CanonicalSchool, id: &AthleteId) -> CanonicalAthlete {
    let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14407777");
    let mut row = CanonicalAthlete::new(
        &school.id,
        "Marta Reyes",
        GradYear::CO2027,
        Gender::Girls,
        source,
    );
    row.id = id.clone();
    row
}

#[test]
fn a_canonical_id_collision_reaches_the_conflict_queue() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school = school();
    store.append(Table::Schools, &school)?;
    let jane = athlete(&school);
    let marta = other_subject_under_one_id(&school, &jane.id);
    store.append(Table::Athletes, &jane)?;
    store.append(Table::Athletes, &marta)?;
    let report = derive(&store, "index", "2026-09-22", season()?)?;
    let conflicts: Vec<RetainedConflict> = store.scan(Table::Conflicts)?;
    let collision = conflicts
        .iter()
        .find(|row| row.family == CANONICAL_ID_COLLISION_FAMILY)
        .ok_or("an id two subjects were keyed under is a retained conflict, not a silent merge")?;
    check!(eq; collision.subject_id, jane.id.as_str());
    let detail = collision.detail.to_lowercase();
    check!(
        detail.contains("jane"),
        "the kept side's material: {detail}"
    );
    check!(
        detail.contains("marta"),
        "the other side's material: {detail}"
    );
    check!(
        report.conflicts >= 1,
        "the pass counts the finding it wrote into the queue"
    );
    Ok(())
}

fn case_for(store: &Store, subject: &str) -> TestResult<ReviewCase> {
    Ok(store
        .scan::<ReviewCase>(Table::ReviewCases)?
        .into_iter()
        .find(|case| case.subject_id == subject)
        .ok_or("the subject's case")?)
}

#[test]
fn a_recorded_decision_survives_the_next_derivation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let meet = unplaced_meet();
    store.append(Table::Meets, &meet)?;
    derive(&store, "index", "2026-09-22", season()?)?;
    let case = case_for(&store, meet.id.as_str())?;
    check!(eq; case.state, ReviewState::Pending, "the lane has asked nothing");
    let mut decided = case.clone();
    decided.state = ReviewState::Resolved;
    store.replace(Table::ReviewCases, &decided)?;
    let report = derive(&store, "index", "2026-09-22", season()?)?;
    check!(eq; case_for(&store, meet.id.as_str())?.state, ReviewState::Resolved,
        "the second pass re-derives the finding without putting the decision back in the queue");
    check!(eq; report.superseded, 0, "a case this pass derives again is not superseded");
    Ok(())
}

#[test]
fn a_pending_case_whose_finding_is_gone_is_superseded() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let ghost = ReviewCase::pending(
        UNRESOLVED_VENUE_FAMILY,
        "meet:ghost",
        "Ghost Invitational",
        "no evidence placed the venue in a jurisdiction",
    );
    store.replace(Table::ReviewCases, &ghost)?;
    let live = unplaced_meet();
    store.append(Table::Meets, &live)?;
    let report = derive(&store, "index", "2026-09-22", season()?)?;
    check!(eq; report.superseded, 1);
    check!(eq; case_for(&store, "meet:ghost")?.state, ReviewState::Superseded,
        "the reading that no longer stands is closed rather than left owing a decision");
    check!(eq; case_for(&store, live.id.as_str())?.state, ReviewState::Pending, "the finding this pass did reach is still the lane's work");
    Ok(())
}

#[test]
fn a_cohort_claim_the_rules_decide_is_stored_retained() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school = school();
    store.append(Table::Schools, &school)?;
    let athlete = athlete(&school);
    store.append(Table::Athletes, &athlete)?;
    derive(&store, "index", "2026-09-22", season()?)?;
    let unverified = store
        .scan::<ReviewCase>(Table::ReviewCases)?
        .into_iter()
        .find(|case| case.family == COHORT_UNVERIFIED_FAMILY)
        .ok_or("a class-of-2027 athlete with no grade observation is a retained finding")?;
    check!(eq; unverified.state, ReviewState::Retained, "the evidence rule decided it, so it is not a question the lane is holding open");
    check!(eq; unverified.subject_id, athlete.id.as_str());
    Ok(())
}
