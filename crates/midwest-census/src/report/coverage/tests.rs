//! Coverage-report tests: the reconciliation, the empty-store rule and the gap classes, over a
//! fixture store built from canonical observations only (no network, no consolidated snapshot).

use super::*;
use crate::store::{Store, Table};
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, CoachRole, CompetitionLevel, EventKind, Evidence, Gender,
    GradYear, Grade, Mark, ObservedGrade, SchoolYear, SourceIdentity, SourceNamespace, SourceRef,
    Sport,
};
use census_domain::UsJurisdiction;
use tempfile::TempDir;

/// The fixture spans four placed jurisdictions with distinct shapes: one with a core athlete and a
/// measured coach, one whose athlete has no coach, one whose only athlete disagrees with its stored
/// cohort, and one whose school universe has no cohort athlete at all. It also holds an athlete whose
/// school was never stored and a performance whose athlete was never stored, so every gap class the
/// report can raise is reachable from it.
fn fixture_store() -> (TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let (school_a, school_a_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford");
    store.append(Table::Schools, &school_a).unwrap();
    let (school_b, school_b_id) =
        CanonicalSchool::new(UsJurisdiction::Minnesota, "Wayzata", "wayzata");
    store.append(Table::Schools, &school_b).unwrap();
    let (school_c, school_c_id) = CanonicalSchool::new(
        UsJurisdiction::Illinois,
        "Hinsdale Central",
        "hinsdale central",
    );
    store.append(Table::Schools, &school_c).unwrap();
    // Minted but never stored: this school id is what a missing school looks like in the store.
    let (_school_d, school_d_id) =
        CanonicalSchool::new(UsJurisdiction::Ohio, "Never Stored", "never stored");

    // WI: one athlete the platform's own evidence reaches, with a comparable mark and a profile.
    let mut wi_core = CanonicalAthlete::new(
        &school_a_id,
        "Julian Aguilera",
        GradYear::CO2027,
        Gender::Boys,
    );
    wi_core.evidence.push(evidence("milesplit_roster"));
    wi_core.source_identities.push(SourceIdentity::new(
        SourceNamespace::MilesplitAthlete,
        "14399169",
    ));
    wi_core
        .public_profile_urls
        .push("https://wi.milesplit.com/athletes/14399169-julian-aguilera".to_string());
    wi_core.sports.push(Sport::OutdoorTrack);
    wi_core
        .observed_grades
        .push(observed_grade(11, 2025, "milesplit_roster"));
    store.append(Table::Athletes, &wi_core).unwrap();

    // WI: one athlete known only through the Athletic.net mirror, with an unparsed mark whose event
    // row was never stored.
    let mut wi_mirror = CanonicalAthlete::new(
        &school_a_id,
        "Rae Lindgren",
        GradYear::CO2027,
        Gender::Girls,
    );
    wi_mirror.evidence.push(evidence("athleticlive_athletes"));
    store.append(Table::Athletes, &wi_mirror).unwrap();

    // MN: one in-cohort athlete whose school carries no coach, and one athlete outside the cohort.
    let mut mn_athlete = CanonicalAthlete::new(
        &school_b_id,
        "Nora Halvorsen",
        GradYear::CO2027,
        Gender::Girls,
    );
    mn_athlete.evidence.push(evidence("delphi_timing"));
    store.append(Table::Athletes, &mn_athlete).unwrap();
    let mut mn_off_cohort = CanonicalAthlete::new(
        &school_b_id,
        "Older Halvorsen",
        GradYear::new(2028).unwrap(),
        Gender::Boys,
    );
    mn_off_cohort.evidence.push(evidence("delphi_timing"));
    store.append(Table::Athletes, &mn_off_cohort).unwrap();

    // KS: a school universe with no athlete in the cohort at all — "covered, empty", which the row
    // has to publish as schools counted and athletes zero.
    let (school_k, school_k_id) =
        CanonicalSchool::new(UsJurisdiction::Kansas, "Olathe West", "olathe west");
    store.append(Table::Schools, &school_k).unwrap();
    let mut ks_off_cohort = CanonicalAthlete::new(
        &school_k_id,
        "Younger Kansan",
        GradYear::new(2028).unwrap(),
        Gender::Girls,
    );
    ks_off_cohort.evidence.push(evidence("kshsaa_results"));
    store.append(Table::Athletes, &ks_off_cohort).unwrap();

    // IL: one athlete whose observed grade implies a different graduation year than the stored one.
    let mut il_athlete =
        CanonicalAthlete::new(&school_c_id, "Theo Vance", GradYear::CO2027, Gender::Boys);
    il_athlete.evidence.push(evidence("ihsa_results"));
    il_athlete
        .observed_grades
        .push(observed_grade(11, 2024, "ihsa_results"));
    store.append(Table::Athletes, &il_athlete).unwrap();

    // The unplaceable athlete: nothing in the school table carries its school id.
    let mut unplaced = CanonicalAthlete::new(
        &school_d_id,
        "Unplaced Runner",
        GradYear::CO2027,
        Gender::Boys,
    );
    unplaced.evidence.push(evidence("ohsaa_results"));
    store.append(Table::Athletes, &unplaced).unwrap();

    let mut tf_coach = CanonicalCoach::new(
        &school_a_id,
        "Dana Coach",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    tf_coach.professional_email = Some("coach@example.org".to_string());
    store.append(Table::Coaches, &tf_coach).unwrap();
    let xc_coach = CanonicalCoach::new(
        &school_a_id,
        "Wren Coach",
        Some(Sport::CrossCountry),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    store.append(Table::Coaches, &xc_coach).unwrap();

    let meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "Abbotsford Invite",
        "2026-05-01",
        CompetitionLevel::Invitational,
    );
    store.append(Table::Meets, &meet).unwrap();
    let event = CanonicalEvent::new(&meet.id, EventKind::Track100m, Gender::Boys, None, None);
    store.append(Table::Events, &event).unwrap();
    let comparable = performance(
        &wi_core.id,
        &event.id,
        &meet.id,
        &school_a_id,
        EventKind::Track100m,
        Mark::TimeSeconds(10.94),
        "wiaa_results",
        "wi-1",
    );
    store.append(Table::Performances, &comparable).unwrap();

    // A performance whose event row is deliberately absent, carrying a mark nothing can compare.
    let missing_event =
        CanonicalEvent::new(&meet.id, EventKind::Track200m, Gender::Girls, None, None);
    let unparsed = performance(
        &wi_mirror.id,
        &missing_event.id,
        &meet.id,
        &school_a_id,
        EventKind::Track200m,
        Mark::Raw("12.4h".to_string()),
        "athleticlive_athletes",
        "al-1",
    );
    store.append(Table::Performances, &unparsed).unwrap();

    // A performance whose athlete was never stored: a partial run leaves these behind, and the
    // report has to keep them visible without inventing an athlete row for them.
    let never_stored =
        CanonicalAthlete::new(&school_d_id, "Never Stored", GradYear::CO2027, Gender::Boys);
    let orphan = performance(
        &never_stored.id,
        &missing_event.id,
        &meet.id,
        &school_d_id,
        EventKind::Track200m,
        Mark::TimeSeconds(24.10),
        "ohsaa_results",
        "oh-orphan",
    );
    store.append(Table::Performances, &orphan).unwrap();

    (dir, store)
}

fn evidence(source: &str) -> Evidence {
    Evidence::parsed(SourceRef::new(source, None), "2026-09-20")
}

fn observed_grade(grade: u8, school_year: i16, source: &str) -> ObservedGrade {
    ObservedGrade {
        grade: Grade::new(grade).unwrap(),
        school_year: SchoolYear(school_year),
        source: SourceRef::new(source, None),
    }
}

#[allow(clippy::too_many_arguments)]
fn performance(
    athlete: &census_domain::model::AthleteId,
    event: &census_domain::model::EventId,
    meet: &census_domain::model::MeetId,
    school: &census_domain::model::SchoolId,
    kind: EventKind,
    mark: Mark,
    source: &str,
    source_key: &str,
) -> CanonicalPerformance {
    CanonicalPerformance {
        id: CanonicalPerformance::mint(athlete, meet, &kind, "2026-05-01", source_key),
        athlete: athlete.clone(),
        team: CanonicalTeam::mint(school, Sport::OutdoorTrack, Gender::Boys, SchoolYear(2026)),
        event: event.clone(),
        meet: meet.clone(),
        date: "2026-05-01".to_string(),
        mark,
        wind_mps: None,
        place: None,
        heat: None,
        round: None,
        timing: None,
        observed_grade: None,
        evidence: vec![evidence(source)],
        source_key: source_key.to_string(),
    }
}

fn row<'a>(report: &'a CoverageReport, code: &str) -> &'a JurisdictionCoverage {
    report
        .jurisdictions
        .iter()
        .find(|row| row.jurisdiction.code() == code)
        .unwrap()
}

