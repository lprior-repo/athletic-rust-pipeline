use super::*;
#[test]
fn cen10_throw_mass_known_unknown_and_category_have_exact_slots() -> TestResult {
    let mut data = dataset()?;
    for (name, label, mark) in [
        ("four_slow", "Boys Shot Put (4kg)", "1200"),
        ("four_fast", "ShotPut (4000g) Boys", "1300"),
        ("five", "Boys Shot Put (5kg)", "1500"),
        ("unknown", "Boys Shot Put", "2000"),
        ("pounds", "Boys Shot Put (12lb)", "1600"),
    ] {
        let meet = meet(name, Sport::OutdoorTrack, "2025-05-01");
        let event = event(&meet, label)?;
        add(&mut data, meet, event, mark)?;
    }
    let rows = selections(&data);
    let mut winners: Vec<_> = rows
        .iter()
        .map(|row| {
            (
                row.source
                    .specification
                    .implement
                    .map(ImplementMass::micrograms),
                &row.result.mark,
            )
        })
        .collect();
    winners.sort_by_key(|(mass, _)| *mass);
    assert_eq!(
        winners,
        vec![
            (None, &Mark::DistanceMetres(CentiMetres::new(2000))),
            (
                Some(4_000_000_000),
                &Mark::DistanceMetres(CentiMetres::new(1300))
            ),
            (
                Some(5_000_000_000),
                &Mark::DistanceMetres(CentiMetres::new(1500))
            ),
            (
                Some(5_443_108_440),
                &Mark::DistanceMetres(CentiMetres::new(1600))
            ),
        ]
    );
    Ok(())
}

#[test]
fn cen10_hurdle_height_spacing_and_unknown_specs_do_not_merge() -> TestResult {
    let mut data = dataset()?;
    for (name, label, time) in [
        ("33_slow", "Boys 60m Hurdles (33\")", "9.0"),
        ("33_fast", "Boys 60MetreHurdles (838.2mm)", "8.5"),
        ("39", "Boys 60m Hurdles (39\")", "8.0"),
        ("unknown", "Boys 60m Hurdles", "7.0"),
        ("spacing", "Boys 60m Hurdles (33\"/8.5m)", "8.2"),
    ] {
        let meet = meet(name, Sport::IndoorTrack, "2025-02-01");
        let event = event(&meet, label)?;
        add(&mut data, meet, event, time)?;
    }
    let rows = selections(&data);
    let winner = |height: Option<u32>, spacing: Option<u64>| {
        rows.iter().find(|row| {
            row.source.specification.hurdles
                == height.map(|height_micrometres| HurdleSpecification {
                    height_micrometres,
                    spacing_micrometres: spacing,
                })
        })
    };
    assert_eq!(rows.len(), 4);
    assert_eq!(
        winner(Some(838_200), None)
            .ok_or("33 inch winner")?
            .result
            .mark,
        Mark::TimeSeconds(ExactSeconds::parse("8.5")?)
    );
    assert_eq!(
        winner(Some(990_600), None)
            .ok_or("39 inch winner")?
            .result
            .mark,
        Mark::TimeSeconds(ExactSeconds::parse("8.0")?)
    );
    assert_eq!(
        winner(Some(838_200), Some(8_500_000))
            .ok_or("spacing winner")?
            .result
            .mark,
        Mark::TimeSeconds(ExactSeconds::parse("8.2")?)
    );
    assert_eq!(
        winner(None, None).ok_or("unknown winner")?.result.mark,
        Mark::TimeSeconds(ExactSeconds::parse("7.0")?)
    );
    Ok(())
}
