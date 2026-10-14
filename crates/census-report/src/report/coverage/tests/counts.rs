use super::*;

#[test]
fn a_jurisdiction_row_carries_the_counted_columns() -> TestResult {
    let (_dir, store) = fixture_store()?;

    let report = coverage_of(&store, Some(2027))?;

    let wi = row(&report, "WI")?;
    check!(eq; wi.schools, 1);
    check!(eq; wi.schools_with_athletes, 1);
    check!(eq; wi.athletes, 2);
    check!(eq; wi.athletes_core, 1);
    check!(eq; wi.core_share_pct, Some(50));
    check!(eq; wi.boys, 1);
    check!(eq; wi.girls, 1);
    check!(eq; wi.grad_verified, 1);
    check!(eq; wi.grad_unresolved, 1);
    check!(eq; wi.outdoor_track, 1);
    check!(eq; wi.indoor_track, 0);
    check!(eq; wi.cross_country, 0);
    check!(eq; wi.with_performance, 2);
    check!(eq; wi.with_comparable_mark, 1);
    check!(eq; wi.multisource, 0);
    check!(eq; wi.with_profile_url, 1);
    check!(eq; wi.with_milesplit_url, 1);
    check!(eq; wi.with_athletic_net_url, 0);
    check!(eq; wi.coaches, 2);
    check!(eq; wi.coaches_with_email, 1);
    check!(eq; wi.schools_with_tf_coach, 1);
    check!(eq; wi.schools_with_xc_coach, 1);
    check!(eq; wi.schools_with_coach_email, 1);
    check!(eq; wi.meets, 1);
    check!(eq; wi.performances, 3);
    check!(eq; wi.sources.get("milesplit_roster"), Some(&1));
    check!(eq; wi.sources.get("athleticlive_athletes"), Some(&1));

    let mn = row(&report, "MN")?;
    check!(eq; mn.schools, 1);
    check!(eq; mn.athletes, 1);
    check!(eq; mn.coaches, 0);

    let ks = row(&report, "KS")?;
    check!(eq; ks.schools, 1);
    check!(eq; ks.athletes, 0);
    check!(eq; ks.core_share_pct, None);

    let unplaced = row(&report, "UNKNOWN")?;
    check!(eq; unplaced.schools, 0);
    check!(eq; unplaced.athletes, 1);
    check!(eq; unplaced.performances, 1);
    check!(eq; unplaced.with_performance, 0);
    check!(eq; unplaced.with_comparable_mark, 0);
    Ok(())
}

#[test]
fn a_mirror_result_plane_row_is_counted_but_never_core() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Ohio,
        "Dublin Coffman",
        "dublin coffman",
        None,
    );
    store.append(Table::Schools, &school)?;
    let mut core_athlete = CanonicalAthlete::new(
        &school_id,
        "Core Runner",
        GradYear::CO2027,
        Gender::Boys,
        fixture_source("oh-core"),
    )?;
    core_athlete.evidence.push(evidence("ohsaa_results"));
    store.append(Table::Athletes, &core_athlete)?;
    let mut mirror_athlete = CanonicalAthlete::new(
        &school_id,
        "Mirror Runner",
        GradYear::CO2027,
        Gender::Girls,
        fixture_source("oh-mirror"),
    )?;
    mirror_athlete
        .evidence
        .push(evidence("athleticlive_results"));
    store.append(Table::Athletes, &mirror_athlete)?;

    let report = coverage_of(&store, Some(2027))?;
    let ohio = row(&report, "OH")?;
    check!(eq; ohio.athletes, 2);
    check!(eq; ohio.athletes_core, 1);
    check!(eq; ohio.core_share_pct, Some(50));
    check!(eq; ohio.sources.get("athleticlive_results"), Some(&1));
    check!(eq; ohio.sources.get("ohsaa_results"), Some(&1));

    let core = census_of(&store, crate::report::Scope::Core)?;
    check!(eq; core.totals.athletes, 1);
    Ok(())
}

#[test]
fn coverage_notes_include_lifecycle_information() -> TestResult {
    let (_dir, store) = fixture_store()?;

    let report = coverage_of(&store, Some(2027))?;

    let has_source_lifecycle = report
        .notes
        .iter()
        .any(|note| note.contains("source lifecycle:"));
    let has_contact_attempts = report
        .notes
        .iter()
        .any(|note| note.contains("contact attempts:"));
    let has_coverage_denominators = report
        .notes
        .iter()
        .any(|note| note.contains("coverage denominators:"));

    check!(has_source_lifecycle, "missing source lifecycle note");
    check!(has_contact_attempts, "missing contact attempts note");
    check!(
        has_coverage_denominators,
        "missing coverage denominators note"
    );

    report.reconcile()?;
    Ok(())
}

