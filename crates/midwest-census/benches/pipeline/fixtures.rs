//! The census fixture: the performance arithmetic the pipeline benches feed through the merge.
use super::*;

/// `PERFORMANCES` performances, each observed `OBSERVATIONS_PER_PERFORMANCE` times, spread over one
/// championship meet per jurisdiction so the batch covers the country instead of one state.
pub(super) fn performance_observations() -> Result<Vec<CanonicalPerformance>> {
    let mut batch = Vec::with_capacity(PERFORMANCES.saturating_mul(OBSERVATIONS_PER_PERFORMANCE));
    let jurisdictions = UsJurisdiction::ALL.iter().cycle().copied();
    for (index, jurisdiction) in (0..PERFORMANCES).zip(jurisdictions) {
        let name = format!("{SCHOOL_PREFIX} {} 000", jurisdiction.code());
        let school = CanonicalSchool::mint(jurisdiction, &name, &normalize_name(&name));
        let meet = CanonicalMeet::mint(Some(jurisdiction), MEET_DATE, MEET_NAME, None);
        let athlete_name = format!("Athlete {index:05}");
        let athlete =
            CanonicalAthlete::mint(&school, &athlete_name, GradYear::CO2027, Gender::Boys);
        let team = CanonicalTeam::mint(
            &school,
            Sport::OutdoorTrack,
            Gender::Boys,
            SchoolYear::new(2024).expect("2024 is a season"),
        );
        let kind = EventKind::Track800m;
        let event = CanonicalEvent::new(&meet, kind.clone(), Gender::Boys, None, Some("finals"));
        let source_key = format!("bench:{index:05}");
        let id = CanonicalPerformance::mint(&athlete, &meet, &kind, MEET_DATE, &source_key);
        let step = u32::try_from(index % 900).context("a mark step does not fit u32")?;
        let mark = Mark::TimeSeconds(120.0 + f64::from(step) / 100.0);
        for observation in 0..OBSERVATIONS_PER_PERFORMANCE {
            let seen = u16::try_from(observation).context("an observation does not fit u16")?;
            let first = observation == 0;
            let timing = if first {
                TimingMethod::Fat
            } else {
                TimingMethod::Hand
            };
            let observed_grade = Grade::new(if first { 11 } else { 12 });
            batch.push(CanonicalPerformance {
                id: id.clone(),
                athlete: athlete.clone(),
                team: team.clone(),
                event: event.id.clone(),
                meet: meet.clone(),
                date: MEET_DATE.to_string(),
                mark: mark.clone(),
                wind_mps: Some(f64::from(seen) + 1.2),
                place: Some(seen.saturating_add(1)),
                heat: None,
                round: Some("finals".to_string()),
                timing: Some(timing),
                observed_grade,
                evidence: vec![Evidence::parsed(
                    SourceRef::new("bench", None),
                    format!("2025-06-{:02}", seen.saturating_add(6)),
                )],
                source_key: source_key.clone(),
                retained_conflicts: Vec::new(),
            });
        }
    }
    Ok(batch)
}

/// `<crawl crate>/tests/fixtures/<dir>/<file>`, read as UTF-8: the path resolution `benches/core.rs`
/// uses, from the manifest directory at compile time.
pub(super) fn fixture(dir: &str, file: &str) -> Result<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../census-crawl/tests/fixtures")
        .join(dir)
        .join(file);
    fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))
}
