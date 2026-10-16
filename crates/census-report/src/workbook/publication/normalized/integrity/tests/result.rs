use crate::export::public_identity::tests::fixture::TestResult;
use crate::export::{
    CurrentAffiliationResolution, ExportDataset, VerifiedCurrentAffiliations,
    VerifiedParticipationView,
};
use census_domain::model::*;
use census_domain::UsJurisdiction;
use census_store::Store;
use std::collections::BTreeMap;

pub(super) fn append(
    store: &Store,
    dataset: &mut ExportDataset,
    athlete: &CanonicalAthlete,
    label: &str,
    date: &str,
) -> TestResult {
    let team = CanonicalTeam {
        id: CanonicalTeam::mint(
            &athlete.school,
            Sport::OutdoorTrack,
            athlete.gender,
            SchoolYear::DEFAULT,
        ),
        school: athlete.school.clone(),
        sport: Sport::OutdoorTrack,
        gender: athlete.gender,
        school_year: SchoolYear::DEFAULT,
        level: None,
        source_identities: Vec::new(),
        evidence: Vec::new(),
        retained_conflicts: Vec::new(),
    };
    let mut meet = CanonicalMeet::new_checked(
        Some(UsJurisdiction::Michigan),
        format!("Synthetic identity join meet {label}"),
        date,
        CompetitionLevel::Invitational,
    )?;
    meet.sports.push(Sport::OutdoorTrack);
    let event = crate::test_events::owned(CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::Track400m,
            gender: athlete.gender,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?);
    let source_key = format!("synthetic-identity-join-{label}");
    let mut performance = CanonicalPerformance::new(
        PerformanceIdentity {
            athlete: &athlete.id,
            event: &event.id,
            meet: &meet.id,
            date,
            source_key: &source_key,
        },
        PerformanceResult {
            team: &team.id,
            mark: Mark::TimeSeconds(ExactSeconds::parse("50.00")?),
            wind_mps: None,
            adjustment: None,
            place: Some(1),
        },
    )?;
    performance.timing = Some(TimingMethod::Fat);
    performance.source_athlete = athlete.source.clone();
    performance.evidence.push(Evidence::parsed(
        SourceRef::new(
            "synthetic-identity-results",
            Some(format!("https://synthetic-identity.test/results/{label}")),
        ),
        date,
    ));
    dataset.teams.insert(team.id.clone(), team);
    dataset.meets.push(meet);
    dataset.events.push(event);
    dataset.performances.push(performance);
    dataset.bind_participation_rule_for_test(
        dataset
            .selected_participation_rule()
            .ok_or("captured rule")?
            .clone(),
    )?;
    dataset.bind_public_identity_fixture_from_store(store)?;
    let generation = dataset.lineage.input_generation.clone();
    let profile = dataset.selected_profile_id().ok_or("profile")?.clone();
    let cutoff = dataset.selected_as_of().ok_or("cutoff")?.clone();
    let rule = dataset.selected_participation_rule().ok_or("rule")?.clone();
    dataset.bind_verified_views_for_test(
        VerifiedParticipationView::fixture(
            generation.clone(),
            profile.clone(),
            rule,
            BTreeMap::new(),
        ),
        VerifiedCurrentAffiliations::fixture(
            generation,
            profile,
            cutoff,
            dataset
                .athletes
                .iter()
                .map(|athlete| {
                    (
                        athlete.id.clone(),
                        CurrentAffiliationResolution::Resolved {
                            school: athlete.school.clone(),
                            evidence_refs: vec![format!("synthetic-affiliation-{}", athlete.id)],
                        },
                    )
                })
                .collect(),
        ),
    )?;
    Ok(())
}
