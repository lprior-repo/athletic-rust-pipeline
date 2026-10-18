use super::*;

fn divergent_meet_date(original: &CanonicalMeet) -> CanonicalMeet {
    let mut variant = original.clone();
    variant.date = "2025-12-31".to_string();
    variant
}

fn divergent_event_kind(original: &CanonicalEvent) -> CanonicalEvent {
    let mut variant = original.clone();
    variant.kind = census_domain::model::EventKind::from_source_label("100m Dash");
    variant
}

#[tokio::test]
async fn school_projection_refuses_same_id_merged_meet_conflict() -> TestResult {
    let fixture = crate::export::selected_test_fixture::Fixture::new().await?;
    let objects = tempfile::tempdir()?;
    let mut dataset = fixture.capture(objects.path())?;
    let performance = dataset.performances.first().ok_or("performance")?.clone();
    let team = dataset.teams.get(&performance.team).ok_or("team")?.clone();
    let original_meet = dataset
        .meets
        .iter()
        .find(|row| row.id == performance.meet)
        .ok_or("meet")?
        .clone();
    let event = dataset
        .events
        .iter()
        .find(|row| row.id == performance.event)
        .ok_or("event")?
        .clone();

    let divergent = divergent_meet_date(&original_meet);
    let meet = dataset
        .meets
        .iter_mut()
        .find(|row| row.id == performance.meet)
        .ok_or("meet")?;
    meet.merge(divergent);
    check!(
        !dataset
            .meets
            .iter()
            .find(|row| row.id == performance.meet)
            .ok_or("meet")?
            .retained_conflicts
            .is_empty(),
        "Entity::merge must record a meet date collision"
    );

    let mut budget = Admission::new("same id merged meet conflict");
    let result = canonical(
        &dataset,
        &original_meet,
        &event,
        &team,
        &performance,
        &team.school,
        &mut budget,
    );
    check!(matches!(
        result,
        Err(crate::report::ReportError::Store(
            census_store::StoreError::Refused { .. }
        ))
    ));
    check!(eq; dataset.performances.first().ok_or("retained performance")?, &performance);

    *dataset
        .meets
        .iter_mut()
        .find(|row| row.id == performance.meet)
        .ok_or("meet")? = original_meet.clone();
    let mut budget = Admission::new("same id merged meet conflict recovered");
    canonical(
        &dataset,
        &original_meet,
        &event,
        &team,
        &performance,
        &team.school,
        &mut budget,
    )?;
    Ok(())
}

#[tokio::test]
async fn school_projection_refuses_same_id_merged_event_conflict() -> TestResult {
    let fixture = crate::export::selected_test_fixture::Fixture::new().await?;
    let objects = tempfile::tempdir()?;
    let mut dataset = fixture.capture(objects.path())?;
    let performance = dataset.performances.first().ok_or("performance")?.clone();
    let team = dataset.teams.get(&performance.team).ok_or("team")?.clone();
    let meet = dataset
        .meets
        .iter()
        .find(|row| row.id == performance.meet)
        .ok_or("meet")?
        .clone();
    let original_event = dataset
        .events
        .iter()
        .find(|row| row.id == performance.event)
        .ok_or("event")?
        .clone();

    let divergent = divergent_event_kind(&original_event);
    let event = dataset
        .events
        .iter_mut()
        .find(|row| row.id == performance.event)
        .ok_or("event")?;
    event.merge(divergent);
    check!(
        !dataset
            .events
            .iter()
            .find(|row| row.id == performance.event)
            .ok_or("event")?
            .retained_conflicts
            .is_empty(),
        "Entity::merge must record an event kind collision"
    );

    let mut budget = Admission::new("same id merged event conflict");
    let result = canonical(
        &dataset,
        &meet,
        &original_event,
        &team,
        &performance,
        &team.school,
        &mut budget,
    );
    check!(matches!(
        result,
        Err(crate::report::ReportError::Store(
            census_store::StoreError::Refused { .. }
        ))
    ));
    check!(eq; dataset.performances.first().ok_or("retained performance")?, &performance);

    *dataset
        .events
        .iter_mut()
        .find(|row| row.id == performance.event)
        .ok_or("event")? = original_event.clone();
    let mut budget = Admission::new("same id merged event conflict recovered");
    canonical(
        &dataset,
        &meet,
        &original_event,
        &team,
        &performance,
        &team.school,
        &mut budget,
    )?;
    Ok(())
}

#[tokio::test]
async fn school_projection_accepts_clean_dataset_without_merged_conflicts() -> TestResult {
    let fixture = crate::export::selected_test_fixture::Fixture::new().await?;
    let objects = tempfile::tempdir()?;
    let dataset = fixture.capture(objects.path())?;
    let performance = dataset.performances.first().ok_or("performance")?.clone();
    let team = dataset.teams.get(&performance.team).ok_or("team")?.clone();
    let meet = dataset
        .meets
        .iter()
        .find(|row| row.id == performance.meet)
        .ok_or("meet")?
        .clone();
    let event = dataset
        .events
        .iter()
        .find(|row| row.id == performance.event)
        .ok_or("event")?
        .clone();

    check!(meet.retained_conflicts.is_empty(), "fixture meet must be clean");
    check!(event.retained_conflicts.is_empty(), "fixture event must be clean");
    check!(team.retained_conflicts.is_empty(), "fixture team must be clean");

    let mut budget = Admission::new("clean dataset no merged conflicts");
    canonical(
        &dataset,
        &meet,
        &event,
        &team,
        &performance,
        &team.school,
        &mut budget,
    )?;
    Ok(())
}