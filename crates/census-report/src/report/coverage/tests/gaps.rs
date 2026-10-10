use super::*;
use census_domain::model::CentiPoints;

fn dated_performance(
    athlete: &CanonicalAthlete,
    event: &census_domain::model::EventId,
    meet: &census_domain::model::MeetId,
    date: &str,
    mark: Mark,
    provenance: (&str, &str),
) -> TestResult<CanonicalPerformance> {
    let (source, source_key) = provenance;
    Ok(CanonicalPerformance {
        id: CanonicalPerformance::mint(&athlete.id, meet, event, date, source_key),
        athlete: athlete.id.clone(),
        team: CanonicalTeam::mint(
            &athlete.school,
            Sport::OutdoorTrack,
            Gender::Boys,
            SchoolYear::new(2026).ok_or("invalid fixture season")?,
        ),
        event: event.clone(),
        meet: meet.clone(),
        date: date.to_string(),
        mark,
        wind_mps: None,
        place: None,
        heat: None,
        round: None,
        timing: None,
        observed_grade: None,
        evidence: vec![evidence(source)],
        source_key: source_key.to_string(),
        source_athlete: athlete.source.clone(),
        retained_conflicts: Vec::new(),
    })
}

fn contested_store() -> TestResult<(tempfile::TempDir, Store)> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Contested High",
        "contested high",
        None,
    );
    store.append(Table::Schools, &school)?;
    let mut athlete = CanonicalAthlete::new(
        &school_id,
        "Contested Runner",
        GradYear::CO2027,
        Gender::Boys,
        fixture_source("contested"),
    );
    athlete.evidence.push(evidence("wiaa_results"));
    athlete
        .observed_grades
        .push(observed_grade(11, 2025, "wiaa_results")?);
    store.append(Table::Athletes, &athlete)?;
    let mut meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "Contested Invite",
        "2026-05-01",
        CompetitionLevel::Invitational,
    );
    meet.sports.push(Sport::OutdoorTrack);
    store.append(Table::Meets, &meet)?;
    let event = CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::Track400m,
            gender: Gender::Boys,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?;
    store.append(Table::Events, &event)?;
    for (label, seconds) in [("contest-first", "50.00"), ("contest-second", "51.00")] {
        store.append(
            Table::Performances,
            &dated_performance(
                &athlete,
                &event.id,
                &meet.id,
                "2026-05-01",
                Mark::TimeSeconds(ExactSeconds::parse(seconds)?),
                ("wiaa_results", label),
            )?,
        )?;
    }
    Ok((dir, store))
}

fn published_bests(store: &Store) -> TestResult<Vec<crate::bests::SharedSelection>> {
    let dataset = crate::export::ExportDataset::load(store)?;
    Ok(crate::bests::build_from_dataset(
        &dataset,
        &crate::bests::Options {
            scope: crate::report::Scope::AllSources,
            grad_year: Some(2027),
            limit: None,
        },
    ))
}

#[test]
fn an_empty_store_publishes_every_jurisdiction_with_a_gap() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;

    let report = coverage_of(&store, None)?;

    check!(eq; report.jurisdictions.len(),
    UsJurisdiction::CENSUS_SCOPE.len().saturating_add(1));
    check!(eq; report
        .jurisdictions
        .last()
        .ok_or("missing unplaced row")?
        .jurisdiction
        .code(),
    "UNKNOWN");
    check!(report
        .jurisdictions
        .iter()
        .all(|row| row.athletes == 0 && row.schools == 0 && row.core_share_pct == 0));
    check!(report
        .jurisdictions
        .iter()
        .any(|row| row.jurisdiction.code() == "WI" && row.schools == 0));

    check!(eq; report.gaps.len(),
    UsJurisdiction::CENSUS_SCOPE.len().saturating_add(1));
    check!(report.gaps.iter().all(|gap| gap
        .jurisdiction
        .jurisdiction()
        .is_none_or(|jurisdiction| jurisdiction.is_in_census_scope())));
    check!(report.gaps.iter().all(|gap| {
        gap.class == GapClass::EmptyJurisdiction && gap.count == 0 && gap.unit == "schools"
    }));

    check!(eq; report.read, CoverageTotals::default());
    check!(eq; report.off_cohort_athletes, 0);
    check!(report
        .notes
        .iter()
        .all(|note| !note.contains("outside the census run scope")));
    report.reconcile()?;
    Ok(())
}

#[test]
fn gap_classes_carry_the_count_that_produced_them() -> TestResult {
    let (_dir, store) = fixture_store()?;

    let report = coverage_of(&store, Some(2027))?;

    check!(eq; gap(&report, "WI", GapClass::MissingGraduationEvidence),
    Some(1));
    check!(eq; gap(&report, "WI", GapClass::MissingProfile), Some(1));
    check!(eq; gap(&report, "WI", GapClass::MissingPrSupport), Some(1));
    check!(eq; gap(&report, "WI", GapClass::MissingEventContext), Some(1));
    check!(eq; gap(&report, "WI", GapClass::UnmappedEvent), Some(1));
    let unmapped = report
        .gaps
        .iter()
        .find(|gap| gap.class == GapClass::UnmappedEvent)
        .ok_or("missing unmapped-event gap")?;
    check!(eq; unmapped.unit, "performances");
    check!(eq; gap(&report, "WI", GapClass::MissingPerformanceHistory),
    None);
    let event_context = report
        .gaps
        .iter()
        .find(|gap| gap.class == GapClass::MissingEventContext)
        .ok_or("missing event-context gap")?;
    check!(eq; event_context.unit, "performances");

    check!(eq; row(&report, "IL")?.identity_conflicts, 1);
    check!(eq; gap(&report, "IL", GapClass::ConflictingIdentity), Some(1));
    check!(eq; gap(&report, "IL", GapClass::MissingGraduationEvidence),
    Some(1));

    check!(eq; gap(&report, "MN", GapClass::MissingCoach), Some(1));

    check!(eq; gap(&report, "UNKNOWN", GapClass::MissingSchool), Some(1));
    check!(eq; gap(&report, "UNKNOWN", GapClass::MissingCoach), Some(1));
    check!(eq; gap(&report, "UNKNOWN", GapClass::UnknownJurisdiction),
    Some(1));
    check!(eq; gap(&report, "UNKNOWN", GapClass::MissingProfile), Some(1));
    check!(eq; gap(&report, "UNKNOWN", GapClass::MissingEventContext),
    Some(1));

    check!(eq; gap(&report, "MN", GapClass::EmptyJurisdiction), None);
    check!(eq; gap(&report, "WY", GapClass::EmptyJurisdiction), Some(0));
    Ok(())
}

