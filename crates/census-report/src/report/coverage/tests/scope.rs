use super::*;

#[test]
fn an_out_of_scope_jurisdiction_enters_no_denominator_and_still_reconciles() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;

    let (school, school_id) =
        CanonicalSchool::new(UsJurisdiction::Ohio, "Dublin Coffman", "dublin coffman");
    store.append(Table::Schools, &school)?;
    let mut core_athlete = CanonicalAthlete::new(
        &school_id,
        "Core Runner",
        GradYear::CO2027,
        Gender::Boys,
        fixture_source("oh-core"),
    );
    core_athlete.evidence.push(evidence("ohsaa_results"));
    store.append(Table::Athletes, &core_athlete)?;
    let mut mirror_athlete = CanonicalAthlete::new(
        &school_id,
        "Mirror Runner",
        GradYear::CO2027,
        Gender::Girls,
        fixture_source("oh-mirror"),
    );
    mirror_athlete
        .evidence
        .push(evidence("athleticlive_results"));
    store.append(Table::Athletes, &mirror_athlete)?;

    let (ak_school, ak_school_id) =
        CanonicalSchool::new(UsJurisdiction::Alaska, "Service High", "service high");
    store.append(Table::Schools, &ak_school)?;
    let mut ak_athlete = CanonicalAthlete::new(
        &ak_school_id,
        "Denali Runner",
        GradYear::CO2027,
        Gender::Boys,
        fixture_source("ak-runner"),
    );
    ak_athlete.evidence.push(evidence("athleticlive_athletes"));
    store.append(Table::Athletes, &ak_athlete)?;
    let ak_coach = CanonicalCoach::new(
        &ak_school_id,
        "Aurora Coach",
        Some(Sport::CrossCountry),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    store.append(Table::Coaches, &ak_coach)?;
    let ak_meet = CanonicalMeet::new(
        Some(UsJurisdiction::Alaska),
        "Service Invite",
        "2026-05-01",
        CompetitionLevel::Invitational,
    );
    store.append(Table::Meets, &ak_meet)?;
    let ak_event = CanonicalEvent::new(&ak_meet.id, EventKind::Track100m, Gender::Boys, None, None);
    store.append(Table::Events, &ak_event)?;
    let ak_result = performance(
        &ak_athlete,
        &ak_event.id,
        &ak_meet.id,
        EventKind::Track100m,
        Mark::TimeSeconds(CentiSeconds::new(1142)),
        "athleticlive_results",
        "ak-1",
    )?;
    store.append(Table::Performances, &ak_result)?;

    let report = coverage_of(&store, Some(2027))?;

    check!(report
        .jurisdictions
        .iter()
        .all(|published| published.jurisdiction.code() != "AK"));
    let ohio = row(&report, "OH")?;
    check!(eq; ohio.athletes, 2);
    check!(eq; ohio.athletes_core, 1);

    check!(eq; report.off_cohort_athletes, 0);
    check!(eq; report.read, report.published_totals());
    check!(eq; report.read.schools, 1);
    check!(eq; report.read.athletes, 2);
    check!(eq; report.read.coaches, 0);
    check!(eq; report.read.meets, 0);
    check!(eq; report.read.performances, 0);
    report.reconcile()?;

    let scope_note = report
        .notes
        .iter()
        .find(|note| note.contains("outside the census run scope"));
    let scope_note = scope_note.ok_or("missing out-of-scope jurisdiction note")?;
    check!(
        scope_note.contains("schools=1 athletes=1 coaches=1 meets=1 performances=1"),
        "{scope_note}"
    );
    Ok(())
}
