//! Coverage-report tests: the reconciliation, the empty-store rule and the gap classes, over a
//! fixture store built from canonical observations only (no network, no consolidated snapshot).

use super::*;
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, CoachRole, CompetitionLevel, EventKind, Evidence, Gender,
    GradYear, Grade, Mark, CentiSeconds, ObservedGrade, SchoolYear, SourceIdentity, SourceNamespace, SourceRef,
    Sport,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
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
        Mark::TimeSeconds(CentiSeconds(1094)),
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

    // An event the ontology has no home for yet, with one row published under it: the vocabulary gap
    // is the event's, and the class is raised for the rows that reference that event.
    let unmapped_event = CanonicalEvent::new(
        &meet.id,
        EventKind::Unmapped {
            label: "Coed 200m".to_string(),
        },
        Gender::Girls,
        None,
        None,
    );
    store.append(Table::Events, &unmapped_event).unwrap();
    let unmapped_row = performance(
        &wi_mirror.id,
        &unmapped_event.id,
        &meet.id,
        &school_a_id,
        EventKind::Unmapped {
            label: "Coed 200m".to_string(),
        },
        Mark::Raw("27.1h".to_string()),
        "athleticlive_athletes",
        "al-2",
    );
    store.append(Table::Performances, &unmapped_row).unwrap();

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
        Mark::TimeSeconds(CentiSeconds(2410)),
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
        school_year: SchoolYear::new(school_year).expect("the fixture's season is in range"),
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
        team: CanonicalTeam::mint(
            school,
            Sport::OutdoorTrack,
            Gender::Boys,
            SchoolYear::new(2026).expect("2026 is a season"),
        ),
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
        source_athlete: None,
        // The fixture mints the id from its own fields and merges nothing, so no id ever collided.
        retained_conflicts: Vec::new(),
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
        UsJurisdiction::CENSUS_SCOPE.len().saturating_add(1)
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
    // findable rather than inferred from an absent key. A jurisdiction outside the run scope has no
    // row to carry a gap: its absence is ADR-009, not a finding.
    assert_eq!(
        report.gaps.len(),
        UsJurisdiction::CENSUS_SCOPE.len().saturating_add(1)
    );
    assert!(report.gaps.iter().all(|gap| gap
        .jurisdiction
        .jurisdiction()
        .is_none_or(|jurisdiction| jurisdiction.is_in_census_scope())));
    assert!(report.gaps.iter().all(|gap| {
        gap.class == GapClass::EmptyJurisdiction && gap.count == 0 && gap.unit == "schools"
    }));

    assert_eq!(report.read, CoverageTotals::default());
    assert_eq!(report.off_cohort_athletes, 0);
    assert!(report
        .notes
        .iter()
        .all(|note| !note.contains("outside the census run scope")));
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
            |line| line.contains("read: schools=4 athletes=5 coaches=2 meets=1 performances=4")
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
    // One row per stored performance, not one per athlete: the core athlete's comparable mark, the
    // mirror athlete's unparsed mark, and the row published under the event the ontology has no home
    // for yet.
    assert_eq!(wi.performances, 3);
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
    // The same row's kind is a source label the ontology has no home for yet, which is its own
    // class: a row that can only publish under a vendor string is a vocabulary gap, and it must not
    // be folded into "the events table is missing an id".
    assert_eq!(gap(&report, "WI", GapClass::UnmappedEvent), Some(1));
    let unmapped = report
        .gaps
        .iter()
        .find(|gap| gap.class == GapClass::UnmappedEvent)
        .unwrap();
    assert_eq!(unmapped.unit, "performances");
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

