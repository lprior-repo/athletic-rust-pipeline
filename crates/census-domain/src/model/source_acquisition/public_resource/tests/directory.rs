use super::*;
use crate::model::{
    AcquisitionDigest, AcquisitionWorkV1, CaptureCacheIntent, CensusRun, HistoryWindow,
    ProfileDate, PublicResourceCaptureScopeV1, PublicResourceObligationV1, RequestRepresentationV1,
    StoreIdentity,
};

const WIRE: &str = r#"{"kind":"mile_split_team_directory"}"#;
const AS_OF: &str = "2026-10-21";
const TARGETS: [(UsJurisdiction, &str); 5] = [
    (UsJurisdiction::Michigan, "https://mi.milesplit.com/teams"),
    (UsJurisdiction::Minnesota, "https://mn.milesplit.com/teams"),
    (UsJurisdiction::Iowa, "https://ia.milesplit.com/teams"),
    (UsJurisdiction::Illinois, "https://il.milesplit.com/teams"),
    (UsJurisdiction::Colorado, "https://co.milesplit.com/teams"),
];

#[test]
fn directory_wire_is_literal_closed_and_has_no_invented_native_identity() -> TestResult {
    let resource = PublicSourceResourceV1::milesplit_team_directory();
    check!(eq; serde_json::to_string(&resource)?, WIRE);
    check!(eq; resource.kind(), PublicSourceResourceKindV1::MileSplitTeamDirectory);
    let cold: PublicSourceResourceV1 = serde_json::from_str(WIRE)?;
    check!(eq; cold, resource);
    check!(eq; serde_json::to_string(&cold)?, WIRE);
    check!(cold.athlete_id().is_none());
    check!(cold.team_id().is_none());
    check!(cold.meet_id().is_none());
    check!(cold.result_id().is_none());
    check!(cold.provider_school_id().is_none());
    check!(cold.origin().is_none());
    check!(cold.sport().is_none());
    check!(cold.school_year().is_none());
    Ok(())
}

#[test]
fn directory_deserialization_refuses_every_unmodeled_field_and_invalid_shape() -> TestResult {
    let original: serde_json::Value = serde_json::from_str(WIRE)?;
    for field in [
        "athlete_id",
        "team_id",
        "meet_id",
        "result_id",
        "provider_school_id",
        "origin",
        "sport",
        "school_year",
        "source_site",
        "target_url",
        "slug",
        "query",
        "profile",
        "as_of",
        "plan",
        "retry",
        "version",
        "unknown",
    ] {
        let mut changed = original.clone();
        changed
            .as_object_mut()
            .ok_or("directory object")?
            .insert(field.into(), serde_json::json!("1"));
        check!(
            serde_json::from_value::<PublicSourceResourceV1>(changed).is_err(),
            "directory accepted unmodeled field {field}"
        );
    }
    for raw in [
        r#"{}"#,
        r#"{"kind":"milesplit_team_directory"}"#,
        r#"{"kind":"mile_split_team_directory","kind":"mile_split_team_directory"}"#,
        r#"{"kind":"mile_split_team_directory","kind":"robots"}"#,
        r#"{"kind":{"mile_split_team_directory":{}}}"#,
        r#"{"mile_split_team_directory":{}}"#,
        r#"["mile_split_team_directory"]"#,
        r#""mile_split_team_directory""#,
        r#"null"#,
    ] {
        check!(
            serde_json::from_str::<PublicSourceResourceV1>(raw).is_err(),
            "directory accepted invalid wire {raw}"
        );
    }
    Ok(())
}

#[test]
fn five_selected_directories_accept_exact_normal_site_url_and_refuse_other_states() -> TestResult {
    let directory = PublicSourceResourceV1::milesplit_team_directory();
    for (site, target) in TARGETS {
        directory.validate_target(site, target)?;
        for (other_site, other_target) in TARGETS {
            if site != other_site {
                check!(
                    directory.validate_target(site, other_target).is_err(),
                    "{site:?} accepted foreign directory {other_target}"
                );
            }
        }
        check!(eq; serde_json::to_string(&directory)?, WIRE);
    }
    Ok(())
}

