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
    let rows = selections(&data);
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
    let rows = selections(&data);
    check!(eq; rows.len(), 2);
    check!(
        eq;
        rows.iter()
            .find(|row| row.source.specification.category.is_none())
            .ok_or("unknown category")?
            .result
            .mark,
        Mark::TimeSeconds(ExactSeconds::parse("140")?)
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
    Ok(())
}
