use super::*;
use sha2::{Digest, Sha256};

#[test]
fn source_image_accepts_each_physical_row_once_and_only_current_review_generation() -> TestResult {
    let (entities, meta) = source_fixture()?;
    let mut image = SourceImage::capture(&entities, &meta)?;
    check!(eq; image.expected.len(), 6);
    check!(eq; image.expected.get("review_cases").ok_or("review image")?.len(), 1);
    check!(eq; image.expected.get("athletes").ok_or("athlete image")?.len(), 2);
    let expected = image.expected.clone();
    for table in TABLES {
        image.observe_object(&table_object(table)?)?;
        image.observe_object(&page_object(
            table,
            expected.get(table).ok_or("expected table")?.iter(),
        )?)?;
    }
    image.finish(true)
}

#[test]
fn source_image_rejects_consistently_resealed_omission_from_each_of_six_tables() -> TestResult {
    for omitted in TABLES {
        let (entities, meta) = source_fixture()?;
        let mut image = SourceImage::capture(&entities, &meta)?;
        let expected = image.expected.clone();
        for table in TABLES {
            image.observe_object(&table_object(table)?)?;
            let rows = expected.get(table).ok_or("expected table")?;
            let retained = rows
                .iter()
                .skip(usize::from(table == omitted))
                .collect::<Vec<_>>();
            if !retained.is_empty() {
                image.observe_object(&page_object(table, retained.into_iter())?)?;
            }
        }
        let error = image
            .finish(true)
            .err()
            .ok_or("omitted source row was accepted")?;
        check!(error
            .to_string()
            .starts_with("complete physical source inventory;"));
    }
    Ok(())
}

#[test]
fn source_image_rejects_duplicate_physical_rows_across_resealed_pages_for_each_table() -> TestResult
{
    for duplicated in TABLES {
        let (entities, meta) = source_fixture()?;
        let mut image = SourceImage::capture(&entities, &meta)?;
        let expected = image
            .expected
            .get(duplicated)
            .ok_or("expected table")?
            .clone();
        let first = page_object(duplicated, expected.iter().take(1))?;
        image.observe_object(&first)?;
        let duplicate = page_object_at(duplicated, expected.iter().take(1), 1)?;
        check!(ne; first, duplicate);
        let error = image
            .observe_object(&duplicate)
            .err()
            .ok_or("duplicate source row was accepted")?;
        check!(eq; error.to_string(), "duplicate physical source key");
    }
    Ok(())
}

#[test]
fn source_image_requires_empty_table_manifests_and_refuses_duplicate_manifests() -> TestResult {
    let (_, meta) = source_fixture()?;
    for omitted in TABLES {
        let mut image = SourceImage::capture(&Rows::new(), &meta)?;
        for table in TABLES.into_iter().filter(|table| *table != omitted) {
            image.observe_object(&table_object(table)?)?;
        }
        let error = image
            .finish(true)
            .err()
            .ok_or("missing empty table was accepted")?;
        check!(error
            .to_string()
            .starts_with("complete six-table manifest inventory;"));
    }
    let mut image = SourceImage::capture(&Rows::new(), &meta)?;
    let table = table_object("athletes")?;
    image.observe_object(&table)?;
    let error = image
        .observe_object(&table)
        .err()
        .ok_or("duplicate manifest was accepted")?;
    check!(eq; error.to_string(), "duplicate source table manifest");
    Ok(())
}

#[test]
fn source_image_rejects_resealed_original_value_changes_in_each_table() -> TestResult {
    for changed in TABLES {
        let (entities, meta) = source_fixture()?;
        let mut image = SourceImage::capture(&entities, &meta)?;
        let mut rows = image.expected.get(changed).ok_or("expected table")?.clone();
        let (_, value) = rows.first_key_value().ok_or("expected source row")?;
        let mut replacement = value.clone();
        replacement.push(b' ');
        let key = rows.keys().next().ok_or("source key")?.clone();
        check!(rows.insert(key, replacement).is_some());
        let altered = page_object(changed, rows.iter())?;
        let error = image
            .observe_object(&altered)
            .err()
            .ok_or("changed source value was accepted")?;
        check!(error.to_string().contains("left=Some("));
    }
    Ok(())
}

#[test]
fn source_image_rejects_stale_review_generation_substitution() -> TestResult {
    let (entities, meta) = source_fixture()?;
    let mut image = SourceImage::capture(&entities, &meta)?;
    let old = source_prefix("review_cases", 6);
    let rows = page_object(
        "review_cases",
        entities.iter().filter(|(key, _)| key.starts_with(&old)),
    )?;
    let error = image
        .observe_object(&rows)
        .err()
        .ok_or("stale review generation was accepted")?;
    check!(error.to_string().contains("left=None right=Some("));
    Ok(())
}