/// The count of one gap class in one jurisdiction, `None` when the class was not raised there.
fn gap(report: &CoverageReport, code: &str, class: GapClass) -> Option<usize> {
    report
        .gaps
        .iter()
        .find(|gap| gap.jurisdiction.code() == code && gap.class == class)
        .map(|gap| gap.count)
}

#[test]
fn an_empty_store_publishes_every_jurisdiction_with_a_gap() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let report = coverage_report(&store, None).unwrap();

    // Every configured jurisdiction plus UNKNOWN: an omitted row is the one outcome the report may
    // not produce, so an empty store still publishes the full set.
    assert_eq!(
        report.jurisdictions.len(),
        UsJurisdiction::ALL.len().saturating_add(1)
    );
    assert_eq!(
        report.jurisdictions.last().unwrap().jurisdiction.code(),
        "UNKNOWN"
    );
    assert!(report
        .jurisdictions
        .iter()
        .all(|row| row.athletes == 0 && row.schools == 0 && row.core_share_pct == 0));
    assert!(report
        .jurisdictions
        .iter()
        .any(|row| row.jurisdiction.code() == "WI" && row.schools == 0));

    // ... and an explicit gap class for every row, the unplaceable one included, so emptiness is
    // findable rather than inferred from an absent key.
    assert_eq!(
        report.gaps.len(),
        UsJurisdiction::ALL.len().saturating_add(1)
    );
    assert!(report.gaps.iter().all(|gap| {
        gap.class == GapClass::EmptyJurisdiction && gap.count == 0 && gap.unit == "schools"
    }));

    assert_eq!(report.read, CoverageTotals::default());
    assert_eq!(report.off_cohort_athletes, 0);
    report.reconcile().unwrap();
}

