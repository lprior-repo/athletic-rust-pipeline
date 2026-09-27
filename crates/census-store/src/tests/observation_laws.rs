use super::*;

#[test]
fn keys_with_a_zero_low_sequence_byte_still_reopen() {
    let dir = tempfile::tempdir().unwrap();
    {
        let store = Store::open(dir.path()).unwrap();
        let rows: Vec<CanonicalSchool> = (0..300)
            .map(|index| school(&format!("School {index}")))
            .collect();
        store.append_many(Table::Schools, &rows).unwrap();
    }
    {
        let store = Store::open(dir.path()).unwrap();
        assert_eq!(
            store.scan::<CanonicalSchool>(Table::Schools).unwrap().len(),
            300
        );
        assert_eq!(store.stats().unwrap().observations, 300);
    }
}

#[test]
fn consolidation_merges_evidence_and_identities() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let mut a = school("Abbotsford High School");
    a.source_identities
        .push(SourceIdentity::new(SourceNamespace::MilesplitTeam, "52649"));
    a.evidence.push(Evidence::parsed(
        SourceRef::id("milesplit_teams"),
        "2026-09-20",
    ));
    let mut b = a.clone();
    b.co_op = true;
    b.city = Some("Abbotsford".into());
    b.source_identities.push(SourceIdentity::new(
        SourceNamespace::AssociationSchool {
            association: "wiaa".into(),
        },
        "1",
    ));
    store.append(Table::Schools, &a).unwrap();
    store.append(Table::Schools, &b).unwrap();
    let out = dir.path().join("out/schools.jsonl");
    let count = store
        .consolidate::<CanonicalSchool>(Table::Schools, &out)
        .unwrap()
        .rows;
    assert_eq!(count, 1);
    let merged: CanonicalSchool = serde_json::from_str(
        std::fs::read_to_string(&out)
            .unwrap()
            .lines()
            .next()
            .unwrap(),
    )
    .unwrap();
    assert!(merged.co_op);
    assert_eq!(merged.city.as_deref(), Some("Abbotsford"));
    assert_eq!(merged.source_identities.len(), 2);
}

#[test]
fn observations_survive_reopen_without_overwriting() {
    let dir = tempfile::tempdir().unwrap();
    {
        let store = Store::open(dir.path()).unwrap();
        let mut first = school("Abbotsford");
        first.evidence.push(Evidence::parsed(
            SourceRef::id("wiaa_schools"),
            "2026-09-19",
        ));
        store.append(Table::Schools, &first).unwrap();
    }
    {
        let store = Store::open(dir.path()).unwrap();
        let mut second = school("Abbotsford");
        second.evidence.push(Evidence::parsed(
            SourceRef::id("mshsl_schools"),
            "2026-09-20",
        ));
        store.append(Table::Schools, &second).unwrap();
        let rows = store.scan::<CanonicalSchool>(Table::Schools).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows.first().map(|row| row.evidence.len()), Some(2));
    }
}

#[test]
fn oversized_and_empty_ids_are_rejected_before_any_write() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let oversized = serde_json::json!({ "id": "s".repeat(MAX_ID_BYTES + 1) });
    assert!(store.append(Table::Schools, &oversized).is_err());
    let empty = serde_json::json!({ "id": "" });
    assert!(store.append(Table::Schools, &empty).is_err());
    assert_eq!(store.stats().unwrap().observations, 0);
}

#[test]
fn stats_count_observations_per_table() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store.append(Table::Schools, &school("Abbotsford")).unwrap();
    store
        .append_many(Table::Schools, &[school("Colby"), school("Medford")])
        .unwrap();
    let stats = store.stats().unwrap();
    let schools = stats
        .tables
        .iter()
        .find(|(table, _)| table == "schools")
        .map(|(_, count)| *count)
        .unwrap();
    assert_eq!(schools, 3);
    assert_eq!(stats.observations, 3);
    assert!(stats.store_bytes > 0);
    assert!(stats.store_bytes >= stats.bytes_on_disk);
}

#[test]
fn a_consumer_mailbox_is_published_as_personal_and_a_school_address_as_professional() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir).unwrap();
    let school = school("Abbotsford").id;
    let mut personal = CanonicalCoach::new(
        &school,
        "J. Riethmiller",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    personal.professional_email = Some("jriethmiller.ptc@gmail.com".to_string());
    let mut professional = CanonicalCoach::new(
        &school,
        "A. Bender",
        Some(Sport::CrossCountry),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    professional.personal_email = Some("abender@ofsd.k12.wi.us".to_string());
    store
        .append_many(Table::Coaches, &[personal, professional])
        .unwrap();

    let coaches = store.scan::<CanonicalCoach>(Table::Coaches).unwrap();
    let personal_row = coaches
        .iter()
        .find(|coach| coach.name == "J. Riethmiller")
        .unwrap();
    assert_eq!(
        personal_row.personal_email.as_deref(),
        Some("jriethmiller.ptc@gmail.com")
    );
    assert_eq!(personal_row.professional_email, None);
    let professional_row = coaches
        .iter()
        .find(|coach| coach.name == "A. Bender")
        .unwrap();
    assert_eq!(
        professional_row.professional_email.as_deref(),
        Some("abender@ofsd.k12.wi.us")
    );
    assert_eq!(professional_row.personal_email, None);
}

#[test]
fn the_row_ledger_is_a_count_the_keyspace_can_contradict() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let abbotsford = school("Abbotsford").id;
    store
        .append_many(Table::Schools, &[school("Abbotsford"), school("Colby")])
        .unwrap();
    assert_eq!(rows_held(&store, Table::Schools), 2);
    assert!(store.integrity().unwrap().ok);

    let key = super::keys::observation_key(Table::Schools, abbotsford.as_str(), 0);
    store.entities.remove(key).unwrap();

    assert_eq!(
        rows_held(&store, Table::Schools),
        2,
        "the ledger is the count the store kept, not one re-derived on demand"
    );
    assert_eq!(store.walk_table(Table::Schools).unwrap().rows, 1);
    let report = store.integrity().unwrap();
    assert!(!report.ok, "a lost row is what integrity exists to report");
    let schools = report
        .tables
        .iter()
        .find(|entry| entry.table == "schools")
        .expect("every table is checked");
    assert_eq!(schools.expected, 2, "the count the store holds");
    assert_eq!(schools.actual, 1, "the rows the walk finds");
}
