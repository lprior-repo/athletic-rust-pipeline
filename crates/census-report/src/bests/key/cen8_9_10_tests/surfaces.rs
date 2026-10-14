use super::*;
#[test]
fn cen10_banked_flat_unknown_and_published_category_remain_distinct() -> TestResult {
    let mut data = dataset()?;
    for (name, label, time) in [
        ("banked_slow", "Boys 1000m (200m banked)", "150"),
        ("banked_fast", "Boys 1000m (200m banked)", "145"),
        ("flat", "Boys 1000m (200m flat)", "140"),
        ("unknown", "Boys 1000m", "130"),
        ("long", "Boys 1000m (300m banked)", "135"),
        ("u18", "Boys U18 1000m (200m banked)", "125"),
        ("banking_unknown", "Boys 1000m (200m track)", "132"),
    ] {
        let meet = meet(name, Sport::IndoorTrack, "2025-02-01");
        let event = event(&meet, label)?;
        add(&mut data, meet, event, time)?;
    }
    let rows = selections(&data)?;
    check!(eq; rows.len(), 6);
    let expected_track = IndoorTrackSpecification::new(200_000_000, TrackBanking::Banked)?;
    let banked = rows
        .iter()
        .find(|row| {
            row.source.specification.category == Some(CompetitionCategory::Boys)
                && row.source.specification.indoor_track == Some(expected_track)
        })
        .ok_or("banked winner")?;
    check!(
        eq;
        banked.result.mark,
        Mark::TimeSeconds(ExactSeconds::parse("145")?)
    );
    let unknown = rows
        .iter()
        .find(|row| row.source.specification.indoor_track.is_none())
        .ok_or("unknown winner")?;
    check!(
        eq;
        unknown.result.mark,
        Mark::TimeSeconds(ExactSeconds::parse("130")?)
    );
    let unknown_banking = IndoorTrackSpecification::new(200_000_000, TrackBanking::Unknown)?;
    let unknown_banking = rows
        .iter()
        .find(|row| row.source.specification.indoor_track == Some(unknown_banking))
        .ok_or("unknown banking winner")?;
    check!(
        eq;
        unknown_banking.result.mark,
        Mark::TimeSeconds(ExactSeconds::parse("132")?)
    );
    let flat = IndoorTrackSpecification::new(200_000_000, TrackBanking::Flat)?;
    let flat = rows
        .iter()
        .find(|row| row.source.specification.indoor_track == Some(flat))
        .ok_or("flat winner")?;
    check!(
        eq;
        flat.result.mark,
        Mark::TimeSeconds(ExactSeconds::parse("140")?)
    );
    Ok(())
}
#[test]
fn cen10_unknown_category_never_merges_with_published_boys() -> TestResult {
    let mut data = dataset()?;
    for (name, label, gender, time) in [
        ("known_category", "Boys 1000m", Gender::Boys, "150"),
        ("unknown_category", "1000m", Gender::Unknown, "140"),
    ] {
        let meet = meet(name, Sport::IndoorTrack, "2025-02-01");
        let specification =
            EventSpecification::from_published_label(label, &EventKind::Track1000m)?;
        let event = event_with_specification(&meet, label, gender, specification)?;
        add(&mut data, meet, event, time)?;
    }
    let retained = serialized_bytes(&(&data.events, &data.performances))?;
    let admission = crate::report::EventAdmission::of(&data)?;
    check!(
        eq;
        admission.counts(),
        &crate::report::EventCounts {
            included: 1,
            excluded: std::collections::BTreeMap::new(),
            unresolved: std::collections::BTreeMap::from([(
                EventResolutionReason::UnresolvedCategoryOrContext,
                1,
            )]),
        }
    );
    let mut pure_keys = Vec::new();
    for event in &data.events {
        let performance = data
            .performances
            .iter()
            .find(|performance| performance.event == event.id)
            .ok_or("retained category performance")?;
        let meet = data
            .meets
            .iter()
            .find(|meet| meet.id == event.meet)
            .ok_or("retained category meet")?;
        let pure_key = crate::bests::PrKey::base(
            performance,
            &EventKind::Track1000m,
            crate::bests::SurfaceClass::Indoor,
            crate::bests::Measure::Time,
            &performance.athlete,
        )
        .qualify(event)
        .ok_or("pure comparison key")?;
        let checked_key = crate::bests::PrKey::from_performance(
            performance,
            event,
            Some(meet),
            crate::bests::Measure::Time,
            census_domain::model::EventSemantics::FlatEvents19,
        )?;
        match event.gender {
            Gender::Boys => {
                check!(
                    eq;
                    admission.disposition(&event.id),
                    Some(&EventDisposition::Included(EventKind::Track1000m))
                );
                check!(eq; checked_key, Some(pure_key.clone()));
                check!(eq; pure_key.specification.category, Some(CompetitionCategory::Boys));
                check!(eq; performance.mark, Mark::TimeSeconds(ExactSeconds::parse("150")?));
            }
            Gender::Unknown => {
                check!(
                    eq;
                    admission.disposition(&event.id),
                    Some(&EventDisposition::Unresolved(
                        EventResolutionReason::UnresolvedCategoryOrContext
                    ))
                );
                check!(eq; checked_key, None);
                check!(eq; pure_key.specification.category, None);
                check!(eq; performance.mark, Mark::TimeSeconds(ExactSeconds::parse("140")?));
            }
            other => return Err(format!("unexpected fixture category: {other:?}").into()),
        }
        pure_keys.push(pure_key);
    }
    check!(eq; pure_keys.len(), 2);
    check!(
        ne;
        pure_keys.first().ok_or("known category pure key")?,
        pure_keys.get(1).ok_or("unknown category pure key")?
    );
    let mut without_category = pure_keys.first().ok_or("known category pure key")?.clone();
    without_category.specification.category = None;
    check!(eq; &without_category, pure_keys.get(1).ok_or("unknown category pure key")?);
    let rows = selections(&data)?;
    check!(eq; rows.len(), 1);
    check!(
        eq;
        rows.first().ok_or("qualified category winner")?.source.specification.category,
        Some(CompetitionCategory::Boys)
    );
    check!(
        eq;
        rows.iter()
            .find(|row| row.source.specification.category == Some(CompetitionCategory::Boys))
            .ok_or("known category")?
            .result
            .mark,
        Mark::TimeSeconds(ExactSeconds::parse("150")?)
    );
    check!(eq; data.events.len(), 2);
    check!(eq; data.performances.len(), 2);
    check!(eq; serialized_bytes(&(&data.events, &data.performances))?, retained);
    Ok(())
}