fn source_fixture() -> TestResult<(Rows, Rows)> {
    let mut entities = Rows::new();
    for table in TABLES {
        let key = source_key(table, 7, 0);
        check!(entities
            .insert(
                key,
                serde_json::to_vec(&json!({"source":table,"revision":0}))?
            )
            .is_none());
    }
    check!(entities
        .insert(
            source_key("athletes", 7, 1),
            b"{\"source\":\"athletes\",\"revision\":1}".to_vec()
        )
        .is_none());
    check!(entities
        .insert(
            source_key("review_cases", 6, 0),
            b"{\"source\":\"old-review\"}".to_vec()
        )
        .is_none());
    check!(entities
        .insert(
            b"schools\0unrelated\0\0\0\0\0\0\0\0\0".to_vec(),
            b"{}".to_vec()
        )
        .is_none());
    let mut meta = Rows::new();
    check!(meta
        .insert(b"generation:derived:current".to_vec(), b"7".to_vec())
        .is_none());
    Ok((entities, meta))
}

fn source_key(table: &str, generation: u64, sequence: u64) -> Vec<u8> {
    let mut key = source_prefix(table, generation);
    key.extend_from_slice(b"same-logical-id");
    if table != "review_cases" {
        key.push(0);
        key.extend_from_slice(&sequence.to_be_bytes());
    }
    key
}

fn table_object(table: &str) -> TestResult<Rows> {
    object(&[
        json!({"key":"header","at":"","payload":{"kind":"table_header","value":{
            "schema":"public-identity-authority-table/v2","binding":{"table":table},
        }}}),
    ])
}

fn page_object<'a>(
    table: &str,
    rows: impl Iterator<Item = (&'a Vec<u8>, &'a Vec<u8>)>,
) -> TestResult<Rows> {
    page_object_at(table, rows, 0)
}

fn page_object_at<'a>(
    table: &str,
    rows: impl Iterator<Item = (&'a Vec<u8>, &'a Vec<u8>)>,
    page: u32,
) -> TestResult<Rows> {
    let mut records = Vec::new();
    let mut count = 0usize;
    let mut raw_bytes = 0usize;
    for (key, value) in rows {
        check!(count < 512);
        let ordinal = format!("row:{count:020}");
        records.push(
            json!({"key":ordinal,"at":"","payload":{"kind":"raw_row","value":{
                "physical_key_base64":BASE64_STANDARD.encode(key),"value_bytes":value.len(),
                "value_sha256":digest(value),"parts":1,
            }}}),
        );
        records.push(json!({"key":format!("{ordinal}:part:{:020}",0),"at":"","payload":{"kind":"raw_part","value":{
            "bytes_base64":BASE64_STANDARD.encode(value),
        }}}));
        count = count.checked_add(1).ok_or("fixture source count")?;
        raw_bytes = raw_bytes
            .checked_add(key.len())
            .and_then(|sum| sum.checked_add(value.len()))
            .ok_or("fixture raw bytes")?;
    }
    check!(count > 0);
    records.insert(
        0,
        json!({"key":"header","at":"","payload":{"kind":"raw_page_header","value":{
            "schema":"public-identity-authority-raw-page/v2","binding":digest(table.as_bytes()),
            "table":table,"page":page,"first_ordinal":page,"rows":count,"raw_bytes":raw_bytes,
        }}}),
    );
    object(&records)
}

fn object(records: &[Value]) -> TestResult<Rows> {
    let mut bytes = Vec::new();
    let mut entries = Vec::new();
    for (sequence, record) in records.iter().enumerate() {
        let mut raw = serde_json::to_vec(record)?;
        raw.push(b'\n');
        entries.push(json!({"sequence":sequence,"offset":bytes.len(),"length":raw.len()}));
        bytes.extend_from_slice(&raw);
    }
    check!(bytes.len() <= 16_384);
    let root = digest(&bytes);
    let mut rows = Rows::new();
    let chunk = serde_json::to_vec(
        &json!({"type":"journal-chunk","version":2,"object":root,"index":0,
        "length":bytes.len(),"digest":root,"data":BASE64_STANDARD.encode(&bytes)}),
    )?;
    let page = serde_json::to_vec(
        &json!({"type":"journal-index","version":2,"object":root,"page":0,"entries":entries}),
    )?;
    let manifest = serde_json::to_vec(
        &json!({"type":"journal-manifest","version":2,"object":root,"codec":"jsonl",
        "total_bytes":bytes.len(),"chunks":1,"records":records.len(),"index_pages":1,"chunk_digests":[root]}),
    )?;
    check!(rows
        .insert(super::super::key("chunk", &root, Some(0))?, chunk.clone())
        .is_none());
    check!(rows
        .insert(super::super::key("index", &root, Some(0))?, page.clone())
        .is_none());
    check!(rows
        .insert(
            super::super::key("manifest", &root, None)?,
            manifest.clone()
        )
        .is_none());
    let physical_bytes = rows.iter().try_fold(0usize, |sum, (key, value)| {
        sum.checked_add(key.len())
            .and_then(|sum| sum.checked_add(value.len()))
            .ok_or("fixture physical bytes")
    })?;
    let descriptor = json!({
        "object":{"root":root,"bytes":bytes.len(),"records":records.len(),"physical_bytes":physical_bytes},
        "physical":{"root":root,"full_bytes":physical_bytes,"jsonl_bytes":bytes.len(),"records":records.len(),
            "chunks":[fingerprint(&chunk)],"pages":[fingerprint(&page)],"manifest":fingerprint(&manifest)},
    });
    super::super::physical::verify(&rows, &descriptor, true)
}

fn fingerprint(raw: &[u8]) -> Value {
    json!({"length":raw.len(),"digest":Sha256::digest(raw).to_vec()})
}
