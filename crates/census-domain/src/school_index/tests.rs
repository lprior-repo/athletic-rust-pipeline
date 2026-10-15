use super::*;
use crate::model::CanonicalSchool;

fn index() -> SchoolIndex {
    let names = [
        ("Milwaukee Bradley Tech", "milwaukee bradley tech"),
        ("Brookfield Central", "brookfield central"),
        ("Madison La Follette", "madison la follette"),
        ("Milwaukee King", "milwaukee king"),
        ("Wisconsin Lutheran", "wisconsin lutheran"),
        ("Eau Claire Memorial", "eau claire memorial"),
        ("Homestead", "homestead"),
        ("West De Pere", "west de pere"),
        ("Franklin", "franklin"),
    ];
    let schools: Vec<CanonicalSchool> = names
        .iter()
        .map(|(name, normalized)| {
            CanonicalSchool::new(UsJurisdiction::Wisconsin, *name, *normalized, None).0
        })
        .collect();
    SchoolIndex::from_schools(&schools)
}

#[test]
fn exact_and_abbreviated_labels_resolve() {
    let index = index();
    assert_eq!(
        index
            .resolve(UsJurisdiction::Wisconsin, "West De Pere")
            .map(|(_, kind)| kind),
        Some(SchoolMatch::Exact)
    );
    assert_eq!(
        index
            .resolve(UsJurisdiction::Wisconsin, "Milw. Bradley Tech")
            .map(|(_, kind)| kind),
        Some(SchoolMatch::Abbreviation)
    );
}

#[test]
fn truncated_and_prefixed_labels_resolve_to_the_unique_school() {
    let index = index();
    assert_eq!(
        index
            .resolve(UsJurisdiction::Wisconsin, "Brookfield Cent.")
            .map(|(_, kind)| kind),
        Some(SchoolMatch::Partial)
    );
    assert_eq!(
        index
            .resolve(UsJurisdiction::Wisconsin, "La Follette")
            .map(|(_, kind)| kind),
        Some(SchoolMatch::Partial)
    );
    assert_eq!(
        index
            .resolve(UsJurisdiction::Wisconsin, "Wisconsin Luth.")
            .map(|(_, kind)| kind),
        Some(SchoolMatch::Partial)
    );
    assert_eq!(
        index
            .resolve(UsJurisdiction::Wisconsin, "EC Memorial")
            .map(|(_, kind)| kind),
        Some(SchoolMatch::Abbreviation)
    );
}

#[test]
fn relay_squad_letters_are_not_part_of_the_school_name() {
    let index = index();
    assert_eq!(
        index
            .resolve(UsJurisdiction::Wisconsin, "Homestead A")
            .map(|(_, kind)| kind),
        Some(SchoolMatch::Exact)
    );
    assert_eq!(
        index
            .resolve(UsJurisdiction::Wisconsin, "Homestead B")
            .map(|(_, kind)| kind),
        Some(SchoolMatch::Exact)
    );
    assert_eq!(
        index
            .resolve(UsJurisdiction::Wisconsin, "Milw. Bradley Tech A")
            .map(|(_, kind)| kind),
        Some(SchoolMatch::Abbreviation)
    );
}

#[test]
fn ambiguous_labels_stay_unresolved() {
    let schools: Vec<CanonicalSchool> = ["Madison Memorial", "Milwaukee Memorial"]
        .iter()
        .map(|name| {
            CanonicalSchool::new(UsJurisdiction::Wisconsin, *name, normalize_name(name), None).0
        })
        .collect();
    let index = SchoolIndex::from_schools(&schools);
    assert_eq!(index.resolve(UsJurisdiction::Wisconsin, "Memorial"), None);
}

#[test]
fn matching_never_crosses_state_lines() {
    let index = index();
    assert_eq!(
        index.resolve(UsJurisdiction::Minnesota, "West De Pere"),
        None
    );
}

