use super::*;
use census_domain::model::{
    CanonicalEvent, CanonicalPerformance, EventIdentity, EventSpecification, ExactSeconds, Gender,
    PerformanceIdentity, PerformanceResult, SourceEventLabel, TimingMethod,
};
use census_store::Table;

#[test]
fn current_six_event_publication_reopens_and_independently_verifies() -> TestResult {
    let fixture = fixture()?;
    seed_flat_events(&fixture)?;
    let dataset = ExportDataset::load(&fixture.store)?;
    let selected = bests::build_from_dataset(&dataset, &best_options())?;
    check!(eq; selected.len(), 7);
    let workbook = publish(&fixture, &dataset)?;
    let directory = workbook.parent().ok_or("generation")?;
    let before = std::fs::read(directory.join("frozen-input.json"))?;
    drop(dataset);
    crate::workbook::publication::verify_published(&workbook)?;
    let reopened = ExportDataset::reopen_frozen(&directory.join("frozen-input.json"))?;
    let selected = bests::build_from_dataset(&reopened, &best_options())?;
    for kind in [
        EventKind::Track300m,
        EventKind::Track500m,
        EventKind::Track600m,
        EventKind::Track2Miles,
        EventKind::Track2000m,
        EventKind::Track10000m,
    ] {
        check!(eq; selected.iter().filter(|row| row.key.event_kind == kind).count(), 1);
    }
    check!(eq; std::fs::read(directory.join("frozen-input.json"))?, before);
    for policy in [18, 19] {
        let mut historical = ExportDataset::reopen_frozen(&directory.join("frozen-input.json"))?;
        historical.lineage.policy_revision = policy;
        let refused_root = fixture
            .directory
            .path()
            .join(format!("historical-policy{policy}-publish-refused"));
        let mut options = options()?;
        options.out = Some(refused_root.clone());
        let censuses = Censuses::of(&historical, &fixture.store.out_dir(), &[]);
        check!(workbook::build_from(&historical, &fixture.store, &options, &censuses).is_err());
        check!(!refused_root.exists());
    }
    Ok(())
}

fn seed_flat_events(fixture: &Fixture) -> TestResult {
    let source = ExportDataset::load(&fixture.store)?;
    let ordinary = source
        .events
        .iter()
        .find(|event| event.id == fixture.ordinary)
        .ok_or("ordinary event")?;
    let performance = source
        .performances
        .iter()
        .find(|row| row.event == ordinary.id)
        .ok_or("ordinary performance")?;
    for label in ["300m", "500m", "600m", "2 miles", "2000m", "10000m"] {
        let mut event = CanonicalEvent::new(
            EventIdentity {
                meet: &ordinary.meet,
                kind: EventKind::from_source_label(label),
                gender: Gender::Boys,
                division: None,
                round: None,
            },
            EventSpecification::default(),
        )?;
        event.source_labels = ordinary
            .source_labels
            .iter()
            .map(|owned| SourceEventLabel {
                source: owned.source.clone(),
                label: label.into(),
            })
            .collect();
        event.evidence = ordinary.evidence.clone();
        let mut result = CanonicalPerformance::new(
            PerformanceIdentity {
                athlete: &fixture.athlete,
                event: &event.id,
                meet: &event.meet,
                date: &performance.date,
                source_key: label,
            },
            PerformanceResult {
                team: &performance.team,
                mark: Mark::TimeSeconds(ExactSeconds::parse("60.10")?),
                adjustment: None,
                wind_mps: None,
                place: Some(1),
            },
        )?;
        result.source_athlete = performance.source_athlete.clone();
        result.evidence = performance.evidence.clone();
        result.timing = Some(TimingMethod::Fat);
        fixture.store.append(Table::Events, &event)?;
        fixture.store.append(Table::Performances, &result)?;
    }
    Ok(())
}