#[test]
fn jurisdiction_rows_sum_to_the_store_totals() {
    let (_dir, store) = fixture_store();

    let report = coverage_report(&store, Some(2027)).unwrap();
    let published = report.published_totals();

    // Every stored row is either published in a jurisdiction or reported as off cohort.
    let stored_athletes = store
        .scan::<CanonicalAthlete>(Table::Athletes)
        .unwrap()
        .len();
    assert_eq!(stored_athletes, 7);
    assert_eq!(report.off_cohort_athletes, 2);
    assert_eq!(
        published
            .athletes
            .saturating_add(report.off_cohort_athletes),
        stored_athletes
    );
    assert_eq!(
        published.schools,
        store.scan::<CanonicalSchool>(Table::Schools).unwrap().len()
    );
    assert_eq!(
        published.coaches,
        store.scan::<CanonicalCoach>(Table::Coaches).unwrap().len()
    );
    assert_eq!(
        published.meets,
        store.scan::<CanonicalMeet>(Table::Meets).unwrap().len()
    );
    assert_eq!(
        published.performances,
        store
            .scan::<CanonicalPerformance>(Table::Performances)
            .unwrap()
            .len()
    );

    assert_eq!(published, report.read);
    report.reconcile().unwrap();
    let lines = report.reconciliation_lines();
    assert_eq!(lines.len(), 4);
    assert!(lines
        .iter()
        .any(|line| line.contains("coverage reconciliation: matches")));
    assert!(
        lines.iter().any(
            |line| line.contains("read: schools=4 athletes=5 coaches=2 meets=1 performances=3")
        ),
        "{lines:?}"
    );
}

