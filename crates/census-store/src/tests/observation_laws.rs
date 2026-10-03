use super::*;

#[test]
fn keys_with_a_zero_low_sequence_byte_still_reopen() -> TestResult {
    let dir = tempfile::tempdir()?;
    {
        let store = Store::open(dir.path())?;
        let rows: Vec<CanonicalSchool> = (0..300)
            .map(|index| school(&format!("School {index}")))
            .collect();
        store.append_many(Table::Schools, &rows)?;
    }
    {
        let store = Store::open(dir.path())?;
        check!(eq; store.scan::<CanonicalSchool>(Table::Schools)?.len(), 300);
        check!(eq; store.stats()?.observations, 300);
    }
    Ok(())
}

#[test]
fn consolidation_merges_evidence_and_identities() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
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
    store.append(Table::Schools, &a)?;
    store.append(Table::Schools, &b)?;
    let out = dir.path().join("out/schools.jsonl");
    let count = store
        .consolidate::<CanonicalSchool>(Table::Schools, &out)?
        .rows;
    check!(eq; count, 1);
    let raw = std::fs::read_to_string(&out)?;
    let merged: CanonicalSchool =
        serde_json::from_str(raw.lines().next().ok_or("consolidated school row")?)?;
    check!(merged.co_op);
    check!(eq; merged.city.as_deref(), Some("Abbotsford"));
    check!(eq; merged.source_identities.len(), 2);
    Ok(())
}

#[test]
fn observations_survive_reopen_without_overwriting() -> TestResult {
    let dir = tempfile::tempdir()?;
    {
        let store = Store::open(dir.path())?;
        let mut first = school("Abbotsford");
        first.evidence.push(Evidence::parsed(
            SourceRef::id("wiaa_schools"),
            "2026-09-19",
        ));
        store.append(Table::Schools, &first)?;
    }
    {
        let store = Store::open(dir.path())?;
        let mut second = school("Abbotsford");
        second.evidence.push(Evidence::parsed(
            SourceRef::id("mshsl_schools"),
            "2026-09-20",
        ));
        store.append(Table::Schools, &second)?;
        let rows = store.scan::<CanonicalSchool>(Table::Schools)?;
        check!(eq; rows.len(), 1);
        check!(eq; rows.first().map(|row| row.evidence.len()), Some(2));
    }
    Ok(())
}

#[test]
fn oversized_and_empty_ids_are_rejected_before_any_write() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let oversized = serde_json::json!({ "id": "s".repeat(MAX_ID_BYTES + 1) });
    check!(store.append(Table::Schools, &oversized).is_err());
    let empty = serde_json::json!({ "id": "" });
    check!(store.append(Table::Schools, &empty).is_err());
    check!(eq; store.stats()?.observations, 0);
    Ok(())
}

#[test]
fn stats_count_observations_per_table() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.append(Table::Schools, &school("Abbotsford"))?;
    store.append_many(Table::Schools, &[school("Colby"), school("Medford")])?;
    let stats = store.stats()?;
    let schools = stats
        .tables
        .iter()
        .find(|(table, _)| table == "schools")
        .map(|(_, count)| *count)
        .ok_or("school count")?;
    check!(eq; schools, 3);
    check!(eq; stats.observations, 3);
    check!(stats.store_bytes > 0);
    check!(stats.store_bytes >= stats.bytes_on_disk);
    Ok(())
}

#[test]
fn a_consumer_mailbox_is_published_as_personal_and_a_school_address_as_professional() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let store = Store::open(&dir)?;
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
    store.append_many(Table::Coaches, &[personal, professional])?;
    let coaches = store.scan::<CanonicalCoach>(Table::Coaches)?;
    let personal_row = coaches
        .iter()
        .find(|coach| coach.name == "J. Riethmiller")
        .ok_or("personal coach row")?;
    check!(eq; personal_row.personal_email.as_deref(), Some("jriethmiller.ptc@gmail.com"));
    check!(eq; personal_row.professional_email, None);
    let professional_row = coaches
        .iter()
        .find(|coach| coach.name == "A. Bender")
        .ok_or("professional coach row")?;
    check!(eq; professional_row.professional_email.as_deref(), Some("abender@ofsd.k12.wi.us"));
    check!(eq; professional_row.personal_email, None);
    Ok(())
}

#[test]
fn the_row_ledger_is_a_count_the_keyspace_can_contradict() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let abbotsford = school("Abbotsford").id;
    store.append_many(Table::Schools, &[school("Abbotsford"), school("Colby")])?;
    check!(eq; rows_held(&store, Table::Schools)?, 2);
    check!(store.integrity()?.ok);
    let key = super::keys::observation_key(Table::Schools, abbotsford.as_str(), 0);
    store.entities.remove(key)?;
    check!(eq; rows_held(&store, Table::Schools)?, 2, "the ledger is the count the store kept, not one re-derived on demand");
    check!(eq; store.walk_table(Table::Schools)?.rows, 1);
    let report = store.integrity()?;
    check!(!report.ok, "a lost row is what integrity exists to report");
    let schools = report
        .tables
        .iter()
        .find(|entry| entry.table == "schools")
        .ok_or("every table is checked")?;
    check!(eq; schools.expected, 2, "the count the store holds");
    check!(eq; schools.actual, 1, "the rows the walk finds");
    Ok(())
}

#[test]
fn escaped_ids_parse_the_same_as_plain_ids() -> TestResult {
    let row =
        br#"{"id":"milesplit_result_row:athletes:1299818:Run as \"5 Alive\":0","sequence":0}"#;
    let id = crate::keys::observation_id(row)?;
    check!(eq; id, "milesplit_result_row:athletes:1299818:Run as \"5 Alive\":0");
    let plain = crate::keys::observation_id(br#"{"id":"schools:abbotsford","sequence":0}"#)?;
    check!(eq; plain, "schools:abbotsford");
    Ok(())
}
