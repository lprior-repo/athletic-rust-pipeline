use super::*;
use crate::registry::{descriptor, descriptors, AccessClass};

#[test]
fn every_row_names_a_registered_slug() {
    for row in table() {
        assert!(
            descriptor(row.slug).is_some(),
            "applicability names {:?}, which is not a registered source",
            row.slug
        );
    }
}

#[test]
fn the_table_names_every_registered_source_once() {
    let mut named: Vec<&str> = table().iter().map(|row| row.slug).collect();
    named.sort_unstable();
    let mut registered: Vec<&str> = descriptors().map(|entry| entry.slug).collect();
    registered.sort_unstable();
    assert_eq!(
        named, registered,
        "the applicability table and the registry do not name the same sources"
    );
}

#[test]
fn every_listed_jurisdiction_is_in_scope_and_listed_once() {
    for row in table() {
        let mut seen: Vec<UsJurisdiction> = Vec::new();
        for jurisdiction in row.jurisdictions {
            assert!(
                jurisdiction.is_in_census_scope(),
                "{} plans {jurisdiction:?}, which is outside the census scope",
                row.slug
            );
            assert!(
                !seen.contains(jurisdiction),
                "{} lists {jurisdiction:?} twice",
                row.slug
            );
            seen.push(*jurisdiction);
        }
        assert!(
            !seen.is_empty(),
            "{} plans nothing, so it has no reason to be a row",
            row.slug
        );
    }
}

#[test]
fn every_census_jurisdiction_plans_both_national_platforms() {
    for jurisdiction in UsJurisdiction::CENSUS_SCOPE {
        let planned = applicable_sources(jurisdiction);
        for slug in ["athleticnet", "milesplit"] {
            assert!(
                planned.iter().any(|entry| entry.slug == slug),
                "{jurisdiction:?} does not plan {slug}"
            );
        }
    }
}

#[test]
fn a_plan_holds_no_source_twice_and_follows_bulk_first_order() {
    for jurisdiction in UsJurisdiction::CENSUS_SCOPE {
        let planned = applicable_sources(jurisdiction);
        let slugs: Vec<&str> = planned.iter().map(|entry| entry.slug).collect();

        let mut unique = slugs.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(
            unique.len(),
            slugs.len(),
            "{jurisdiction:?} plans a source more than once: {slugs:?}"
        );

        let expected: Vec<&str> = bulk_first(&slugs).iter().map(|entry| entry.slug).collect();
        assert_eq!(
            slugs, expected,
            "{jurisdiction:?} is not planned in bulk_first order"
        );
    }
}

#[test]
fn out_of_scope_jurisdictions_plan_nothing_and_every_jurisdiction_answers() {
    for jurisdiction in UsJurisdiction::EXCLUDED_FROM_CENSUS {
        assert!(
            applicable_sources(jurisdiction).is_empty(),
            "{jurisdiction:?} is excluded from the census scope but plans sources"
        );
    }
    for jurisdiction in UsJurisdiction::ALL {
        let planned = applicable_sources(jurisdiction);
        assert_eq!(
            planned.is_empty(),
            !jurisdiction.is_in_census_scope(),
            "{jurisdiction:?} answered inconsistently with is_in_census_scope"
        );
    }
}

#[test]
fn single_state_adapters_are_planned_in_their_own_state_only() {
    let homes = [
        ("aia", UsJurisdiction::Arizona),
        ("ihsa", UsJurisdiction::Illinois),
        ("ks", UsJurisdiction::Kansas),
        ("mshsl", UsJurisdiction::Minnesota),
        ("ohsaa", UsJurisdiction::Ohio),
        ("wiaa", UsJurisdiction::Wisconsin),
        ("wiaa_results", UsJurisdiction::Wisconsin),
        ("uhsaa", UsJurisdiction::Utah),
    ];
    for (slug, home) in homes {
        for jurisdiction in UsJurisdiction::CENSUS_SCOPE {
            let planned = applicable_sources(jurisdiction)
                .iter()
                .any(|entry| entry.slug == slug);
            assert_eq!(
                planned,
                jurisdiction == home,
                "{slug} planned for {jurisdiction:?} (home {home:?})"
            );
        }
    }
}

#[test]
fn artifact_adapters_are_planned_where_the_platform_is_evidenced(
) -> Result<(), Box<dyn std::error::Error>> {
    let planned = applicable_sources(UsJurisdiction::Wisconsin);
    let harvest = planned
        .iter()
        .find(|entry| entry.slug == "athleticlive")
        .ok_or("Wisconsin plans the AthleticLIVE harvest")?;
    let actual = harvest.access_class();
    if actual != AccessClass::Artifact {
        return Err(format!(
            "access class: left: {actual:?}, right: {:?}",
            AccessClass::Artifact
        )
        .into());
    }
    if applicable_sources(UsJurisdiction::SouthDakota)
        .iter()
        .any(|entry| entry.slug == "athleticlive")
    {
        return Err(
            "South Dakota records no AthleticLIVE coverage, so the harvest is not planned".into(),
        );
    }
    Ok(())
}

#[test]
fn tfrrs_is_planned_only_where_high_school_depth_is_evidenced() {
    for jurisdiction in [
        UsJurisdiction::Indiana,
        UsJurisdiction::Florida,
        UsJurisdiction::NewHampshire,
    ] {
        assert!(
            applicable_sources(jurisdiction)
                .iter()
                .any(|entry| entry.slug == "tfrrs"),
            "{jurisdiction:?} does not plan tfrrs"
        );
    }
    for jurisdiction in [
        UsJurisdiction::Wisconsin,
        UsJurisdiction::Minnesota,
        UsJurisdiction::Ohio,
        UsJurisdiction::Illinois,
    ] {
        assert!(
            !applicable_sources(jurisdiction)
                .iter()
                .any(|entry| entry.slug == "tfrrs"),
            "{jurisdiction:?} plans tfrrs, whose HS depth the research does not evidence there"
        );
    }
}

#[test]
fn every_row_states_its_evidence_and_its_refusal() {
    for row in table() {
        assert!(
            !row.evidence.trim().is_empty(),
            "{} has no evidence",
            row.slug
        );
        assert!(
            !row.refusal.trim().is_empty(),
            "{} has no refusal",
            row.slug
        );
    }
}
