use super::*;
use census_domain::model::*;
use census_domain::UsJurisdiction;

fn school(name: &str) -> CanonicalSchool {
    CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalize_name(name)).0
}

#[test]
fn keys_with_a_zero_low_sequence_byte_still_reopen() {
    // A sequence is stored big-endian in the key tail, so every 256th key ends in 0x00 — the
    // same byte that separates the id from the sequence. Parsing that separator by search made
    // `Store::open` fail forever once such a key existed.
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
fn journal_roundtrips_resume_keys() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store
        .journal_done(
            "milesplit_rosters",
            "wi:52649",
            &serde_json::json!({"athletes": 59}),
        )
        .unwrap();
    store
        .journal_done(
            "milesplit_rosters",
            "wi:26848",
            &serde_json::json!({"athletes": 0}),
        )
        .unwrap();
    let keys = store.journal_keys("milesplit_rosters").unwrap();
    assert!(keys.contains("wi:52649"));
    assert_eq!(keys.len(), 2);
    assert_eq!(
        store.journal_payloads("milesplit_rosters").unwrap().len(),
        2
    );
    // A second phase must not leak into the first phase's resume set.
    store
        .journal_done("other_phase", "wi:1", &serde_json::json!({}))
        .unwrap();
    assert_eq!(store.journal_keys("milesplit_rosters").unwrap().len(), 2);
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
        // Reopening must resume the sequence, not restart it: a restarted sequence would
        // overwrite the first observation and silently drop its evidence.
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
fn legacy_journals_are_imported_once() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    std::fs::create_dir_all(root.join("entities")).unwrap();
    std::fs::create_dir_all(root.join("journal")).unwrap();
    let row = school("Abbotsford");
    std::fs::write(
        root.join("entities/schools.jsonl"),
        format!("{}\n", serde_json::to_string(&row).unwrap()),
    )
    .unwrap();
    std::fs::write(
        root.join("journal/milesplit_rosters.jsonl"),
        "{\"key\":\"wi:1\",\"at\":\"2026-09-20\",\"payload\":{\"athletes\":5}}\n",
    )
    .unwrap();

    {
        let store = Store::open(&root).unwrap();
        assert_eq!(
            store.scan::<CanonicalSchool>(Table::Schools).unwrap().len(),
            1
        );
        assert!(store
            .journal_keys("milesplit_rosters")
            .unwrap()
            .contains("wi:1"));
    }
    // The importer is idempotent: the second open sees the marker and adds nothing.
    let store = Store::open(&root).unwrap();
    assert_eq!(
        store.scan::<CanonicalSchool>(Table::Schools).unwrap().len(),
        1
    );
    assert_eq!(store.journal_keys("milesplit_rosters").unwrap().len(), 1);
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
}

#[test]
fn a_consumer_mailbox_never_survives_a_read_but_a_school_address_does() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let school = school("Abbotsford").id;
    let mut withheld = CanonicalCoach::new(
        &school,
        "J. Riethmiller",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    withheld.professional_email = Some("jriethmiller.ptc@gmail.com".to_string());
    let mut published = CanonicalCoach::new(
        &school,
        "A. Bender",
        Some(Sport::CrossCountry),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    published.professional_email = Some("abender@ofsd.k12.wi.us".to_string());
    store
        .append_many(Table::Coaches, &[withheld, published])
        .unwrap();

    let coaches = store.scan::<CanonicalCoach>(Table::Coaches).unwrap();
    let withheld_row = coaches
        .iter()
        .find(|coach| coach.name == "J. Riethmiller")
        .unwrap();
    assert_eq!(withheld_row.professional_email, None);
    assert!(withheld_row.email_withheld);
    let published_row = coaches
        .iter()
        .find(|coach| coach.name == "A. Bender")
        .unwrap();
    assert_eq!(
        published_row.professional_email.as_deref(),
        Some("abender@ofsd.k12.wi.us")
    );
    assert!(!published_row.email_withheld);
}
