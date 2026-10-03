use super::*;
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, CentiSeconds, CoachRole, CompetitionLevel, EventKind, Evidence,
    Gender, GradYear, Grade, Mark, ObservedGrade, SchoolYear, SourceIdentity, SourceNamespace,
    SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod counts;
mod gaps;
mod published_cohort;
mod reconcile;
mod scope;

fn coverage_of(store: &Store, grad_year: Option<i16>) -> TestResult<CoverageReport> {
    let dataset = crate::export::ExportDataset::load(store)?;
    Ok(coverage_report(&dataset, grad_year)?)
}

fn census_of(store: &Store, scope: crate::report::Scope) -> TestResult<crate::report::Census> {
    let dataset = crate::export::ExportDataset::load(store)?;
    Ok(crate::report::build_census(
        &crate::report::Derivation::of(&dataset, scope, None),
        &store.out_dir(),
    ))
}

fn fixture_store() -> TestResult<(TempDir, Store)> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;

    let (school_a, school_a_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford");
    store.append(Table::Schools, &school_a)?;
    let (school_b, school_b_id) =
        CanonicalSchool::new(UsJurisdiction::Minnesota, "Wayzata", "wayzata");
    store.append(Table::Schools, &school_b)?;
    let (school_c, school_c_id) = CanonicalSchool::new(
        UsJurisdiction::Illinois,
        "Hinsdale Central",
        "hinsdale central",
    );
    store.append(Table::Schools, &school_c)?;
    let (_school_d, school_d_id) =
        CanonicalSchool::new(UsJurisdiction::Ohio, "Never Stored", "never stored");

    let mut wi_core = CanonicalAthlete::new(
        &school_a_id,
        "Julian Aguilera",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169"),
    );
    wi_core.evidence.push(evidence("milesplit_roster"));
    wi_core
        .public_profile_urls
        .push("https://wi.milesplit.com/athletes/14399169-julian-aguilera".to_string());
    wi_core.sports.push(Sport::OutdoorTrack);
    wi_core
        .observed_grades
        .push(observed_grade(11, 2025, "milesplit_roster")?);
    store.append(Table::Athletes, &wi_core)?;

    let mut wi_mirror = CanonicalAthlete::new(
        &school_a_id,
        "Rae Lindgren",
        GradYear::CO2027,
        Gender::Girls,
        fixture_source("wi-mirror"),
    );
    wi_mirror.evidence.push(evidence("athleticlive_athletes"));
    store.append(Table::Athletes, &wi_mirror)?;

    let mut mn_athlete = CanonicalAthlete::new(
        &school_b_id,
        "Nora Halvorsen",
        GradYear::CO2027,
        Gender::Girls,
        fixture_source("mn-runner"),
    );
    mn_athlete.evidence.push(evidence("delphi_timing"));
    store.append(Table::Athletes, &mn_athlete)?;
    let mut mn_off_cohort = CanonicalAthlete::new(
        &school_b_id,
        "Older Halvorsen",
        GradYear::new(2028).ok_or("invalid graduation year")?,
        Gender::Boys,
        fixture_source("mn-younger"),
    );
    mn_off_cohort.evidence.push(evidence("delphi_timing"));
    store.append(Table::Athletes, &mn_off_cohort)?;

    let (school_k, school_k_id) =
        CanonicalSchool::new(UsJurisdiction::Kansas, "Olathe West", "olathe west");
    store.append(Table::Schools, &school_k)?;
    let mut ks_off_cohort = CanonicalAthlete::new(
        &school_k_id,
        "Younger Kansan",
        GradYear::new(2028).ok_or("invalid graduation year")?,
        Gender::Girls,
        fixture_source("ks-younger"),
    );
    ks_off_cohort.evidence.push(evidence("kshsaa_results"));
    store.append(Table::Athletes, &ks_off_cohort)?;

    let mut il_athlete = CanonicalAthlete::new(
        &school_c_id,
        "Theo Vance",
        GradYear::CO2027,
        Gender::Boys,
        fixture_source("il-runner"),
    );
    il_athlete.evidence.push(evidence("ihsa_results"));
    il_athlete
        .observed_grades
        .push(observed_grade(11, 2024, "ihsa_results")?);
    store.append(Table::Athletes, &il_athlete)?;

    let mut unplaced = CanonicalAthlete::new(
        &school_d_id,
        "Unplaced Runner",
        GradYear::CO2027,
        Gender::Boys,
        fixture_source("unplaced"),
    );
    unplaced.evidence.push(evidence("ohsaa_results"));
    store.append(Table::Athletes, &unplaced)?;

    let mut tf_coach = CanonicalCoach::new(
        &school_a_id,
        "Dana Coach",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    tf_coach.professional_email = Some("coach@example.org".to_string());
    store.append(Table::Coaches, &tf_coach)?;
    let xc_coach = CanonicalCoach::new(
        &school_a_id,
        "Wren Coach",
        Some(Sport::CrossCountry),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    store.append(Table::Coaches, &xc_coach)?;

    let meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "Abbotsford Invite",
        "2026-05-01",
        CompetitionLevel::Invitational,
    );
    store.append(Table::Meets, &meet)?;
    let event = CanonicalEvent::new(&meet.id, EventKind::Track100m, Gender::Boys, None, None);
    store.append(Table::Events, &event)?;
    let comparable = performance(
        &wi_core,
        &event.id,
        &meet.id,
        EventKind::Track100m,
        Mark::TimeSeconds(CentiSeconds::new(1094)),
        "wiaa_results",
        "wi-1",
    )?;
    store.append(Table::Performances, &comparable)?;

    let missing_event =
        CanonicalEvent::new(&meet.id, EventKind::Track200m, Gender::Girls, None, None);
    let unparsed = performance(
        &wi_mirror,
        &missing_event.id,
        &meet.id,
        EventKind::Track200m,
        Mark::Raw("12.4h".to_string()),
        "athleticlive_athletes",
        "al-1",
    )?;
    store.append(Table::Performances, &unparsed)?;

    let unmapped_event = CanonicalEvent::new(
        &meet.id,
        EventKind::Unmapped {
            label: "Coed 200m".to_string(),
        },
        Gender::Girls,
        None,
        None,
    );
    store.append(Table::Events, &unmapped_event)?;
    let unmapped_row = performance(
        &wi_mirror,
        &unmapped_event.id,
        &meet.id,
        EventKind::Unmapped {
            label: "Coed 200m".to_string(),
        },
        Mark::Raw("27.1h".to_string()),
        "athleticlive_athletes",
        "al-2",
    )?;
    store.append(Table::Performances, &unmapped_row)?;

    let never_stored = CanonicalAthlete::new(
        &school_d_id,
        "Never Stored",
        GradYear::CO2027,
        Gender::Boys,
        fixture_source("never-stored"),
    );
    let orphan = performance(
        &never_stored,
        &missing_event.id,
        &meet.id,
        EventKind::Track200m,
        Mark::TimeSeconds(CentiSeconds::new(2410)),
        "ohsaa_results",
        "oh-orphan",
    )?;
    store.append(Table::Performances, &orphan)?;

    Ok((dir, store))
}

fn fixture_source(id: &str) -> SourceIdentity {
    SourceIdentity::new(SourceNamespace::Other("fixture".to_owned()), id)
}

fn evidence(source: &str) -> Evidence {
    Evidence::parsed(SourceRef::new(source, None), "2026-09-20")
}

fn observed_grade(grade: u8, school_year: i16, source: &str) -> TestResult<ObservedGrade> {
    Ok(ObservedGrade {
        grade: Grade::new(grade).ok_or("invalid fixture grade")?,
        school_year: SchoolYear::new(school_year).ok_or("invalid fixture season")?,
        source: SourceRef::new(source, None),
    })
}

fn performance(
    athlete: &CanonicalAthlete,
    event: &census_domain::model::EventId,
    meet: &census_domain::model::MeetId,
    kind: EventKind,
    mark: Mark,
    source: &str,
    source_key: &str,
) -> TestResult<CanonicalPerformance> {
    Ok(CanonicalPerformance {
        id: CanonicalPerformance::mint(&athlete.id, meet, &kind, "2026-05-01", source_key),
        athlete: athlete.id.clone(),
        team: CanonicalTeam::mint(
            &athlete.school,
            Sport::OutdoorTrack,
            Gender::Boys,
            SchoolYear::new(2026).ok_or("invalid fixture season")?,
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
        source_athlete: athlete.source.clone(),
        retained_conflicts: Vec::new(),
    })
}

fn row<'a>(report: &'a CoverageReport, code: &str) -> TestResult<&'a JurisdictionCoverage> {
    report
        .jurisdictions
        .iter()
        .find(|row| row.jurisdiction.code() == code)
        .ok_or_else(|| format!("missing jurisdiction {code}").into())
}

fn gap(report: &CoverageReport, code: &str, class: GapClass) -> Option<usize> {
    report
        .gaps
        .iter()
        .find(|gap| gap.jurisdiction.code() == code && gap.class == class)
        .map(|gap| gap.count)
}
