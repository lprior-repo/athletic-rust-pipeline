use super::*;
use census_domain::model::SourceAthleteObservation;
use census_domain::UsJurisdiction;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn expected() -> SourceSchoolObservation {
    SourceSchoolObservation::new(
        SourceNamespace::MilesplitSchool,
        "12439",
        "target-packet-row",
        "Controlled School",
        "2026-10-16",
    )
    .with_city(Some("Novi".into()))
    .with_state(Some(UsJurisdiction::Michigan))
    .with_association_id(Some("controlled-association".into()))
    .with_district(Some("controlled-district".into()))
    .with_urls(
        Some("https://school.example.edu".into()),
        Some("controlled-team".into()),
    )
}

#[test]
fn retained_other_packet_and_day_do_not_supply_the_exact_target() -> TestResult {
    let expected = expected();
    let mut actual = expected.clone();
    actual.source_row_key = "earlier-packet-row".into();
    actual.observed_on = "2026-10-15".into();
    for phase in [ScanPhase::Physical, ScanPhase::Published] {
        check!(!observation(
            &SourceObservation::School(actual.clone()),
            &expected,
            phase
        )?);
    }
    Ok(())
}

#[test]
fn raw_packet_date_contradiction_cannot_become_published_authority() -> TestResult {
    let expected = expected();
    let mut actual = expected.clone();
    actual.observed_on = "2026-10-15".into();
    check!(matches!(
        observation(
            &SourceObservation::School(actual.clone()),
            &expected,
            ScanPhase::Physical
        ),
        Err(StoreError::Refused { .. })
    ));
    check!(!observation(
        &SourceObservation::School(actual),
        &expected,
        ScanPhase::Published
    )?);
    Ok(())
}

#[test]
fn subject_and_stable_id_cannot_disagree() -> TestResult {
    let expected = expected();
    let mut forged_id = expected.clone();
    forged_id.id = "unrelated-observation-id".into();
    let mut foreign_subject = expected.clone();
    foreign_subject.source_school_id = "99999".into();
    for actual in [forged_id, foreign_subject] {
        for phase in [ScanPhase::Physical, ScanPhase::Published] {
            check!(matches!(
                observation(&SourceObservation::School(actual.clone()), &expected, phase),
                Err(StoreError::Refused { .. })
            ));
        }
    }
    Ok(())
}

#[test]
fn empty_alternate_locator_is_not_a_retained_occurrence() -> TestResult {
    let expected = expected();
    let mut actual = expected.clone();
    actual.source_row_key.clear();
    for phase in [ScanPhase::Physical, ScanPhase::Published] {
        check!(matches!(
            observation(&SourceObservation::School(actual.clone()), &expected, phase),
            Err(StoreError::Refused { .. })
        ));
    }
    Ok(())
}

#[test]
fn different_school_facts_refuse_in_both_phases_and_packet_scopes() -> TestResult {
    let expected = expected();
    let changes: [fn(&mut SourceSchoolObservation); 7] = [
        |row| row.observed_name = "Contradictory School".into(),
        |row| row.city = None,
        |row| row.state = None,
        |row| row.association_id = None,
        |row| row.district = None,
        |row| row.official_url = None,
        |row| row.athletics_net_team_id = None,
    ];
    for change in changes {
        for locator in ["target-packet-row", "other-packet-row"] {
            let mut actual = expected.clone();
            actual.source_row_key = locator.into();
            change(&mut actual);
            for phase in [ScanPhase::Physical, ScanPhase::Published] {
                check!(matches!(
                    observation(&SourceObservation::School(actual.clone()), &expected, phase),
                    Err(StoreError::Refused { .. })
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn an_athlete_row_cannot_claim_the_school_observation_id() -> TestResult {
    let expected = expected();
    let mut actual = SourceAthleteObservation::new(
        SourceNamespace::MilesplitAthlete,
        "77",
        "athlete-source-row",
        "Controlled Athlete",
        "2026-10-16",
    );
    actual.id = expected.id.clone();
    for phase in [ScanPhase::Physical, ScanPhase::Published] {
        check!(matches!(
            observation(
                &SourceObservation::Athlete(actual.clone()),
                &expected,
                phase
            ),
            Err(StoreError::Refused { .. })
        ));
    }
    Ok(())
}
