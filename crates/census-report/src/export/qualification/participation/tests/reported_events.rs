use super::*;
use census_domain::model::{EventKind, SourceIdentity, SourceNamespace};
use fixture::CapturedCase;

mod publication;

#[tokio::test]
async fn retained_michigan_field_codes_are_reported_without_promoting_eligibility() -> TestResult {
    let fixture = Fixture::captured(CapturedCase::Michigan).await?;
    let pinned = fixture.assess()?;
    for (key, kind, basis) in [
        (
            "milesplit_result:190653353",
            EventKind::Discus,
            "owned_disc_reference",
        ),
        (
            "milesplit_result:190653349",
            EventKind::ShotPut,
            "owned_shot_reference",
        ),
        (
            "milesplit_result:190653350",
            EventKind::ShotPut,
            "owned_shot_reference",
        ),
    ] {
        let performance = fixture
            .store
            .scan::<CanonicalPerformance>(Table::Performances)?
            .into_iter()
            .find(|row| row.source_key == key)
            .ok_or("MI source row")?;
        let assessment = pinned.rows.get(&performance.id).ok_or("MI assessment")?;
        check!(ne; assessment.outcome, CandidateOutcome::EligibleCandidate);
        let interpretation = assessment
            .reported_event
            .as_ref()
            .ok_or("MI interpretation")?;
        check!(eq; interpretation.kind(), &kind);
        check!(eq; interpretation.basis(), basis);
        check!(eq; serde_json::to_value(interpretation)?.get("basis"), Some(&json!(basis)));
        let event = fixture
            .store
            .scan::<census_domain::model::CanonicalEvent>(Table::Events)?
            .into_iter()
            .find(|event| event.id == performance.event)
            .ok_or("MI event")?;
        check!(event.specification.implement.is_none());
    }
    Ok(())
}

#[tokio::test]
async fn unrelated_retained_track_event_never_gains_a_paired_code_interpretation() -> TestResult {
    let fixture = Fixture::prepare(None).await?;
    let pinned = fixture.assess()?;
    let row = pinned
        .rows
        .get(&fixture.performance.id)
        .ok_or("track assessment")?;
    check!(row.reported_event.is_none());
    Ok(())
}

#[tokio::test]
async fn retained_illinois_xc_context_keeps_physical_track_identity_and_unknown_comparison(
) -> TestResult {
    for case in [
        CapturedCase::IllinoisBoys,
        CapturedCase::IllinoisGirls,
        CapturedCase::IllinoisVarsity,
    ] {
        let fixture = Fixture::captured(case).await?;
        let before = serde_json::to_value(&fixture.performance)?;
        let snapshot = fixture.store.snapshot();
        let dataset = fixture.dataset(&snapshot)?;
        let pinned = fixture.read(&dataset, &snapshot, &fixture.profile)?;
        let assessment = pinned
            .rows
            .get(&fixture.performance.id)
            .ok_or("IL assessment")?;
        check!(ne; assessment.outcome, CandidateOutcome::EligibleCandidate);
        let interpretation = assessment
            .reported_event
            .as_ref()
            .ok_or("IL interpretation")?;
        check!(eq; interpretation.kind(), &EventKind::CrossCountry);
        check!(eq; interpretation.basis(), "owned_5000m_with_raw_cross_country_context");
        check!(eq; serde_json::to_value(interpretation)?.get("basis"), Some(&json!(interpretation.basis())));
        let event = dataset
            .events
            .iter()
            .find(|event| event.id == fixture.performance.event)
            .ok_or("retained event")?;
        check!(eq; event.kind, EventKind::Track5000m);
        check!(event.specification.cross_country.is_none());
        check!(eq; serde_json::to_value(&fixture.performance)?, before);
        let descriptor = pinned.replay.install(&fixture.objects())?;
        let affiliations =
            crate::export::qualification::current_affiliation::PinnedCurrentAffiliations::read(
                &dataset, &snapshot,
            )?;
        let replayed = PinnedParticipation::replay(
            &dataset,
            &fixture.profile,
            &fixture.objects(),
            &descriptor,
            &pinned.replay.fingerprints(),
            affiliations.roster_context(),
        )?;
        let replayed = replayed
            .rows
            .get(&fixture.performance.id)
            .ok_or("replayed IL assessment")?;
        check!(eq; replayed.reported_event, assessment.reported_event);
        check!(eq; replayed.outcome, assessment.outcome);
    }
    Ok(())
}

#[tokio::test]
async fn forged_physical_owner_never_receives_a_reported_interpretation() -> TestResult {
    let fixture = Fixture::captured(CapturedCase::IllinoisBoys).await?;
    let mut changed = fixture.performance.clone();
    changed.source_key = "milesplit_result:99999999".into();
    changed.id = CanonicalPerformance::mint(
        &changed.athlete,
        &changed.meet,
        &changed.event,
        &changed.date,
        &changed.source_key,
    );
    changed.source_athlete = Some(SourceIdentity::new(
        SourceNamespace::MilesplitAthlete,
        "99999999",
    ));
    fixture.store.append(Table::Performances, &changed)?;
    let pinned = fixture.assess()?;
    let row = pinned.rows.get(&changed.id).ok_or("forged row")?;
    check!(row.reported_event.is_none());
    check!(eq; row.outcome, CandidateOutcome::Unresolved);
    check!(row
        .diagnostics
        .contains(&ParticipationDiagnostic::SourceBindingMismatch));
    Ok(())
}
