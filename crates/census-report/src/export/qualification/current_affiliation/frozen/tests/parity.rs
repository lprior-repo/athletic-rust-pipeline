use super::*;
use crate::export::CurrentAffiliationResolution as Resolution;
use census_domain::model::{AthleteId, CanonicalSchool, QueryProfileV1, Sport};

#[test]
fn resolved_capture_day_live_and_frozen_results_match() -> TestResult {
    let fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
    parity(&fixture, &fixture.athlete, "resolved_at_selected_cutoff", 2)
}

#[test]
fn unknown_capture_day_live_and_frozen_results_match_without_tenure_inference() -> TestResult {
    let fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-08", true)?;
    parity(&fixture, &fixture.athlete, "current_affiliation_unknown", 0)
}

#[test]
fn cooperative_capture_day_live_and_frozen_results_match_without_campus_inference() -> TestResult {
    let fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
    let mut school = fixture
        .store
        .scan::<CanonicalSchool>(Table::Schools)?
        .into_iter()
        .next()
        .ok_or("school")?;
    school.co_op = true;
    fixture.store.append(Table::Schools, &school)?;
    parity(&fixture, &fixture.athlete, "coop_campus_unresolved", 2)
}

#[test]
fn cross_sport_conflict_matches_live_and_frozen_and_survives_missing_required_archive() -> TestResult
{
    let mut fixture = Fixture::new("2026-10-07T09:00:00Z", "2026-10-07", true)?;
    select_sports(&mut fixture, vec![Sport::CrossCountry, Sport::OutdoorTrack])?;
    let (other, _, _, _) = fixture::append_roster_sport(
        &fixture.store,
        "26848",
        "Current High",
        "2026-10-07T12:00:00Z",
        Sport::CrossCountry,
    )?;
    let canonical =
        identity::accept_pair(&fixture.store, &fixture.writer, &fixture.athlete, &other)?;
    parity(&fixture, &canonical, "conflicting_affiliation_evidence", 4)?;
    let (_, _, _, missing) =
        fixture::append_roster(&fixture.store, "52649", "Old High", "2026-10-07T18:00:00Z")?;
    std::fs::remove_file(missing)?;
    references_and_reasons(&fixture, &canonical, 0, &["archive_unavailable"])?;
    parity(&fixture, &canonical, "conflicting_affiliation_evidence", 4)
}

#[test]
fn selected_sports_union_same_school_references_independently_of_observation_order() -> TestResult {
    let mut fixture = Fixture::new("2026-10-07T09:00:00Z", "2026-10-07", true)?;
    let (athlete, school, _, _) = fixture::append_roster_sport(
        &fixture.store,
        "52649",
        "Old High",
        "2026-10-07T12:00:00Z",
        Sport::CrossCountry,
    )?;
    check!(eq; athlete, fixture.athlete);
    check!(eq; school, fixture.school);
    select_sports(&mut fixture, vec![Sport::CrossCountry, Sport::OutdoorTrack])?;
    parity(&fixture, &fixture.athlete, "resolved_at_selected_cutoff", 4)?;
    observation_order(&fixture)?;
    select_sports(&mut fixture, vec![Sport::CrossCountry])?;
    references_and_reasons(&fixture, &fixture.athlete, 2, &["outside_selected_sport"])?;
    parity(&fixture, &fixture.athlete, "resolved_at_selected_cutoff", 4)
}

#[test]
fn nonselected_sport_remains_unknown_with_retained_references_in_live_and_frozen_results(
) -> TestResult {
    let mut fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
    select_sports(&mut fixture, vec![Sport::IndoorTrack])?;
    parity(&fixture, &fixture.athlete, "current_affiliation_unknown", 2)
}

#[test]
fn no_positive_sport_cells_remain_unknown_in_live_and_frozen_results() -> TestResult {
    let fixture = Fixture::new("2026-10-06T12:00:00Z", "2026-10-07", true)?;
    fixture::append_roster_sport(
        &fixture.store,
        "52649",
        "Old High",
        "2026-10-07T12:00:00Z",
        Sport::Unknown,
    )?;
    parity(&fixture, &fixture.athlete, "current_affiliation_unknown", 2)
}

