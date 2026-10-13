use super::*;
use census_domain::model::{
    CatalogError, EventDisposition, EventExclusionReason, EventResolutionReason, Evidence,
    RecruitingCatalogV1, SourceEventLabel, SourceRef,
};

fn catalog_event(kind: EventKind, label: &str, meet: &MeetId) -> TestResult<CanonicalEvent> {
    let mut event = test_event(kind, meet)?;
    event.source_labels.clear();
    event.evidence.clear();
    let source = SourceRef::new(
        "catalog_fixture",
        Some("https://example.invalid/catalog/results".into()),
    );
    event.source_labels.push(SourceEventLabel {
        source: source.clone(),
        label: label.into(),
    });
    event.evidence.push(Evidence::parsed(source, "2025-03-01"));
    Ok(event)
}

fn catalog_performance(
    event: &CanonicalEvent,
    label: &str,
    athlete: &AthleteId,
) -> TestResult<CanonicalPerformance> {
    let mut performance = test_performance(
        event.kind.clone(),
        Mark::TimeSeconds(ExactSeconds::parse("10.94")?),
        Some(TimingMethod::Fat),
        Some(1.0),
        "2025-03-01",
    );
    performance.id = Id::mint("perf", &[label]);
    performance.event = event.id.clone();
    performance.meet = event.meet.clone();
    performance.athlete = athlete.clone();
    performance.source_key = label.into();
    Ok(performance)
}

#[test]
fn checked_public_key_admits_only_the_positive_event_for_one_athlete() -> TestResult {
    let meet = test_meet(vec![Sport::OutdoorTrack]);
    let athlete = athlete_id();
    let mut keys = Vec::new();
    for (label, kind, expected) in [
        (
            "100m",
            EventKind::Track100m,
            EventDisposition::Included(EventKind::Track100m),
        ),
        (
            "100m Wheelchair",
            EventKind::Unmapped {
                label: "100m Wheelchair".into(),
            },
            EventDisposition::Excluded(EventExclusionReason::WheelchairRace),
        ),
        (
            "100m Speedwalk",
            EventKind::Track100m,
            EventDisposition::Excluded(EventExclusionReason::CompetitiveWalking),
        ),
        (
            "600m",
            EventKind::Unmapped {
                label: "600m".into(),
            },
            EventDisposition::Unresolved(EventResolutionReason::UnsupportedScholasticVariant),
        ),
    ] {
        let event = catalog_event(kind, label, &meet.id)?;
        let performance = catalog_performance(&event, label, &athlete)?;
        check!(eq; RecruitingCatalogV1::new().classify(&event, Sport::OutdoorTrack)?, expected);
        if let Some(key) = crate::bests::PrKey::from_performance(
            &performance,
            &event,
            Some(&meet),
            crate::bests::Measure::Time,
        )? {
            keys.push(key);
        }
        check!(eq; performance.athlete, athlete);
    }
    check!(eq; keys.len(), 1);
    check!(eq; keys.first().ok_or("positive key")?.event_kind, EventKind::Track100m);
    check!(eq; keys.first().ok_or("positive key")?.athlete_id, athlete);
    Ok(())
}

#[test]
fn numeric_unmapped_missing_ownership_and_conflicting_labels_do_not_enter_pr_keys() -> TestResult {
    let meet = test_meet(vec![Sport::OutdoorTrack]);
    for label in ["700m", "2 Mile", "100m mystery", "100m <span>"] {
        let event = catalog_event(
            EventKind::Unmapped {
                label: label.into(),
            },
            label,
            &meet.id,
        )?;
        let performance = catalog_performance(&event, label, &athlete_id())?;
        check!(crate::bests::PrKey::from_performance(&performance, &event, Some(&meet), crate::bests::Measure::Time)?.is_none());
    }
    let mut event = catalog_event(EventKind::Track100m, "100m", &meet.id)?;
    let performance = catalog_performance(&event, "missing_owner", &athlete_id())?;
    event.evidence.clear();
    check!(crate::bests::PrKey::from_performance(&performance, &event, Some(&meet), crate::bests::Measure::Time)?.is_none());
    let mut event = catalog_event(EventKind::Track100m, "100m", &meet.id)?;
    let source = event
        .source_labels
        .first()
        .ok_or("fixture label")?
        .source
        .clone();
    event.source_labels.push(SourceEventLabel {
        source,
        label: "100m Wheelchair".into(),
    });
    let performance = catalog_performance(&event, "mixed_labels", &athlete_id())?;
    check!(crate::bests::PrKey::from_performance(&performance, &event, Some(&meet), crate::bests::Measure::Time)?.is_none());
    Ok(())
}