#[test]
fn converted_marks_carry_labels_but_never_count_as_pr_support() -> TestResult {
    let (_dir, store, converted_id, native_id) = conversion_coverage_store()?;
    let dataset = crate::export::ExportDataset::load(&store)?;
    let bests = crate::bests::build_from_dataset(
        &dataset,
        &crate::bests::Options {
            scope: crate::report::Scope::AllSources,
            grad_year: Some(2027),
            limit: None,
            jurisdictions: Vec::new(),
        },
    )?;
    check!(eq; bests.len(), 2);
    let converted_row = bests
        .iter()
        .find(|row| row.athlete_id() == &converted_id)
        .ok_or("missing converted best")?;
    check!(
        converted_row.result.adjustment_kind.is_some(),
        "converted carries kind"
    );
    check!(
        eq;
        converted_row.result.adjustment_note,
        Some("Converted from 9:22.21 for Track Size. ".to_string())
    );
    let native_row = bests
        .iter()
        .find(|row| row.athlete_id() == &native_id)
        .ok_or("missing native best")?;
    check!(eq; native_row.result.adjustment_kind, None);
    check!(eq; native_row.result.adjustment_note, None);
    let report = coverage_of(&store, Some(2027))?;
    let wi = row(&report, "WI")?;
    check!(eq; wi.with_performance, 2);
    check!(eq; wi.with_comparable_mark, 2);
    check!(eq; wi.with_pr_support, 1, "only the native winner counts");
    check!(eq; gap(&report, "WI", GapClass::MissingPrSupport), Some(1));
    report.reconcile()?;
    Ok(())
}

fn conversion_coverage_store() -> TestResult<(
    tempfile::TempDir,
    Store,
    census_domain::model::AthleteId,
    census_domain::model::AthleteId,
)> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (converted_athlete, native_athlete, meet) = conversion_people(&store)?;
    let (distance, mile) = conversion_events(&store, &meet)?;
    conversion_performances(
        &store,
        &converted_athlete,
        &native_athlete,
        &meet,
        &distance,
        &mile,
    )?;
    Ok((dir, store, converted_athlete.id, native_athlete.id))
}

fn conversion_people(
    store: &Store,
) -> TestResult<(CanonicalAthlete, CanonicalAthlete, CanonicalMeet)> {
    let (school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Conversion High",
        "conversion high",
        None,
    );
    store.append(Table::Schools, &school)?;
    let mut converted_athlete = CanonicalAthlete::new(
        &school_id,
        "Converted Runner",
        GradYear::CO2027,
        Gender::Boys,
        fixture_source("converted"),
    )?;
    converted_athlete.evidence.push(evidence("wiaa_results"));
    converted_athlete
        .observed_grades
        .push(observed_grade(11, 2025, "wiaa_results")?);
    store.append(Table::Athletes, &converted_athlete)?;
    let mut native_athlete = CanonicalAthlete::new(
        &school_id,
        "Native Runner",
        GradYear::CO2027,
        Gender::Boys,
        fixture_source("native"),
    )?;
    native_athlete.evidence.push(evidence("wiaa_results"));
    native_athlete
        .observed_grades
        .push(observed_grade(11, 2025, "wiaa_results")?);
    store.append(Table::Athletes, &native_athlete)?;
    let mut meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "Conversion Invite",
        "2026-05-01",
        CompetitionLevel::Invitational,
    );
    meet.sports.push(Sport::OutdoorTrack);
    store.append(Table::Meets, &meet)?;
    Ok((converted_athlete, native_athlete, meet))
}

fn conversion_events(
    store: &Store,
    meet: &CanonicalMeet,
) -> TestResult<(CanonicalEvent, CanonicalEvent)> {
    let distance = crate::test_events::owned(CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::Track3200m,
            gender: Gender::Boys,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?);
    store.append(Table::Events, &distance)?;
    let mile = crate::test_events::owned(CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::Track1600m,
            gender: Gender::Boys,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?);
    store.append(Table::Events, &mile)?;
    Ok((distance, mile))
}

fn conversion_performances(
    store: &Store,
    converted_athlete: &CanonicalAthlete,
    native_athlete: &CanonicalAthlete,
    meet: &CanonicalMeet,
    distance: &CanonicalEvent,
    mile: &CanonicalEvent,
) -> TestResult<()> {
    use census_domain::model::Adjustment;
    let mut conversion = performance(
        converted_athlete,
        &distance.id,
        &meet.id,
        Mark::TimeSeconds(ExactSeconds::parse_clock("9:15.75")?),
        ("wiaa_results", "conversion-only"),
    )?;
    conversion.round = Some("1".to_string());
    conversion.adjustment = Some(Adjustment::track_size_conversion(
        Some(Mark::TimeSeconds(ExactSeconds::parse_clock("9:22.21")?)),
        conversion.mark.clone(),
        Some("Converted from 9:22.21 for Track Size. ".to_string()),
        None,
    ));
    store.append(Table::Performances, &conversion)?;
    let mut native = performance(
        native_athlete,
        &mile.id,
        &meet.id,
        Mark::TimeSeconds(ExactSeconds::parse_clock("4:30.00")?),
        ("wiaa_results", "native-only"),
    )?;
    native.round = Some("1".to_string());
    store.append(Table::Performances, &native)?;
    Ok(())
}
