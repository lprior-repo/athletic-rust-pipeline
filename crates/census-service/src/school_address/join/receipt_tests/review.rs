use super::*;

#[test]
fn ambiguity_is_receipted_once_and_every_existing_decided_state_is_retained() -> TestResult {
    for state in [
        ReviewState::Resolved,
        ReviewState::Retained,
        ReviewState::Superseded,
    ] {
        let (_dir, store) = store_with_school()?;
        let school = merged_school(&store)?;
        let index = fixture::ambiguous_index(&school.name, "Springfield", UsJurisdiction::Ohio)?;
        let lanes = lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-08-01"));
        let (first, _) = process(&store, &index, &lanes, Mode::Apply)?;
        check!(eq; first.review_filed, 1);
        check!(eq; first.review_present, 0);
        let first_witness = witness::single(&store, &school.id, "review-cases")?;
        let effect = witness::effect(&first_witness, Table::ReviewCases)?;
        check!(eq; effect.get("placement"), Some(&json!({"derived-generation": store.derived_generation()})));
        let mut cases = store.scan::<ReviewCase>(Table::ReviewCases)?;
        check!(eq; cases.len(), 1);
        let case = cases.first_mut().ok_or("ambiguity did not create a case")?;
        check!(eq; case.state, ReviewState::Pending);
        check!(eq; case.family.as_str(), SCHOOL_IDENTITY_FAMILY);
        check!(eq; case.subject_id.as_str(), school.id.as_str());
        check!(case.detail.contains("nces:390000000002"));
        check!(case.detail.contains("nces:390000000003"));
        case.state = state;
        store.replace_many(Table::ReviewCases, &cases)?;
        let (store, decided) = physical::checkpoint(store)?;
        check!(eq; decided.schools()?.len(), 1);
        check!(eq; decided.applications()?.len(), 1);
        let (again, _) = process(&store, &index, &lanes, Mode::Apply)?;
        check!(eq; again.review, 1);
        check!(eq; again.review_filed, 0);
        check!(eq; again.review_present, 1);
        check!(eq; store.scan::<ReviewCase>(Table::ReviewCases)?, cases);
        check!(eq; witness::all(&store)?, vec![first_witness]);
        let (store, after) = physical::checkpoint(store)?;
        check!(eq; after, decided);
        check!(eq; store.scan::<ReviewCase>(Table::ReviewCases)?, cases);
    }
    Ok(())
}

#[test]
fn repeated_co_op_case_id_keeps_decision_counters_but_one_physical_case() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let school = co_op_school(
        "Franklin County Coop",
        "Riverside",
        &["Riverside High School", "Riverside High School"],
    )?;
    store.append(Table::Schools, &school)?;
    let index = fixture::ambiguous_index(
        "Riverside High School",
        "Riverside",
        UsJurisdiction::Tennessee,
    )?;
    let lanes = lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-08-01"));
    let (first, _) = process(&store, &index, &lanes, Mode::Apply)?;
    check!(eq; first.review, 2);
    check!(eq; first.review_filed, 2);
    check!(eq; first.review_present, 0);
    let cases = store.scan::<ReviewCase>(Table::ReviewCases)?;
    check!(eq; cases.len(), 1);
    let first_witness = witness::single(&store, &school.id, "review-cases")?;
    let effect = witness::effect(&first_witness, Table::ReviewCases)?;
    check!(eq; effect.get("payload_digest").and_then(Value::as_str), Some(serialized_digest(&cases)?.as_str()));
    let (store, applied) = physical::checkpoint(store)?;
    check!(eq; applied.schools()?.len(), 1);
    check!(eq; applied.applications()?.len(), 1);
    let (again, _) = process(&store, &index, &lanes, Mode::Apply)?;
    check!(eq; again.review, 2);
    check!(eq; again.review_filed, 0);
    check!(eq; again.review_present, 2);
    let (_store, after) = physical::checkpoint(store)?;
    check!(eq; after, applied);
    Ok(())
}

#[test]
fn co_op_school_effects_keep_first_occurrence_website_and_physical_order() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let school = co_op_school(
        "Franklin County Coop",
        "Riverside",
        &["East High School", "West High School"],
    )?;
    store.append(Table::Schools, &school)?;
    let lanes = lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-08-01"));
    let (index, expected_website) = website_fixture(&store, &school, &lanes)?;
    let (first, _) = process(&store, &index, &lanes, Mode::Apply)?;
    check!(eq; first.linked, 2);
    check!(eq; first.websites, 2);
    let merged = merged_school(&store)?;
    check!(eq; merged.school_website.as_deref(), Some(expected_website.as_str()));
    check!(eq; merged.source_identities.len(), 2);
    check!(eq; merged.postal_addresses.len(), 2);
    let first_witness = witness::single(&store, &school.id, "schools")?;
    let (store, applied) = physical::checkpoint(store)?;
    let rows = applied.schools()?;
    check!(eq; rows.len(), 3);
    let observations = rows
        .iter()
        .map(|(_, bytes)| serde_json::from_slice::<CanonicalSchool>(bytes))
        .collect::<Result<Vec<_>, _>>()?;
    let payloads = observations
        .get(1..)
        .ok_or("co-op application has no appended payloads")?;
    check!(eq; payloads.first().and_then(|row| row.school_website.as_deref()), Some(expected_website.as_str()));
    check!(eq; payloads.last().and_then(|row| row.school_website.as_deref()), Some("https://west.example/high"));
    let first = payloads
        .first()
        .ok_or("website fixture lacks first physical payload")?;
    let last = payloads
        .last()
        .ok_or("website fixture lacks second physical payload")?;
    check!(serialized_digest(first)? > serialized_digest(last)?);
    let effect = witness::effect(&first_witness, Table::Schools)?;
    check!(eq; effect.get("payload_digest").and_then(Value::as_str), Some(serialized_digest(payloads)?.as_str()));
    let (again, _) = process(&store, &index, &lanes, Mode::Apply)?;
    check!(eq; again.linked, 0);
    check!(eq; again.already_linked, 2);
    let (_store, after) = physical::checkpoint(store)?;
    check!(eq; after, applied);
    Ok(())
}

fn website_fixture(
    store: &Store,
    school: &CanonicalSchool,
    lanes: &LaneSet,
) -> TestResult<(DirectoryIndex, String)> {
    for suffix in 0..64 {
        let expected_website = format!("https://east.example/high/{suffix}");
        let index = DirectoryIndex::build(&[
            entry_with_website(
                "470000000010",
                "East High School",
                "Riverside",
                UsJurisdiction::Tennessee,
                "1 East Rd",
                &expected_website,
            )?,
            entry_with_website(
                "470000000011",
                "West High School",
                "Riverside",
                UsJurisdiction::Tennessee,
                "2 West Rd",
                "https://west.example/high",
            )?,
        ]);
        let snapshot = store.snapshot();
        let mut prospective = Job::new(&index, lanes, Mode::Apply, store, &snapshot)?;
        prospective.visit(school)?;
        check!(eq; prospective.pending.len(), 2);
        let first = prospective
            .pending
            .first()
            .ok_or("fixture lacks first school effect")?;
        let last = prospective
            .pending
            .last()
            .ok_or("fixture lacks second school effect")?;
        let opposed = serialized_digest(first)? > serialized_digest(last)?;
        drop(prospective);
        if opposed {
            return Ok((index, expected_website));
        }
    }
    Err("bounded fixture search did not produce opposing hash order".into())
}
