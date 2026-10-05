use super::*;
use census_domain::model::Evidence;

fn linked_school() -> TestResult<CanonicalSchool> {
    let (mut school, _) = CanonicalSchool::new(
        UsJurisdiction::Tennessee,
        "Page High School",
        census_domain::model::normalize_name("Page High School"),
    );
    school.city = Some("Franklin".into());
    school.source_identities.push(
        SourceIdentity::new(SourceNamespace::association_school("tssaa"), "157")
            .with_url("https://portal.tssaa.org/common/directory/?id=157".to_string()),
    );
    let mut evidence = Evidence::parsed(
        SourceRef::new(
            "tssaa",
            Some("https://portal.tssaa.org/common/directory/?id=157".into()),
        ),
        "2026-09-22",
    );
    evidence.note = Some(
        "association:tssaa lane crates/census-crawl/tests/fixtures/tssaa/directory_id157.html sha256=2f920f31115b5e96e2a1155747022e96baf54d41069c52a16701a3fb814a6453 generation 3b177e8b31e016783339849762ef0467c652abc0c58e2095aecd3375a5becd0d".into(),
    );
    school.evidence.push(evidence);
    Ok(school)
}

fn exported_row(data: &std::path::Path) -> TestResult<(::csv::StringRecord, ::csv::StringRecord)> {
    let mut reader = ::csv::Reader::from_path(data.join("canonical-schools.csv"))?;
    let headers = reader.headers()?.clone();
    let record = reader.records().next().ok_or("missing exported row")??;
    Ok((headers, record))
}

fn column(headers: &::csv::StringRecord, name: &str) -> TestResult<usize> {
    headers
        .iter()
        .position(|field| field == name)
        .ok_or_else(|| format!("missing link column {name}").into())
}

#[test]
fn an_association_link_publishes_its_owner_and_lane_provenance() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let school = linked_school()?;
    store.append(Table::Schools, &school)?;
    store.flush()?;
    let data = dir.path().join("data");
    run_export_data(
        &store,
        &ExportDataArgs {
            data: data.clone(),
            school_year: Some(2026),
        },
    )?;
    let (headers, record) = exported_row(&data)?;
    for (name, expected) in [
        ("link_school_id", school.id.to_string()),
        ("link_owner_namespace", "association_school:tssaa".into()),
        ("link_owner_id", "157".into()),
        (
            "link_owner_url",
            "https://portal.tssaa.org/common/directory/?id=157".into(),
        ),
        ("link_source", "tssaa".into()),
        (
            "link_source_url",
            "https://portal.tssaa.org/common/directory/?id=157".into(),
        ),
        ("link_observed_date", "2026-09-22".into()),
        (
            "link_note",
            school.evidence[0]
                .note
                .clone()
                .ok_or("missing fixture note")?,
        ),
    ] {
        let index = column(&headers, name)?;
        check!(eq;
            record.get(index),
            Some(expected.as_str()),
            "canonical-schools.csv {name}"
        );
    }
    Ok(())
}

#[test]
fn a_school_without_an_association_link_publishes_empty_link_columns() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let (school, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Unlinked High",
        census_domain::model::normalize_name("Unlinked High"),
    );
    store.append(Table::Schools, &school)?;
    store.flush()?;
    let data = dir.path().join("data");
    run_export_data(
        &store,
        &ExportDataArgs {
            data: data.clone(),
            school_year: Some(2026),
        },
    )?;
    let (headers, record) = exported_row(&data)?;
    for name in [
        "link_school_id",
        "link_owner_namespace",
        "link_owner_id",
        "link_owner_url",
        "link_source",
        "link_source_url",
        "link_observed_date",
        "link_note",
    ] {
        let index = column(&headers, name)?;
        check!(eq; record.get(index), Some(""), "canonical-schools.csv {name}");
    }
    Ok(())
}
