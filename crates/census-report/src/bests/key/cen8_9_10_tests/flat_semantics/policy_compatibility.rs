use super::*;

#[test]
fn explicit_policy_selects_four_distinct_flat_prs_without_conversions() -> TestResult {
    let mut data = dataset()?;
    for label in [
        "300m",
        "500m",
        "600m",
        "2 miles",
        "3200m",
        "3000m",
        "300m Hurdles",
    ] {
        for (date, seconds) in [("2025-01-01", "60.20"), ("2025-02-01", "60.10")] {
            let meet = meet(&format!("{label}/{date}"), Sport::OutdoorTrack, date);
            let mut event = event(&meet, label)?;
            if ["300m", "500m", "600m", "2 miles"].contains(&label) {
                let mut retained = CanonicalEvent::new(
                    EventIdentity {
                        meet: &meet.id,
                        kind: EventKind::Unmapped {
                            label: label.into(),
                        },
                        gender: Gender::Boys,
                        division: None,
                        round: None,
                    },
                    event.specification.clone(),
                )?;
                retained.source_labels = event.source_labels;
                retained.evidence = event.evidence;
                event = retained;
            }
            add(&mut data, meet, event, seconds)?;
        }
    }
    let current = selections(&data)?;
    check!(eq; current.len(), 7);
    check!(current.iter().all(|row| row.population.marks == 2
        && row.meet.date == "2025-02-01"
        && row.result.value == 60_100_000_000));
    data.lineage.policy_revision = 18;
    let historical = selections(&data)?;
    check!(eq; historical.len(), 3);
    check!(historical.iter().all(|row| matches!(
        row.key.event_kind,
        EventKind::Track3200m | EventKind::Track3000m | EventKind::Track300mHurdles
    )));
    Ok(())
}

#[test]
fn four_event_keys_keep_surface_timing_and_track_dimensions_separate() -> TestResult {
    let mut data = dataset()?;
    for (label, sport, timing) in [
        ("300m", Sport::OutdoorTrack, Some(TimingMethod::Fat)),
        ("300m", Sport::OutdoorTrack, Some(TimingMethod::Hand)),
        ("300m", Sport::OutdoorTrack, None),
        (
            "300m (200m banked)",
            Sport::IndoorTrack,
            Some(TimingMethod::Fat),
        ),
        (
            "300m (300m banked)",
            Sport::IndoorTrack,
            Some(TimingMethod::Fat),
        ),
        (
            "300m (200m flat)",
            Sport::IndoorTrack,
            Some(TimingMethod::Fat),
        ),
    ] {
        let meet = meet(
            &format!("{label}/{sport:?}/{timing:?}"),
            sport,
            "2025-01-01",
        );
        let event = event(&meet, label)?;
        add(&mut data, meet, event, "40.00")?;
        data.performances
            .last_mut()
            .ok_or("new performance")?
            .timing = timing;
    }
    let rows = selections(&data)?;
    check!(eq; rows.len(), 6);
    check!(rows
        .iter()
        .all(|row| row.key.event_kind == EventKind::Track300m));
    Ok(())
}

#[test]
fn direct_pr_construction_requires_the_callers_semantic_policy() -> TestResult {
    let mut data = dataset()?;
    for label in ["300m", "500m", "600m", "2 miles"] {
        let meet = meet(label, Sport::OutdoorTrack, "2025-01-01");
        let parsed = event(&meet, label)?;
        let mut retained = CanonicalEvent::new(
            EventIdentity {
                meet: &meet.id,
                kind: EventKind::Unmapped {
                    label: label.into(),
                },
                gender: Gender::Boys,
                division: None,
                round: None,
            },
            parsed.specification,
        )?;
        retained.source_labels = parsed.source_labels;
        retained.evidence = parsed.evidence;
        add(&mut data, meet, retained, "60.10")?;
    }
    check!(eq; data.events.len(), 4);
    for (event, expected) in data.events.iter().zip([
        EventKind::Track300m,
        EventKind::Track500m,
        EventKind::Track600m,
        EventKind::Track2Miles,
    ]) {
        let performance = data
            .performances
            .iter()
            .find(|row| row.event == event.id)
            .ok_or("performance")?;
        let meet = data
            .meets
            .iter()
            .find(|row| row.id == event.meet)
            .ok_or("meet")?;
        let old = crate::bests::PrKey::from_performance(
            performance,
            event,
            Some(meet),
            crate::bests::Measure::Time,
            EventSemantics::Legacy18,
        )?;
        check!(eq; old, None);
        let new = crate::bests::PrKey::from_performance(
            performance,
            event,
            Some(meet),
            crate::bests::Measure::Time,
            EventSemantics::FlatEvents19,
        )?
        .ok_or("current flat-event key")?;
        check!(eq; new.event_kind, expected);
    }
    Ok(())
}
