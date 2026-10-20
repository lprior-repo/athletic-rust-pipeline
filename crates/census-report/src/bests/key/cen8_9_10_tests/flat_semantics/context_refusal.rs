use super::*;

#[test]
fn long_flat_public_pr_api_requires_explicit_current_policy_and_track_context() -> TestResult {
    for (label, kind) in [
        ("2000m", EventKind::Track2000m),
        ("2k", EventKind::Track2000m),
        ("2 K", EventKind::Track2000m),
        ("10000m", EventKind::Track10000m),
        ("10k", EventKind::Track10000m),
        ("10 K", EventKind::Track10000m),
    ] {
        let mut data = dataset()?;
        let meet = meet(label, Sport::OutdoorTrack, "2025-01-01");
        let event = event(&meet, label)?;
        add(&mut data, meet, event, "600.10")?;
        let original = data.events.first().ok_or("event")?.clone();
        let performance = data.performances.first().ok_or("performance")?;
        let meet = data.meets.first().ok_or("meet")?;
        let key = long_flat_key(
            &performance.athlete,
            kind,
            SurfaceClass::Outdoor,
            TimingClass::Fat,
            None,
        );
        let expected = LongFlatWinners::from([(
            key.clone(),
            (performance.id.clone(), meet.id.clone(), 600_100_000_000, 1),
        )]);
        check!(eq; PrKey::from_performance(performance, &original, Some(meet), Measure::Time, EventSemantics::FlatEvents19)?, Some(key));
        check_long_flat_winners(&selections(&data)?, &expected)?;
        for (sport, reason) in [
            (Sport::Unknown, EventResolutionReason::UnresolvedSport),
            (
                Sport::CrossCountry,
                EventResolutionReason::ContradictoryDeclarations,
            ),
        ] {
            data.meets.first_mut().ok_or("meet")?.sports = vec![sport];
            check_long_flat_refusal(&data, reason)?;
        }
        data.meets.first_mut().ok_or("meet")?.sports = vec![Sport::OutdoorTrack];
        data.events.first_mut().ok_or("event")?.gender = Gender::Unknown;
        check_long_flat_refusal(&data, EventResolutionReason::UnresolvedCategoryOrContext)?;
        *data.events.first_mut().ok_or("event")? = original.clone();
        data.events.first_mut().ok_or("event")?.evidence.clear();
        check_long_flat_refusal(&data, EventResolutionReason::MissingSourceOwnership)?;
        *data.events.first_mut().ok_or("event")? = original;
        check_long_flat_winners(&selections(&data)?, &expected)?;
        data.lineage.policy_revision = 18;
        check!(selections(&data).is_err());
    }
    Ok(())
}