/// A national all-sources wave leaves rows behind for a jurisdiction a census run never covers
/// (ADR-009): the live store's Alaska and Hawaii rows are what unbalanced the reconciliation. They
/// publish no coverage row, so the read side has to stop at the same scope the rows stop at — and
/// the exclusion is recorded as a note rather than disappearing.
#[test]
fn an_out_of_scope_jurisdiction_enters_no_denominator_and_still_reconciles() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    // OH: one athlete the platform's own evidence reaches and one the mirror carries, so both sides
    // of the core split stay measured while the reconciliation balances.
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

    // AK: the same wave left a school, a cohort athlete, a coach, a meet and a result behind.
    let (ak_school, ak_school_id) =
        CanonicalSchool::new(UsJurisdiction::Alaska, "Service High", "service high");
    store.append(Table::Schools, &ak_school).unwrap();
    let mut ak_athlete = CanonicalAthlete::new(
        &ak_school_id,
        "Denali Runner",
        GradYear::CO2027,
        Gender::Boys,
    );
    ak_athlete.evidence.push(evidence("athleticlive_athletes"));
    store.append(Table::Athletes, &ak_athlete).unwrap();
    let ak_coach = CanonicalCoach::new(
        &ak_school_id,
        "Aurora Coach",
        Some(Sport::CrossCountry),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    store.append(Table::Coaches, &ak_coach).unwrap();
    let ak_meet = CanonicalMeet::new(
        Some(UsJurisdiction::Alaska),
        "Service Invite",
        "2026-05-01",
        CompetitionLevel::Invitational,
    );
    store.append(Table::Meets, &ak_meet).unwrap();
    let ak_event = CanonicalEvent::new(&ak_meet.id, EventKind::Track100m, Gender::Boys, None, None);
    store.append(Table::Events, &ak_event).unwrap();
    let ak_result = performance(
        &ak_athlete.id,
        &ak_event.id,
        &ak_meet.id,
        &ak_school_id,
        EventKind::Track100m,
        Mark::TimeSeconds(CentiSeconds(1142)),
        "athleticlive_results",
        "ak-1",
    );
    store.append(Table::Performances, &ak_result).unwrap();

    let report = coverage_report(&store, Some(2027)).unwrap();

    // The published universe is the census run scope: no row for Alaska, while the state that is in
    // scope keeps measuring both source scopes.
    assert!(report
        .jurisdictions
        .iter()
        .all(|published| published.jurisdiction.code() != "AK"));
    let ohio = row(&report, "OH");
    assert_eq!(ohio.athletes, 2);
    assert_eq!(ohio.athletes_core, 1);

    // Every read counter stops where the rows stop, and the cohort filter is not what stopped it.
    assert_eq!(report.off_cohort_athletes, 0);
    assert_eq!(report.read, report.published_totals());
    assert_eq!(report.read.schools, 1);
    assert_eq!(report.read.athletes, 2);
    assert_eq!(report.read.coaches, 0);
    assert_eq!(report.read.meets, 0);
    assert_eq!(report.read.performances, 0);
    report.reconcile().unwrap();

    // What the run scope left out is published as a note carrying the counters that were left out.
    let scope_note = report
        .notes
        .iter()
        .find(|note| note.contains("outside the census run scope"));
    let scope_note = scope_note.unwrap_or_else(|| panic!("no scope note in {:?}", report.notes));
    assert!(
        scope_note.contains("schools=1 athletes=1 coaches=1 meets=1 performances=1"),
        "{scope_note}"
    );
}

/// The published side is what the reconciliation guards: a row dropped from a complete report leaves
/// the read side holding a row no row publishes, and a row invented out of nothing publishes a row
/// nothing read. Both fail, naming the counters that disagree.
#[test]
fn reconcile_refuses_a_report_whose_rows_lost_or_invented_a_row() {
    let (_dir, store) = fixture_store();
    let report = coverage_report(&store, Some(2027)).unwrap();

    // Lost: the unplaced row carries a cohort athlete and its orphan performance.
    let mut lost = report.clone();
    lost.jurisdictions.pop();
    let error = lost.reconcile().unwrap_err().to_string();
    assert!(error.contains("coverage reconciliation failed"), "{error}");
    assert!(
        error.contains("rows publish schools=4 athletes=4"),
        "{error}"
    );

    // Invented: a second copy of the unplaced row publishes an athlete nothing read.
    let mut invented = report.clone();
    invented.jurisdictions.push(row(&report, "UNKNOWN").clone());
    let error = invented.reconcile().unwrap_err().to_string();
    assert!(error.contains("coverage reconciliation failed"), "{error}");
    assert!(
        error.contains("rows publish schools=4 athletes=6"),
        "{error}"
    );
}

/// The read side is counted from the store, not summed from the rows: a read count that drifts ahead
/// of what the rows publish (the store's own shape moving under a fresh scan, the failure the live
/// store hit) fails the reconciliation instead of quietly agreeing.
#[test]
fn reconcile_refuses_a_read_count_the_rows_do_not_publish() {
    let (_dir, store) = fixture_store();
    let mut report = coverage_report(&store, Some(2027)).unwrap();

    report.read.athletes = report.read.athletes.saturating_add(1);
    let error = report.reconcile().unwrap_err();
    let text = error.to_string();
    assert!(text.contains("coverage reconciliation failed"), "{text}");
    assert!(text.contains("read schools=4 athletes=6"), "{text}");
    assert!(text.contains("rows publish schools=4 athletes=5"), "{text}");
}