#[test]
fn a_jurisdiction_row_carries_the_counted_columns() {
    let (_dir, store) = fixture_store();

    let report = coverage_report(&store, Some(2027)).unwrap();

    let wi = row(&report, "WI");
    assert_eq!(wi.schools, 1);
    assert_eq!(wi.schools_with_athletes, 1);
    assert_eq!(wi.athletes, 2);
    assert_eq!(wi.athletes_core, 1);
    assert_eq!(wi.core_share_pct, 50);
    assert_eq!(wi.boys, 1);
    assert_eq!(wi.girls, 1);
    assert_eq!(wi.grad_verified, 1);
    assert_eq!(wi.grad_unresolved, 1);
    assert_eq!(wi.outdoor_track, 1);
    assert_eq!(wi.indoor_track, 0);
    assert_eq!(wi.cross_country, 0);
    assert_eq!(wi.with_performance, 2);
    assert_eq!(wi.with_comparable_mark, 1);
    assert_eq!(wi.multisource, 0);
    assert_eq!(wi.with_profile_url, 1);
    assert_eq!(wi.with_milesplit_url, 1);
    assert_eq!(wi.with_athletic_net_url, 0);
    assert_eq!(wi.coaches, 2);
    assert_eq!(wi.coaches_with_email, 1);
    assert_eq!(wi.schools_with_tf_coach, 1);
    assert_eq!(wi.schools_with_xc_coach, 1);
    assert_eq!(wi.schools_with_coach_email, 1);
    assert_eq!(wi.meets, 1);
    assert_eq!(wi.performances, 2);
    assert_eq!(wi.sources.get("milesplit_roster"), Some(&1));
    assert_eq!(wi.sources.get("athleticlive_athletes"), Some(&1));

    // MN: a school, an in-cohort athlete, and no coach row at all.
    let mn = row(&report, "MN");
    assert_eq!(mn.schools, 1);
    assert_eq!(mn.athletes, 1);
    assert_eq!(mn.coaches, 0);

    // KS: the school universe exists and the cohort is empty — the row publishes the school count and
    // zero athletes instead of dropping the state.
    let ks = row(&report, "KS");
    assert_eq!(ks.schools, 1);
    assert_eq!(ks.athletes, 0);
    assert_eq!(ks.core_share_pct, 0);

    let unplaced = row(&report, "UNKNOWN");
    assert_eq!(unplaced.schools, 0);
    assert_eq!(unplaced.athletes, 1);
    // The orphan performance is a stored row with no stored athlete: it publishes as a performance
    // count and never as an athlete, because no athlete row exists for those columns to describe.
    assert_eq!(unplaced.performances, 1);
    assert_eq!(unplaced.with_performance, 0);
    assert_eq!(unplaced.with_comparable_mark, 0);
}