fn select_sports(fixture: &mut Fixture, sports: Vec<Sport>) -> TestResult {
    let mut input = fixture.profile.input().clone();
    input.sports = sports;
    fixture.profile = QueryProfileV1::new(input)?;
    fixture.store.register_profile(&fixture.profile)?;
    Ok(())
}

fn references_and_reasons(
    fixture: &Fixture,
    athlete: &AthleteId,
    references: usize,
    reasons: &[&str],
) -> TestResult {
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let view = PinnedCurrentAffiliations::read(&dataset, &snapshot)?.into_verified();
    check!(eq; view.resolution(athlete).evidence_refs().len(), references);
    let audit = view.audit(athlete).ok_or("audit reasons")?;
    check!(eq; audit.unavailable().iter().map(String::as_str).collect::<Vec<_>>(), reasons);
    Ok(())
}

fn parity(fixture: &Fixture, athlete: &AthleteId, status: &str, references: usize) -> TestResult {
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let before = serde_json::to_vec(&dataset)?;
    let live = PinnedCurrentAffiliations::read(&dataset, &snapshot)?.into_verified();
    let directory = tempfile::tempdir()?;
    let objects = objects(&directory)?;
    let expected = snapshot.tables_digest(&[Table::Athletes])?;
    let (descriptor, captured) = capture(&dataset, &snapshot, &objects)?;
    let wire = serde_json::to_vec(&descriptor)?;
    let decoded: FrozenAffiliationDescriptor = serde_json::from_slice(&wire)?;
    check!(eq; descriptor.version, 1);
    check!(eq; serde_json::to_vec(&decoded)?, wire);
    let replayed = replay(
        &dataset,
        &objects,
        &decoded,
        &expected,
        &descriptor.digest()?,
    )?
    .into_verified();
    let captured = captured.into_verified();
    for view in [&live, &captured, &replayed] {
        check!(eq; view.resolution(athlete), live.resolution(athlete));
        check!(eq; view.resolution(athlete).status(), status);
        let audit = view.audit(athlete).ok_or("audit")?;
        let live_audit = live.audit(athlete).ok_or("live audit")?;
        check!(eq; audit.references(), live_audit.references());
        check!(eq; audit.unavailable(), live_audit.unavailable());
        check!(eq; audit.references().len(), references);
        check!(audit
            .references()
            .windows(2)
            .all(|pair| matches!(pair, [left, right] if left < right)));
        if view.resolution(athlete).school().is_none() {
            identity::assert_unselected(&dataset, view, athlete)?;
        }
    }
    check!(eq; serde_json::to_vec(&dataset)?, before);
    Ok(())
}

fn observation_order(fixture: &Fixture) -> TestResult {
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let mut observations = Vec::new();
    snapshot.for_each_physical_observation_bounded(
        Table::Athletes,
        observation_budget(),
        |athlete: CanonicalAthlete, _, _, _| {
            observations.push(athlete);
            Ok(())
        },
    )?;
    let mut expected = None;
    for reverse in [false, true] {
        let mut reader = super::super::super::observation::Reader::new(
            &dataset,
            &fixture.profile,
            fixture.store.root(),
        )?;
        if reverse {
            observations.reverse();
        }
        for observation in &observations {
            let mut observation = observation.clone();
            if reverse {
                observation.evidence.reverse();
            }
            reader.accept(observation)?;
        }
        let (rows, audit) = reader.finish();
        let resolution = rows.get(&fixture.athlete).ok_or("resolved row")?;
        check!(
            matches!(resolution, Resolution::Resolved { school, evidence_refs } if school == &fixture.school && evidence_refs.len() == 4)
        );
        let audit = audit.into_verified();
        let audit = audit.get(&fixture.athlete).ok_or("ordered audit")?;
        let actual = (
            resolution.clone(),
            audit.references().to_vec(),
            audit.unavailable().to_vec(),
        );
        if let Some(prior) = &expected {
            check!(eq; &actual, prior);
        }
        expected = Some(actual);
    }
    Ok(())
}