#[test]
fn shared_exact_alias_is_unresolved_in_both_orders() {
    let (mut abundant, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Abundant Life Christian",
        "abundant life christian",
        Some("Madison"),
    );
    abundant.aliases.push("Madison WI".to_string());
    let (mut east, east_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Madison East",
        "madison east",
        Some("Madison"),
    );
    east.aliases.push("Madison WI".to_string());
    for index in [
        SchoolIndex::from_schools(&[abundant.clone(), east.clone()]),
        SchoolIndex::from_schools(&[east.clone(), abundant.clone()]),
    ] {
        assert_eq!(
            index.resolve_label(UsJurisdiction::Wisconsin, "Madison WI"),
            SchoolResolution::Ambiguous
        );
        assert_eq!(index.resolve(UsJurisdiction::Wisconsin, "Madison WI"), None);
        assert_eq!(
            index.resolve(UsJurisdiction::Wisconsin, "Madison East"),
            Some((east_id.clone(), SchoolMatch::Exact))
        );
    }
    let (mut one_owner, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Madison East",
        "madison east",
        Some("Madison"),
    );
    one_owner.aliases.push("Madison WI".to_string());
    let (unaliased, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Madison West",
        "madison west",
        Some("Madison"),
    );
    let unique = SchoolIndex::from_schools(&[one_owner, unaliased]);
    assert_eq!(
        unique.resolve_label(UsJurisdiction::Wisconsin, "Madison WI"),
        SchoolResolution::Resolved(east_id, SchoolMatch::Exact)
    );
}
use super::{SchoolIndexBuildError, SchoolIndexCapacity};

#[test]
fn fallible_constructor_builds_identical_index() {
    let names = [
        ("Milwaukee Bradley Tech", "milwaukee bradley tech"),
        ("Brookfield Central", "brookfield central"),
    ];
    let schools: Vec<CanonicalSchool> = names
        .iter()
        .map(|(name, normalized)| {
            CanonicalSchool::new(UsJurisdiction::Wisconsin, *name, *normalized, None).0
        })
        .collect();

    let capacity = SchoolIndexCapacity::compute(&schools);
    let fallible = SchoolIndex::try_from_schools(&schools, &capacity).unwrap();
    let infallible = SchoolIndex::from_schools(&schools);

    assert_eq!(fallible.school_count(), infallible.school_count());
    assert_eq!(
        fallible.resolve(UsJurisdiction::Wisconsin, "Bradley Tech"),
        infallible.resolve(UsJurisdiction::Wisconsin, "Bradley Tech")
    );
    assert_eq!(
        fallible.resolve(UsJurisdiction::Wisconsin, "Brookfield Central"),
        infallible.resolve(UsJurisdiction::Wisconsin, "Brookfield Central")
    );
}

#[test]
fn capacity_plan_computes_exact_counts() {
    let (mut school1, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Test School",
        "test school",
        None,
    );
    school1.aliases.push("TS".to_string());
    school1.aliases.push("T-School".to_string());
    let (school2, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Another School",
        "another school",
        None,
    );

    let schools = vec![school1, school2];
    let capacity = SchoolIndexCapacity::compute(&schools);

    // 2 schools → 2 names entries (×2 growth = 4)
    // 2 aliases → 2 aliases entries (×2 growth = 4)
    // 2 schools → 2 by_state entries
    assert_eq!(capacity.names_entries, 4);
    assert_eq!(capacity.aliases_entries, 4);
    assert_eq!(capacity.by_state_schools, 2);
}

#[test]
fn school_without_state_is_skipped_in_capacity() {
    let (mut school, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Test School",
        "test school",
        None,
    );
    school.state = None;

    let schools = vec![school];
    let capacity = SchoolIndexCapacity::compute(&schools);

    assert_eq!(capacity.names_entries, 0);
    assert_eq!(capacity.aliases_entries, 0);
    assert_eq!(capacity.by_state_schools, 0);
}
