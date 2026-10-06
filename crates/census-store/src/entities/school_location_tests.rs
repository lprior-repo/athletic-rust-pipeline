use crate::{Entity, Store, Table};
use census_domain::jurisdiction::UsJurisdiction;
use census_domain::model::{CanonicalSchool, CANONICAL_ID_COLLISION_FAMILY};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn memorial(city: Option<&str>) -> (CanonicalSchool, String) {
    let (school, id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Memorial High School",
        "memorial",
        city,
    );
    (school, id.as_str().to_string())
}

fn check(condition: bool, detail: &str) -> TestResult {
    if condition {
        Ok(())
    } else {
        Err(detail.into())
    }
}

#[test]
fn one_name_in_two_cities_mints_two_schools() -> TestResult {
    let (_, unlocated) = memorial(None);
    let (_, madison) = memorial(Some("Madison"));
    let (_, milwaukee) = memorial(Some("Milwaukee"));
    check(
        unlocated != madison,
        "the located id differs from the bare one",
    )?;
    check(madison != milwaukee, "two cities are two identities")?;
    Ok(())
}

#[test]
fn one_id_whose_rows_disagree_on_city_retains_the_conflict() -> TestResult {
    let (mut kept, id) = memorial(None);
    kept.city = Some("Madison".to_string());
    let (mut dropped, _) = memorial(None);
    dropped.city = Some("Marshfield".to_string());
    dropped.aliases.push("Marshfield Memorial".to_string());
    kept.merge(dropped);
    check(
        kept.city.as_deref() == Some("Madison"),
        "the kept row keeps its own city",
    )?;
    check(
        kept.aliases.is_empty(),
        "the dropped row's aliases are not blended onto the kept school",
    )?;
    let conflict = kept
        .retained_conflicts
        .first()
        .ok_or("disagreeing city rows under one id have to retain a finding")?;
    check(
        conflict.family == CANONICAL_ID_COLLISION_FAMILY,
        "the finding is an id collision",
    )?;
    check(conflict.subject_id == id, "the finding names the shared id")?;
    Ok(())
}

#[test]
fn consolidation_keeps_same_name_schools_of_two_cities_apart() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let (madison, madison_id) = memorial(Some("Madison"));
    let (milwaukee, milwaukee_id) = memorial(Some("Milwaukee"));
    store.append(Table::Schools, &madison)?;
    store.append(Table::Schools, &milwaukee)?;
    let rows = store.scan::<CanonicalSchool>(Table::Schools)?;
    check(rows.len() == 2, "two cities stay two consolidated schools")?;
    let ids: Vec<&str> = rows.iter().map(|row| row.id.as_str()).collect();
    check(
        ids.contains(&madison_id.as_str()) && ids.contains(&milwaukee_id.as_str()),
        "both located schools survive consolidation under their own ids",
    )?;
    check(
        rows.iter().all(|row| row.retained_conflicts.is_empty()),
        "distinct located ids are not a collision",
    )?;
    Ok(())
}

#[test]
fn consolidation_retains_the_legacy_city_disagreement_under_one_id() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let (mut first, _) = memorial(None);
    first.city = Some("Madison".to_string());
    store.append(Table::Schools, &first)?;
    let (mut second, _) = memorial(None);
    second.city = Some("Marshfield".to_string());
    second.aliases.push("Marshfield Memorial".to_string());
    store.append(Table::Schools, &second)?;
    let rows = store.scan::<CanonicalSchool>(Table::Schools)?;
    check(rows.len() == 1, "the legacy two-part id is one row")?;
    let row = rows.first().ok_or("the consolidated school")?;
    check(
        row.retained_conflicts.len() == 1,
        "the disagreement is retained, not blended",
    )?;
    check(
        row.city.as_deref() == Some("Madison"),
        "the kept row's city is unchanged",
    )?;
    check(
        row.aliases.is_empty(),
        "the dropped row's aliases are not unioned",
    )?;
    Ok(())
}
