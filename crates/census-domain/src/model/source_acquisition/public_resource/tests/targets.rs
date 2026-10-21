use super::*;

#[test]
fn native_targets_allow_only_the_bound_path_and_optional_ascii_primary_slug() -> TestResult {
    for (resource, target) in resources()?.into_iter().take(5) {
        resource.validate_target(UsJurisdiction::Michigan, target)?;
        for suffix in ["/", "/extra", "?", "?login=1", "#", "#result", "%2fextra"] {
            check!(resource
                .validate_target(UsJurisdiction::Michigan, &format!("{target}{suffix}"))
                .is_err());
        }
        for changed in [
            target.replace("https:", "http:"),
            target.replace("mi.milesplit.com", "mn.milesplit.com"),
            target.replace("mi.milesplit.com", "mi.milesplit.com.evil.org"),
            target.replace("mi.milesplit.com", "user@mi.milesplit.com"),
            target.replace("mi.milesplit.com", "mi.milesplit.com:444"),
            target.replace("mi.milesplit.com", "mi.milesplit.com:443"),
            target.replace("https://", "https://MI.MILESPLIT.COM@"),
        ] {
            check!(resource
                .validate_target(UsJurisdiction::Michigan, &changed)
                .is_err());
        }
        check!(resource
            .validate_target(UsJurisdiction::Ohio, target)
            .is_err());
    }
    Ok(())
}

#[test]
fn exact_native_ids_and_secondary_hierarchy_never_accept_substitution() -> TestResult {
    let cases = [
        (
            PublicSourceResourceV1::milesplit_athlete_profile("13749840")?,
            "/athletes/13749841-brennan-smith",
        ),
        (
            PublicSourceResourceV1::milesplit_team_roster("12439")?,
            "/teams/012439-school/roster",
        ),
        (
            PublicSourceResourceV1::milesplit_meet_team("760382", "12439")?,
            "/meets/760382-meet/teams/12684",
        ),
        (
            PublicSourceResourceV1::milesplit_meet_team("760382", "12439")?,
            "/meets/760383-meet/teams/12439",
        ),
        (
            PublicSourceResourceV1::milesplit_meet_team("760382", "12439")?,
            "/meets/760382-meet/teams/12439-school",
        ),
        (
            PublicSourceResourceV1::milesplit_raw_results("760382", "1300204")?,
            "/meets/760382-meet/results/1300205/raw",
        ),
        (
            PublicSourceResourceV1::milesplit_raw_results("760382", "1300204")?,
            "/meets/760382-meet/results/1300204-final/raw",
        ),
        (
            PublicSourceResourceV1::milesplit_team_meet_history("12439")?,
            "/teams/12439-school/results",
        ),
    ];
    for (resource, path) in cases {
        let target = format!("https://mi.milesplit.com{path}");
        check!(resource
            .validate_target(UsJurisdiction::Michigan, &target)
            .is_err());
    }
    Ok(())
}

#[test]
fn slug_changes_do_not_change_resource_bytes_and_encoded_paths_refuse() -> TestResult {
    let resource = PublicSourceResourceV1::milesplit_athlete_profile("13749840")?;
    let bytes = serde_json::to_vec(&resource)?;
    for tail in [
        "13749840",
        "13749840-brennan-smith",
        "13749840-Changed-2026",
    ] {
        resource.validate_target(
            UsJurisdiction::Michigan,
            &format!("https://mi.milesplit.com/athletes/{tail}"),
        )?;
        check!(eq; serde_json::to_vec(&resource)?, bytes);
    }
    for tail in [
        "13749840-",
        "13749840---",
        "13749840-a_b",
        "13749840-%61",
        "13749840-é",
        "13749840/name",
        "%31%33%37%34%39%38%34%30",
        "13749840/../13749840",
        "../athletes/13749840",
        "13749840\\extra",
        "13749840;login",
    ] {
        check!(resource
            .validate_target(
                UsJurisdiction::Michigan,
                &format!("https://mi.milesplit.com/athletes/{tail}")
            )
            .is_err());
    }
    Ok(())
}

#[test]
fn every_jurisdiction_uses_its_own_state_host() -> TestResult {
    let resource = PublicSourceResourceV1::milesplit_team_roster("12684")?;
    for site in UsJurisdiction::ALL {
        let target = format!(
            "https://{}.milesplit.com/teams/12684-marquette/roster",
            site.code().to_ascii_lowercase()
        );
        resource.validate_target(site, &target)?;
    }
    Ok(())
}

#[test]
fn resource_roles_never_substitute_another_native_endpoint() -> TestResult {
    let native = resources()?.into_iter().take(5).collect::<Vec<_>>();
    for (resource, own_target) in &native {
        for (_, other_target) in &native {
            if own_target != other_target {
                check!(resource
                    .validate_target(UsJurisdiction::Michigan, other_target)
                    .is_err());
            }
        }
    }
    check!(ne; PublicSourceResourceV1::milesplit_team_roster("12684")?, PublicSourceResourceV1::milesplit_team_meet_history("12684")?);
    Ok(())
}