#[test]
fn gap_classes_carry_the_count_that_produced_them() {
    let (_dir, store) = fixture_store();

    let report = coverage_report(&store, Some(2027)).unwrap();

    // WI: the mirror-only athlete has no grade observation, no profile and no comparable mark, and
    // its performance names an event the events table does not hold.
    assert_eq!(
        gap(&report, "WI", GapClass::MissingGraduationEvidence),
        Some(1)
    );
    assert_eq!(gap(&report, "WI", GapClass::MissingProfile), Some(1));
    assert_eq!(gap(&report, "WI", GapClass::MissingPrSupport), Some(1));
    assert_eq!(gap(&report, "WI", GapClass::MissingEventContext), Some(1));
    assert_eq!(
        gap(&report, "WI", GapClass::MissingPerformanceHistory),
        None
    );
    let event_context = report
        .gaps
        .iter()
        .find(|gap| gap.class == GapClass::MissingEventContext)
        .unwrap();
    assert_eq!(event_context.unit, "performances");

    // IL: the observed grade implies 2026, the stored cohort says 2027.
    assert_eq!(row(&report, "IL").identity_conflicts, 1);
    assert_eq!(gap(&report, "IL", GapClass::ConflictingIdentity), Some(1));
    assert_eq!(
        gap(&report, "IL", GapClass::MissingGraduationEvidence),
        None
    );

    // MN: its athlete's school carries no coach row at all.
    assert_eq!(gap(&report, "MN", GapClass::MissingCoach), Some(1));

    // UNKNOWN: the school id has no row, so the athlete is missing a school and a coach, and its
    // row is where an unplaceable athlete stays visible.
    assert_eq!(gap(&report, "UNKNOWN", GapClass::MissingSchool), Some(1));
    assert_eq!(gap(&report, "UNKNOWN", GapClass::MissingCoach), Some(1));
    assert_eq!(
        gap(&report, "UNKNOWN", GapClass::UnknownJurisdiction),
        Some(1)
    );
    assert_eq!(gap(&report, "UNKNOWN", GapClass::MissingProfile), Some(1));
    // The orphan performance's event row is absent too, so the row that keeps it visible is the
    // one that names the missing event context.
    assert_eq!(
        gap(&report, "UNKNOWN", GapClass::MissingEventContext),
        Some(1)
    );

    // A jurisdiction with a school universe is not "empty"; one with nothing at all is.
    assert_eq!(gap(&report, "MN", GapClass::EmptyJurisdiction), None);
    assert_eq!(gap(&report, "WY", GapClass::EmptyJurisdiction), Some(0));
}

#[test]
fn a_mirror_result_plane_row_is_counted_but_never_core() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let (school, school_id) =
        CanonicalSchool::new(UsJurisdiction::Ohio, "Dublin Coffman", "dublin coffman");
    store.append(Table::Schools, &school).unwrap();
    let mut core_athlete =
        CanonicalAthlete::new(&school_id, "Core Runner", GradYear::CO2027, Gender::Boys);
    core_athlete.evidence.push(evidence("ohsaa_results"));
    store.append(Table::Athletes, &core_athlete).unwrap();
    let mut mirror_athlete =
        CanonicalAthlete::new(&school_id, "Mirror Runner", GradYear::CO2027, Gender::Girls);
    mirror_athlete
        .evidence
        .push(evidence("athleticlive_results"));
    store.append(Table::Athletes, &mirror_athlete).unwrap();

    // The Athletic.net result plane is a mirror: its row counts in the all-sources columns and in its
    // own provider column, and drops out of the core columns because `NON_CORE_SOURCE_IDS` names it.
    let report = coverage_report(&store, Some(2027)).unwrap();
    let ohio = row(&report, "OH");
    assert_eq!(ohio.athletes, 2);
    assert_eq!(ohio.athletes_core, 1);
    assert_eq!(ohio.core_share_pct, 50);
    assert_eq!(ohio.sources.get("athleticlive_results"), Some(&1));
    assert_eq!(ohio.sources.get("ohsaa_results"), Some(&1));

    let core = crate::report::build_census(&store, crate::report::Scope::Core).unwrap();
    assert_eq!(core.totals.athletes, 1);
}

#[test]
fn reconcile_refuses_a_report_whose_rows_lost_a_row() {
    let (_dir, store) = fixture_store();
    let mut report = coverage_report(&store, Some(2027)).unwrap();

    report.read.athletes = report.read.athletes.saturating_add(1);
    let error = report.reconcile().unwrap_err();
    let text = error.to_string();
    assert!(text.contains("coverage reconciliation failed"), "{text}");
    assert!(text.contains("read schools=4 athletes=6"), "{text}");
    assert!(text.contains("rows publish schools=4 athletes=5"), "{text}");
}