#[test]
fn catalog_input_failure_is_a_typed_public_key_error() -> TestResult {
    let meet = test_meet(vec![Sport::OutdoorTrack]);
    let mut event = catalog_event(EventKind::Track100m, "100m", &meet.id)?;
    let performance = catalog_performance(&event, "oversize", &athlete_id())?;
    event
        .source_labels
        .first_mut()
        .ok_or("fixture label")?
        .label = "x".repeat(4097);
    check!(eq; crate::bests::PrKey::from_performance(&performance, &event, Some(&meet), crate::bests::Measure::Time), Err(CatalogError::LabelBytesExceeded));
    Ok(())
}

fn append_catalog_meet(store: &Store) -> TestResult {
    let (school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Catalog School",
        "catalog school",
        None,
    );
    let mut athlete = CanonicalAthlete::new(
        &school_id,
        "Catalog runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "catalog_runner"),
    );
    athlete
        .published_graduations
        .push(census_domain::model::PublishedGraduation {
            grad_year: GradYear::CO2027,
            source: SourceRef::new(
                "catalog_roster",
                Some("https://example.invalid/catalog/roster".into()),
            ),
        });
    let meet = test_meet(vec![Sport::OutdoorTrack]);
    store.append(Table::Schools, &school)?;
    store.append(Table::Athletes, &athlete)?;
    store.append(Table::Meets, &meet)?;
    for (label, kind) in [
        ("100m", EventKind::Track100m),
        (
            "Wheelchair 100m",
            EventKind::Unmapped {
                label: "Wheelchair 100m".into(),
            },
        ),
        (
            "Racewalk 100m",
            EventKind::Unmapped {
                label: "Racewalk 100m".into(),
            },
        ),
        (
            "600m",
            EventKind::Unmapped {
                label: "600m".into(),
            },
        ),
    ] {
        let event = catalog_event(kind, label, &meet.id)?;
        let mut performance = catalog_performance(&event, label, &athlete.id)?;
        performance.source_athlete = athlete.source.clone();
        store.append(Table::Events, &event)?;
        store.append(Table::Performances, &performance)?;
    }
    Ok(())
}

fn catalog_readback(dataset: &ExportDataset) -> TestResult {
    check!(eq; dataset.events.len(), 4);
    check!(eq; dataset.performances.len(), 4);
    let mut included = 0usize;
    let mut excluded = 0usize;
    let mut unresolved = 0usize;
    let mut keys = Vec::new();
    for event in &dataset.events {
        match RecruitingCatalogV1::new().classify(event, Sport::OutdoorTrack)? {
            EventDisposition::Included(EventKind::Track100m) => {
                included = included.checked_add(1).ok_or("included count")?
            }
            EventDisposition::Included(_) => return Err("unexpected included family".into()),
            EventDisposition::Excluded(_) => {
                excluded = excluded.checked_add(1).ok_or("excluded count")?
            }
            EventDisposition::Unresolved(EventResolutionReason::UnsupportedScholasticVariant) => {
                unresolved = unresolved.checked_add(1).ok_or("unresolved count")?
            }
            EventDisposition::Unresolved(_) => return Err("unexpected unresolved reason".into()),
        }
        let performance = dataset
            .performances
            .iter()
            .find(|performance| performance.event == event.id)
            .ok_or("performance")?;
        let meet = dataset
            .meets
            .iter()
            .find(|meet| meet.id == event.meet)
            .ok_or("meet")?;
        if let Some(key) = crate::bests::PrKey::from_performance(
            performance,
            event,
            Some(meet),
            crate::bests::Measure::Time,
        )? {
            keys.push(key);
        }
    }
    check!(eq; (included, excluded, unresolved), (1, 2, 1));
    check!(eq; keys.len(), 1);
    check!(eq; keys.first().ok_or("eligible key")?.event_kind, EventKind::Track100m);

    Ok(())
}

#[test]
fn whole_meet_reused_store_retains_evidence_and_rebuilds_the_same_checked_key_membership(
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    append_catalog_meet(&store)?;
    let dataset = ExportDataset::load(&store)?;
    catalog_readback(&dataset)?;
    let before = census_domain::model::serialized_bytes(&dataset.events)?;
    drop(dataset);
    drop(store);
    let store = Store::open(directory.path())?;
    let reopened = ExportDataset::load(&store)?;
    catalog_readback(&reopened)?;
    check!(eq; census_domain::model::serialized_bytes(&reopened.events)?, before);
    Ok(())
}
