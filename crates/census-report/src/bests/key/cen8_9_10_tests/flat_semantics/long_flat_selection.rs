use super::*;

#[test]
fn long_flat_prs_select_exact_distances_without_steeple_or_legacy_remapping() -> TestResult {
    let mut data = dataset()?;
    let mut expected = LongFlatWinners::new();
    let mut historical = LongFlatWinners::new();
    for (kind, first, second) in [
        (EventKind::Track2000m, "2000m", "2k"),
        (EventKind::Track10000m, "10000m", "10k"),
        (EventKind::Track3000m, "3000m", "3000m"),
        (
            EventKind::Track2000mSteeplechase,
            "2000m Steeplechase",
            "2k Steeplechase",
        ),
    ] {
        for (label, date, seconds) in [
            (first, "2025-01-01", "600.20"),
            (second, "2025-02-01", "600.10"),
        ] {
            let meet = meet(&format!("{label}/{date}"), Sport::OutdoorTrack, date);
            let parsed = event(&meet, label)?;
            let retained = if ["2000m", "2k", "10000m", "10k"].contains(&label) {
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
                retained
            } else {
                parsed
            };
            add(&mut data, meet, retained, seconds)?;
        }
        let winner = data.performances.last().ok_or("expected alias winner")?;
        let key = long_flat_key(
            &winner.athlete,
            kind.clone(),
            SurfaceClass::Outdoor,
            TimingClass::Fat,
            None,
        );
        let witness = (winner.id.clone(), winner.meet.clone(), 600_100_000_000, 2);
        check!(eq; expected.insert(key.clone(), witness.clone()), None);
        if matches!(
            kind,
            EventKind::Track3000m | EventKind::Track2000mSteeplechase
        ) {
            check!(eq; historical.insert(key, witness), None);
        }
    }
    let retained = serialized_bytes(&(&data.events, &data.performances))?;
    check_long_flat_winners(&selections(&data)?, &expected)?;
    data.lineage.policy_revision = 18;
    check_long_flat_winners(&selections(&data)?, &historical)?;
    check!(eq; serialized_bytes(&(&data.events, &data.performances))?, retained);
    Ok(())
}

#[test]
fn long_flat_aliases_have_exact_winners_for_independently_varied_comparison_dimensions(
) -> TestResult {
    let mut data = dataset()?;
    let mut expected = LongFlatWinners::new();
    for (kind, aliases) in [
        (EventKind::Track2000m, ["2000m", "2k", "2 K"]),
        (EventKind::Track10000m, ["10000m", "10k", "10 K"]),
    ] {
        for (suffix, sport, surface, timing, timing_class, facility) in [
            (
                "",
                Sport::OutdoorTrack,
                SurfaceClass::Outdoor,
                Some(TimingMethod::Fat),
                TimingClass::Fat,
                None,
            ),
            (
                "",
                Sport::OutdoorTrack,
                SurfaceClass::Outdoor,
                Some(TimingMethod::Hand),
                TimingClass::Hand,
                None,
            ),
            (
                "",
                Sport::OutdoorTrack,
                SurfaceClass::Outdoor,
                None,
                TimingClass::Unknown,
                None,
            ),
            (
                "",
                Sport::IndoorTrack,
                SurfaceClass::Indoor,
                Some(TimingMethod::Fat),
                TimingClass::Fat,
                None,
            ),
            (
                " (200m banked)",
                Sport::IndoorTrack,
                SurfaceClass::Indoor,
                Some(TimingMethod::Fat),
                TimingClass::Fat,
                Some(IndoorTrackSpecification::new(
                    200_000_000,
                    TrackBanking::Banked,
                )?),
            ),
            (
                " (300m banked)",
                Sport::IndoorTrack,
                SurfaceClass::Indoor,
                Some(TimingMethod::Fat),
                TimingClass::Fat,
                Some(IndoorTrackSpecification::new(
                    300_000_000,
                    TrackBanking::Banked,
                )?),
            ),
            (
                " (200m flat)",
                Sport::IndoorTrack,
                SurfaceClass::Indoor,
                Some(TimingMethod::Fat),
                TimingClass::Fat,
                Some(IndoorTrackSpecification::new(
                    200_000_000,
                    TrackBanking::Flat,
                )?),
            ),
        ] {
            for (alias, (date, seconds)) in aliases.into_iter().zip([
                ("2025-01-01", "600.30"),
                ("2025-02-01", "600.10"),
                ("2025-03-01", "600.20"),
            ]) {
                let label = format!("{alias}{suffix}");
                let meet = meet(&format!("{label}/{sport:?}/{timing:?}/{date}"), sport, date);
                let event = event(&meet, &label)?;
                add(&mut data, meet, event, seconds)?;
                let performance = data.performances.last_mut().ok_or("new performance")?;
                performance.timing = timing;
                if date == "2025-02-01" {
                    let key = long_flat_key(
                        &performance.athlete,
                        kind.clone(),
                        surface,
                        timing_class,
                        facility,
                    );
                    check!(eq; expected.insert(key, (performance.id.clone(), performance.meet.clone(), 600_100_000_000, 3)), None);
                }
            }
        }
    }
    check!(eq; data.performances.len(), 42);
    check!(eq; expected.len(), 14);
    let retained = serialized_bytes(&(&data.events, &data.performances))?;
    check_long_flat_winners(&selections(&data)?, &expected)?;
    data.lineage.policy_revision = 18;
    check_long_flat_winners(&selections(&data)?, &LongFlatWinners::new())?;
    check!(eq; serialized_bytes(&(&data.events, &data.performances))?, retained);
    Ok(())
}

