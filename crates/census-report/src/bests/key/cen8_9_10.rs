use super::ComparisonPolicy;
use census_domain::model::*;

mod boundaries;
mod courses;
mod dimensions;
mod fixtures;
mod surfaces;
use fixtures::*;

#[test]
fn cen9_six_real_schema_indoor_labels_reduce_across_meets() -> TestResult {
    let mut data = dataset()?;
    for (label, expected) in [
        ("Boys 55 Meter Dash", EventKind::Track55m),
        ("Boys 60m", EventKind::Track60m),
        ("Boys 1000 Meter Run", EventKind::Track1000m),
        ("Boys 1500m", EventKind::Track1500m),
        ("Boys 55 Meter Hurdles", EventKind::Track55mHurdles),
        ("Boys 60m Hurdles", EventKind::Track60mHurdles),
    ] {
        for (date, time) in [("2025-01-01", "12.20"), ("2025-02-01", "12.10")] {
            let meet = meet(&format!("{label}/{date}"), Sport::IndoorTrack, date);
            let event = event(&meet, label)?;
            assert_eq!(event.kind, expected);
            add(&mut data, meet, event, time)?;
        }
    }
    let rows = selections(&data);
    let winners: Vec<_> = rows
        .iter()
        .map(|row| {
            (
                &row.key.event_kind,
                row.meet.date.as_str(),
                row.population.marks,
            )
        })
        .collect();
    assert_eq!(winners.len(), 6);
    for (_, date, marks) in &winners {
        assert_eq!((*date, *marks), ("2025-02-01", 2));
    }
    for kind in [
        EventKind::Track55m,
        EventKind::Track60m,
        EventKind::Track1000m,
        EventKind::Track1500m,
        EventKind::Track55mHurdles,
        EventKind::Track60mHurdles,
    ] {
        assert_eq!(
            winners
                .iter()
                .filter(|(event, _, _)| **event == kind)
                .count(),
            1
        );
    }
    Ok(())
}