#[test]
fn directory_has_exact_path_without_native_ids_slug_query_or_authentication() -> TestResult {
    let directory = PublicSourceResourceV1::milesplit_team_directory();
    for (site, target) in TARGETS {
        for suffix in [
            "/",
            "/12439",
            "/12439-school/roster",
            "/meets",
            "-school",
            "s",
            "?",
            "?page=1",
            "?retry=2",
            "?login=1",
            "#",
            "#teams",
            ";login",
        ] {
            let changed = format!("{target}{suffix}");
            check!(
                directory.validate_target(site, &changed).is_err(),
                "directory accepted {changed}"
            );
        }
    }
    for path in [
        "/",
        "/Teams",
        "//teams",
        "/login/teams",
        "/auth/teams",
        "/account",
        "/athletes/13749840",
        "/teams/../teams",
        "/./teams",
        "/%74eams",
        "/teams%2f",
        "/teams\\",
        "/teams/roster",
        "/robots.txt",
    ] {
        let target = format!("https://mi.milesplit.com{path}");
        check!(
            directory
                .validate_target(UsJurisdiction::Michigan, &target)
                .is_err(),
            "directory accepted substituted path {target}"
        );
    }
    Ok(())
}

#[test]
fn directory_refuses_noncanonical_or_foreign_origins_and_credentials() -> TestResult {
    let directory = PublicSourceResourceV1::milesplit_team_directory();
    for target in [
        "http://mi.milesplit.com/teams",
        "https://mi.milesplit.com:443/teams",
        "https://mi.milesplit.com:444/teams",
        "https://mi.milesplit.com./teams",
        "https://MI.MILESPLIT.COM/teams",
        "https://mi.milesplit.com.evil.org/teams",
        "https://www.milesplit.com/teams",
        "https://milesplit.com/teams",
        "https://mi.milesplit.com@evil.org/teams",
        "https://user@mi.milesplit.com/teams",
        "https://user:password@mi.milesplit.com/teams",
        "https://127.0.0.1/teams",
        "https://[::1]/teams",
        "https://localhost/teams",
        " https://mi.milesplit.com/teams",
        "https://mi.milesplit.com/teams ",
        "https://mi.milesplit.com/teams\n",
    ] {
        check!(
            directory
                .validate_target(UsJurisdiction::Michigan, target)
                .is_err(),
            "directory accepted noncanonical target {target:?}"
        );
    }
    Ok(())
}

fn work(site: UsJurisdiction, target: &str, obligation_day: &str) -> TestResult<AcquisitionWorkV1> {
    let scope = PublicResourceCaptureScopeV1::new(
        &format!("jurisdiction:{}:2026-27:1", site.code()),
        site,
        CensusRun::new(SchoolYear::DEFAULT, 1).ok_or("directory run")?,
        AcquisitionDigest::parse(&"a".repeat(64))?,
        HistoryWindow::new(2023, 2026, AS_OF)?,
        AcquisitionDigest::parse(&"b".repeat(64))?,
        CaptureCacheIntent::ReuseEligible,
    )?;
    Ok(AcquisitionWorkV1::for_public_resource(
        StoreIdentity::parse(&"A".repeat(64))?,
        &scope,
        site,
        PublicResourceObligationV1::new(
            PublicSourceResourceV1::milesplit_team_directory(),
            ProfileDate::parse(obligation_day)?,
        ),
        RequestRepresentationV1::normalized_get(
            target,
            vec![
                ("accept-encoding".into(), "identity".into()),
                ("user-agent".into(), "Census/1.0".into()),
            ],
            "Census/1.0",
        )?,
    )?)
}

#[test]
fn directory_work_retains_public_scope_without_fake_meet_and_refuses_wrong_target_or_day(
) -> TestResult {
    let mut slots = Vec::new();
    for (site, target) in TARGETS {
        let original = work(site, target, AS_OF)?;
        check!(original.slot().source_meet_id().is_none());
        check!(original.slot().historical_year().is_none());
        check!(eq; original.slot().source_site(), site);
        check!(eq; original.slot().public_resource(), Some(&PublicSourceResourceV1::milesplit_team_directory()));
        check!(original.request().observed_http_required());
        let id = original.slot().id()?;
        check!(!slots.contains(&id));
        slots.push(id);
        let cold: AcquisitionWorkV1 = serde_json::from_slice(&serde_json::to_vec(&original)?)?;
        check!(eq; cold, original);
        check!(eq; cold.id()?, original.id()?);
        check!(work(site, target, "2026-10-20").is_err());
        for (other_site, other_target) in TARGETS {
            if site != other_site {
                check!(work(site, other_target, AS_OF).is_err());
            }
        }
    }
    Ok(())
}
