use super::*;
use census_domain::UsJurisdiction;

fn evidence() -> Evidence {
    Evidence::parsed(
        SourceRef::new(
            "synthetic historical result",
            Some("https://result-fixture.test/result/old-result".into()),
        ),
        "2026-05-01",
    )
}

pub(super) fn append_performance(
    store: &Store,
    athlete: &CanonicalAthlete,
    school: &SchoolId,
) -> TestResult {
    let season = SchoolYear::new(2025).ok_or("fixture season")?;
    let team = CanonicalTeam {
        id: CanonicalTeam::mint(school, Sport::OutdoorTrack, Gender::Boys, season),
        school: school.clone(),
        sport: Sport::OutdoorTrack,
        gender: Gender::Boys,
        school_year: season,
        level: None,
        source_identities: Vec::new(),
        evidence: vec![evidence()],
        retained_conflicts: Vec::new(),
    };
    store.append(Table::Teams, &team)?;
    let mut meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "Old school meet",
        "2026-05-01",
        CompetitionLevel::Invitational,
    );
    meet.evidence = vec![evidence()];
    store.append(Table::Meets, &meet)?;
    let mut event = crate::test_events::owned(CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::Track400m,
            gender: Gender::Boys,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?);
    event.evidence = vec![evidence()];
    store.append(Table::Events, &event)?;
    let performance = CanonicalPerformance {
        id: CanonicalPerformance::mint(
            &athlete.id,
            &meet.id,
            &event.id,
            "2026-05-01",
            "old-result",
        ),
        athlete: athlete.id.clone(),
        team: team.id,
        event: event.id,
        meet: meet.id,
        date: "2026-05-01".into(),
        mark: Mark::TimeSeconds(ExactSeconds::parse("60.00")?),
        adjustment: None,
        wind_mps: None,
        place: None,
        heat: None,
        round: None,
        timing: None,
        observed_grade: None,
        evidence: vec![evidence()],
        source_key: "old-result".into(),
        source_athlete: None,
        retained_conflicts: Vec::new(),
        combined_completion: None,
    };
    store.append(Table::Performances, &performance)?;
    Ok(())
}

#[test]
fn controlled_current_school_selection_preserves_performance_time_school_and_wire_bytes(
) -> TestResult {
    let fixture = fixture::Fixture::new()?;
    let current = fixture::install(&fixture, "12439", "42", Sport::IndoorTrack, true)?;
    fixture.accept(&current.athlete.id)?;
    let (historical, historical_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Controlled Historic School",
        "controlled historic school",
        Some("Madison"),
    );
    fixture.store.append(Table::Schools, &historical)?;
    append_performance(&fixture.store, &current.athlete, &historical_id)?;
    issuance::review(
        &fixture,
        &current,
        "controlled-performance-time-affiliation",
    )?;
    let dataset = fixture.dataset()?;
    let original_performances = serde_json::to_vec(&dataset.performances)?;
    let physical = serde_json::to_vec(
        &fixture
            .store
            .scan::<CanonicalPerformance>(Table::Performances)?,
    )?;
    let (verified, _) = fixture.assess()?;
    check!(eq; verified.resolution(&current.athlete.id).school(), Some(&current.school.id));
    let selected = crate::report::Derivation::selected(
        &dataset,
        crate::report::Scope::AllSources,
        None,
        &[UsJurisdiction::Michigan],
        &verified,
    )?;
    check!(eq; selected.athletes().len(), 1);
    check!(eq; selected.athletes().first().ok_or("current selected athlete")?.school, current.school.id);
    let performance = dataset
        .performances
        .first()
        .ok_or("historical performance retained")?;
    check!(eq; dataset.teams.get(&performance.team).ok_or("performance time team")?.school, historical_id);
    check!(eq; serde_json::to_vec(&dataset.performances)?, original_performances);
    check!(eq; serde_json::to_vec(&fixture.store.scan::<CanonicalPerformance>(Table::Performances)?)?, physical);
    Ok(())
}

#[test]
fn historical_v3_and_reviewed_v4_profile_contracts_dispatch_without_reinterpretation() -> TestResult
{
    use census_domain::profile_execution::{
        selected_qualification_contract_ref_v3, validate_current_selected_qualification_profile,
        validate_selected_qualification_profile_v3, validate_selected_qualification_profile_v4,
        ContractRoleV1,
    };
    let reviewed = fixture::profile("2026-10-16")?;
    validate_current_selected_qualification_profile(&reviewed)?;
    let _reviewed_contract = validate_selected_qualification_profile_v4(&reviewed)?;
    let mut input = reviewed.input().clone();
    input.policies.affiliation =
        selected_qualification_contract_ref_v3(ContractRoleV1::Affiliation)?;
    input.policies.publication =
        selected_qualification_contract_ref_v3(ContractRoleV1::Publication)?;
    let historical = QueryProfileV1::new(input)?;
    validate_current_selected_qualification_profile(&historical)?;
    let _historical_contract = validate_selected_qualification_profile_v3(&historical)?;
    check!(ne; historical.id()?, reviewed.id()?);
    check!(ne; historical.input().policies.affiliation, reviewed.input().policies.affiliation);
    check!(ne; historical.input().policies.publication, reviewed.input().policies.publication);
    check!(eq; historical.input().policies.identity, reviewed.input().policies.identity);
    Ok(())
}

#[test]
fn mixed_historical_and_reviewed_affiliation_publication_contracts_are_refused() -> TestResult {
    use census_domain::profile_execution::{
        selected_qualification_contract_ref_v3, validate_current_selected_qualification_profile,
        ContractRoleV1, ProfileExecutionError,
    };
    let reviewed = fixture::profile("2026-10-16")?;
    for role in [ContractRoleV1::Affiliation, ContractRoleV1::Publication] {
        let mut input = reviewed.input().clone();
        match role {
            ContractRoleV1::Affiliation => {
                input.policies.affiliation = selected_qualification_contract_ref_v3(role)?
            }
            ContractRoleV1::Publication => {
                input.policies.publication = selected_qualification_contract_ref_v3(role)?
            }
            _ => return Err("controlled dispatch role".into()),
        }
        let mixed = QueryProfileV1::new(input)?;
        check!(matches!(
            validate_current_selected_qualification_profile(&mixed),
            Err(ProfileExecutionError::UnsupportedContractReference {
                role: ContractRoleV1::Publication
            })
        ));
    }
    Ok(())
}
