mod merged;

use super::*;
use census_domain::model::RetainedConflict;
use census_store::Entity;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Clone, Copy)]
enum Parent {
    Team,
    Meet,
    Event,
}

#[tokio::test]
async fn school_projection_refuses_retained_dataset_parent_conflicts() -> TestResult {
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
    let event = dataset
        .events
        .iter()
        .find(|row| row.id == performance.event)
        .ok_or("event")?
        .clone();
    let mut budget = Admission::new("school parent conflict regression");
    for parent in [Parent::Team, Parent::Meet, Parent::Event] {
        conflicts(&mut dataset, &performance, parent)?.push(RetainedConflict::new(
            "parent",
            "controlled-parent",
            "original",
            "contradiction",
        ));
        let result = canonical(
            &dataset,
            &meet,
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
        conflicts(&mut dataset, &performance, parent)?.clear();
    }
    Ok(())
}

#[tokio::test]
async fn school_projection_refuses_same_id_preconflicted_parent_fields() -> TestResult {
    let fixture = crate::export::selected_test_fixture::Fixture::new().await?;
    let objects = tempfile::tempdir()?;
    let mut dataset = fixture.capture(objects.path())?;
    let performance = dataset.performances.first().ok_or("performance")?.clone();
    let team = dataset.teams.get(&performance.team).ok_or("team")?.clone();
    let original_school = team.school.clone();
    let mut divergent = team.clone();
    divergent.school = census_domain::model::Id::mint("school", &["contradictory", "entity"]);
    dataset
        .teams
        .get_mut(&performance.team)
        .ok_or("team")?
        .merge(divergent);
    check!(
        !dataset
            .teams
            .get(&performance.team)
            .ok_or("team")?
            .retained_conflicts
            .is_empty(),
        "Entity::merge must record an id collision"
    );
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
    let mut budget = Admission::new("same id preconflicted parent field");
    let result = canonical(
        &dataset,
        &meet,
        &event,
        &team,
        &performance,
        &original_school,
        &mut budget,
    );
    check!(matches!(
        result,
        Err(crate::report::ReportError::Store(
            census_store::StoreError::Refused { .. }
        ))
    ));
    check!(eq; dataset.performances.first().ok_or("retained performance")?, &performance);
    Ok(())
}

#[tokio::test]
async fn school_projection_refuses_mismatched_meet_date() -> TestResult {
    let fixture = crate::export::selected_test_fixture::Fixture::new().await?;
    let objects = tempfile::tempdir()?;
    let mut dataset = fixture.capture(objects.path())?;
    let performance = dataset.performances.first().ok_or("performance")?.clone();
    let team = dataset.teams.get(&performance.team).ok_or("team")?.clone();
    let event = dataset
        .events
        .iter()
        .find(|row| row.id == performance.event)
        .ok_or("event")?
        .clone();
    let original_meet = dataset
        .meets
        .iter()
        .find(|row| row.id == performance.meet)
        .ok_or("meet")?
        .clone();
    let mut conflicting = original_meet.clone();
    conflicting.date = "2025-12-31".to_string();
    let meet = dataset
        .meets
        .iter_mut()
        .find(|row| row.id == performance.meet)
        .ok_or("meet")?;
    *meet = conflicting;
    let mut budget = Admission::new("mismatched published meet date");
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
    Ok(())
}

fn conflicts<'a>(
    dataset: &'a mut ExportDataset,
    performance: &CanonicalPerformance,
    parent: Parent,
) -> Result<&'a mut Vec<RetainedConflict>, Box<dyn std::error::Error>> {
    Ok(match parent {
        Parent::Team => {
            &mut dataset
                .teams
                .get_mut(&performance.team)
                .ok_or("team")?
                .retained_conflicts
        }
        Parent::Meet => {
            &mut dataset
                .meets
                .iter_mut()
                .find(|row| row.id == performance.meet)
                .ok_or("meet")?
                .retained_conflicts
        }
        Parent::Event => {
            &mut dataset
                .events
                .iter_mut()
                .find(|row| row.id == performance.event)
                .ok_or("event")?
                .retained_conflicts
        }
    })
}
