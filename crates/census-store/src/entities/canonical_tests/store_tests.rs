use super::*;

#[test]
fn two_same_named_athletes_from_different_schools_stay_two_rows() {
    let Ok(dir) = tempfile::tempdir() else {
        panic!("a temporary directory has to be creatable");
    };
    let Ok(store) = Store::open(dir.path()) else {
        panic!("a store has to open on a fresh directory");
    };

    let mut abbotsford = CanonicalAthlete::new(
        &school(),
        "Jordan Blake",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "111"),
    );
    abbotsford.source_links.push(SourceIdentity::new(
        SourceNamespace::TfrrsAthlete,
        "333",
    ));
    let mut marshall = CanonicalAthlete::new(
        &second_school(),
        "Jordan Blake",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "222"),
    );
    marshall.source_links.push(SourceIdentity::new(
        SourceNamespace::TfrrsAthlete,
        "444",
    ));

    assert_ne!(
        abbotsford.id, marshall.id,
        "a name at two schools is two athletes, never one"
    );

    let observations = [abbotsford.clone(), marshall.clone()];
    let Ok(()) = store.append_many(Table::Athletes, &observations) else {
        panic!("two athlete observations have to append");
    };
    let Ok(rows) = store.scan::<CanonicalAthlete>(Table::Athletes) else {
        panic!("the athlete table has to scan");
    };

    assert_eq!(
        rows.len(),
        2,
        "two subjects read back as two rows: neither school may be folded away"
    );
    for expected in [&abbotsford, &marshall] {
        let Some(row) = rows.iter().find(|row| row.id == expected.id) else {
            panic!("the athlete minted at its own school has to come back");
        };
        assert_eq!(
            row.canonical_name, expected.canonical_name,
            "the name is shared, so it cannot be what told the rows apart"
        );
        assert_eq!(
            row.school, expected.school,
            "each row keeps the school it was minted at"
        );
        let expected_identities: Vec<_> = expected.identities().collect();
        let row_identities: Vec<_> = row.identities().collect();
        assert_eq!(
            row_identities, expected_identities,
            "only the provider that saw this athlete sits on this row"
        );
        assert!(
            row.retained_conflicts.is_empty(),
            "two schools under one name is not a collision: nothing was retained"
        );
    }
}

#[test]
fn an_athlete_observed_once_reads_back_with_derived_confidence() {
    let Ok(dir) = tempfile::tempdir() else {
        panic!("a temporary store directory has to exist");
    };
    let Ok(store) = Store::open(dir.path()) else {
        panic!("the store has to open");
    };
    let id = CanonicalAthlete::mint(
        &school(),
        "Diego Ramos",
        GradYear::CO2027,
        Gender::Boys,
        &SourceIdentity::new(SourceNamespace::MilesplitAthlete, "111"),
    );
    let mut row = athlete(
        &id,
        "Diego Ramos",
        Gender::Boys,
        SourceNamespace::MilesplitAthlete,
        "111",
    );
    observing(&mut row, 11, 2025);
    let Ok(()) = store.append_many(Table::Athletes, &[row]) else {
        panic!("one athlete observation has to append");
    };

    let Ok(rows) = store.scan::<CanonicalAthlete>(Table::Athletes) else {
        panic!("the athlete table has to scan");
    };

    assert_eq!(rows.len(), 1, "one observation of one athlete is one row");
    assert_eq!(
        rows[0].derived_cohort_confidence(),
        Some(Confidence::HIGH),
        "the read derives the cohort confidence"
    );
}