#[test]
fn all_contested_numeric_marks_report_missing_pr_support() -> TestResult {
    let (_dir, store) = contested_store()?;

    check!(eq; published_bests(&store)?.len(), 0);
    let contested = coverage_of(&store, Some(2027))?;
    let wi = row(&contested, "WI")?;
    check!(eq; wi.with_performance, 1, "contested athlete keeps an observed performance");
    check!(eq; wi.with_comparable_mark, 1, "both contested marks are numeric and comparable");
    check!(eq; wi.with_pr_support, 0, "no best publishes from an all-contested context");
    check!(eq; gap(&contested, "WI", GapClass::MissingPrSupport), Some(1));

    let dataset = crate::export::ExportDataset::load(&store)?;
    let athlete = dataset
        .athletes
        .first()
        .ok_or("missing contested athlete")?;
    let event = dataset.events.first().ok_or("missing contested event")?;
    let meet = dataset.meets.first().ok_or("missing contested meet")?;
    store.append(
        Table::Performances,
        &dated_performance(
            athlete,
            &event.id,
            &meet.id,
            "2026-05-08",
            Mark::TimeSeconds(ExactSeconds::parse("49.50")?),
            ("wiaa_results", "contest-clean"),
        )?,
    )?;

    let restored = coverage_of(&store, Some(2027))?;
    let wi = row(&restored, "WI")?;
    check!(eq; wi.with_performance, 1, "the athlete keeps one observed performance row");
    check!(eq; wi.with_comparable_mark, 1, "the clean mark stays comparable");
    check!(eq; wi.with_pr_support, 1, "the clean context publishes a best");
    check!(eq; gap(&restored, "WI", GapClass::MissingPrSupport), None);

    let selections = published_bests(&store)?;
    check!(eq; selections.len(), 1, "one best publishes once a clean context arrives");
    let winner = selections.first().ok_or("missing restored best")?;
    check!(eq; winner.conflicts.len(), 1, "the contested history is retained");
    let conflict = winner
        .conflicts
        .first()
        .ok_or("missing retained conflict")?;
    check!(eq; conflict.meet, "Contested Invite");
    check!(eq; conflict.marks, vec!["50.00".to_string(), "51.00".to_string()]);
    Ok(())
}

#[test]
fn unresolvable_marks_report_missing_pr_support() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Unresolvable High",
        "unresolvable high",
        None,
    );
    store.append(Table::Schools, &school)?;
    let mut athlete = CanonicalAthlete::new(
        &school_id,
        "Unresolvable Runner",
        GradYear::CO2027,
        Gender::Boys,
        fixture_source("unresolvable"),
    );
    athlete.evidence.push(evidence("wiaa_results"));
    athlete
        .observed_grades
        .push(observed_grade(11, 2025, "wiaa_results")?);
    store.append(Table::Athletes, &athlete)?;
    let mut meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "Unresolvable Invite",
        "2026-05-01",
        CompetitionLevel::Invitational,
    );
    meet.sports.push(Sport::OutdoorTrack);
    store.append(Table::Meets, &meet)?;
    let event = CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::Track400m,
            gender: Gender::Boys,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?;
    store.append(Table::Events, &event)?;
    store.append(
        Table::Performances,
        &dated_performance(
            &athlete,
            &event.id,
            &meet.id,
            "2026-05-01",
            Mark::Points(CentiPoints::new(500000)),
            ("wiaa_results", "unresolvable-points"),
        )?,
    )?;
    let vanished = CanonicalMeet::mint(
        Some(UsJurisdiction::Wisconsin),
        "2026-05-01",
        "Vanished Invitational",
        None,
    );
    let ghost = CanonicalEvent::new(
        EventIdentity {
            meet: &vanished,
            kind: EventKind::Track100m,
            gender: Gender::Boys,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?;
    store.append(Table::Events, &ghost)?;
    store.append(
        Table::Performances,
        &dated_performance(
            &athlete,
            &ghost.id,
            &vanished,
            "2026-05-01",
            Mark::TimeSeconds(ExactSeconds::parse("11.00")?),
            ("wiaa_results", "unresolvable-meet"),
        )?,
    )?;
    store.append(
        Table::Performances,
        &dated_performance(
            &athlete,
            &event.id,
            &meet.id,
            "2026-05-01",
            Mark::Raw("25-00.00".to_string()),
            ("wiaa_results", "unresolvable-imperial"),
        )?,
    )?;

    check!(eq; published_bests(&store)?.len(), 0);
    let report = coverage_of(&store, Some(2027))?;
    let wi = row(&report, "WI")?;
    check!(eq; wi.with_performance, 1);
    check!(eq; wi.with_comparable_mark, 1);
    check!(eq; wi.with_pr_support, 0);
    check!(eq; gap(&report, "WI", GapClass::MissingPrSupport), Some(1));
    Ok(())
}