#[test]
fn short_flat_indoor_aliases_pool_only_with_identical_facility_dimensions() -> TestResult {
    let mut data = dataset()?;
    let mut expected = LongFlatWinners::new();
    for (suffix, length, banking) in [
        ("(200m banked)", 200_000_000, TrackBanking::Banked),
        ("(300m banked)", 300_000_000, TrackBanking::Banked),
        ("(200m flat)", 200_000_000, TrackBanking::Flat),
    ] {
        for (alias, date, seconds) in [
            ("2000m", "2025-01-01", "600.30"),
            ("2k", "2025-02-01", "600.20"),
            ("2 K", "2025-03-01", "600.10"),
        ] {
            let label = format!("{alias} {suffix}");
            let meet = meet(&format!("{label}/{date}"), Sport::IndoorTrack, date);
            let parsed = event(&meet, &label)?;
            let mut retained = CanonicalEvent::new(
                EventIdentity {
                    meet: &meet.id,
                    kind: EventKind::Unmapped {
                        label: label.clone(),
                    },
                    gender: Gender::Boys,
                    division: None,
                    round: None,
                },
                parsed.specification,
            )?;
            retained.source_labels = parsed.source_labels;
            retained.evidence = parsed.evidence;
            add(&mut data, meet, retained, seconds)?;
        }
        let winner = data
            .performances
            .last()
            .ok_or("retained short-alias winner")?;
        let key = long_flat_key(
            &winner.athlete,
            EventKind::Track2000m,
            SurfaceClass::Indoor,
            TimingClass::Fat,
            Some(IndoorTrackSpecification::new(length, banking)?),
        );
        check!(eq; expected.insert(key, (winner.id.clone(), winner.meet.clone(), 600_100_000_000, 3)), None);
    }
    let retained = serialized_bytes(&(&data.events, &data.performances))?;
    let rows = selections(&data)?;
    check_long_flat_winners(&rows, &expected)?;
    data.lineage.policy_revision = 18;
    check_long_flat_winners(&selections(&data)?, &LongFlatWinners::new())?;
    check!(eq; serialized_bytes(&(&data.events, &data.performances))?, retained);
    Ok(())
}
