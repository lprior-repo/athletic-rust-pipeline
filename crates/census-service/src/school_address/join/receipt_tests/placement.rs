use super::*;

#[test]
fn missing_review_in_a_new_partition_reinserts_only_review_after_mixed_application() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let g1 = fixture::advance_without_reviews(&store, "g1")?;
    let (mut school, _) = CanonicalSchool::new(
        UsJurisdiction::Ohio,
        "Springfield High School",
        "springfield high school",
        Some("Springfield"),
    );
    school.co_op = true;
    school.aliases.push("Riverside High School".to_string());
    store.append(Table::Schools, &school)?;
    let index = mixed_index()?;
    let lanes = lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-08-01"));
    let (first, _) = process(&store, &index, &lanes, Mode::Apply)?;
    check!(eq; first.linked, 1);
    check!(eq; first.review_filed, 1);
    let first_witness = witness::single(&store, &school.id, "schools-and-review-cases")?;
    check!(eq; witness::effect(&first_witness, Table::ReviewCases)?.get("placement"), Some(&json!({"derived-generation": g1})));
    let original_cases = store.scan::<ReviewCase>(Table::ReviewCases)?;
    check!(eq; original_cases.len(), 1);
    let (store, applied) = physical::checkpoint(store)?;
    check!(eq; applied.schools()?.len(), 2);
    let g2 = fixture::advance_without_reviews(&store, "g2")?;
    check!(g2 > g1);
    check!(eq; store.scan::<ReviewCase>(Table::ReviewCases)?.len(), 0);
    let (store, omitted) = physical::checkpoint(store)?;
    check!(eq; omitted.schools()?, applied.schools()?);
    check!(eq; omitted.applications()?, applied.applications()?);
    let (reinserted, _) = process(&store, &index, &lanes, Mode::Apply)?;
    check!(eq; reinserted.linked, 0);
    check!(eq; reinserted.already_linked, 1);
    check!(eq; reinserted.backfilled, 0);
    check!(eq; reinserted.review_filed, 1);
    check!(eq; store.scan::<ReviewCase>(Table::ReviewCases)?, original_cases);
    let witnesses = witness::all(&store)?;
    check!(eq; witnesses.len(), 2);
    check!(witnesses.contains(&first_witness));
    let added = witnesses
        .iter()
        .find(|value| *value != &first_witness)
        .ok_or("new partition has no new witness")?;
    check!(eq; added.get("mutation").and_then(Value::as_str), Some("review-cases"));
    check!(eq; added.get("effects").and_then(Value::as_array).map(Vec::len), Some(1));
    let second_effect = witness::effect(added, Table::ReviewCases)?;
    check!(eq; second_effect.get("placement"), Some(&json!({"derived-generation": g2})));
    let (_store, after) = physical::checkpoint(store)?;
    check!(eq; after.schools()?, applied.schools()?);
    check!(eq; after.applications()?.len(), 2);
    Ok(())
}

fn mixed_index() -> TestResult<DirectoryIndex> {
    Ok(DirectoryIndex::build(&[
        nces_entry(
            "390000000001",
            "Springfield High School",
            "Springfield",
            UsJurisdiction::Ohio,
            "1 Main St",
            (9, 12),
        )?,
        nces_entry(
            "390000000002",
            "Riverside High School",
            "Springfield",
            UsJurisdiction::Ohio,
            "2 Main St",
            (9, 12),
        )?,
        nces_entry(
            "390000000003",
            "Riverside High School",
            "Springfield",
            UsJurisdiction::Ohio,
            "3 Main St",
            (9, 12),
        )?,
    ]))
}
