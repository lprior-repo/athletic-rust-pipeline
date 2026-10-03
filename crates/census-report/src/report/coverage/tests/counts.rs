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
    check!(eq; wi.core_share_pct, 50);
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
    check!(eq; ks.core_share_pct, 0);

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

    let report = coverage_of(&store, Some(2027))?;
    let ohio = row(&report, "OH")?;
    check!(eq; ohio.athletes, 2);
    check!(eq; ohio.athletes_core, 1);
    check!(eq; ohio.core_share_pct, 50);
    check!(eq; ohio.sources.get("athleticlive_results"), Some(&1));
    check!(eq; ohio.sources.get("ohsaa_results"), Some(&1));

    let core = census_of(&store, crate::report::Scope::Core)?;
    check!(eq; core.totals.athletes, 1);
    Ok(())
}
