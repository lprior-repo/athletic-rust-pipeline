use super::*;

#[test]
fn duplicate_unknown_fields_and_noncanonical_jsonl_refuse() -> TestResult {
    let (binding, page) = page()?;
    let bytes = page.encode_jsonl()?;
    for (old, new) in [
        (
            "\"key\":\"header\"",
            "\"key\":\"header\",\"key\":\"header\"",
        ),
        ("\"at\":\"\"", "\"at\":\"\",\"at\":\"\""),
        (
            "\"kind\":\"raw_page_header\"",
            "\"kind\":\"raw_page_header\",\"kind\":\"raw_page_header\"",
        ),
        ("\"rows\":1", "\"rows\":1,\"rows\":1"),
        ("\"value_bytes\":", "\"unknown\":true,\"value_bytes\":"),
        ("\"bytes_base64\":", "\"extra\":1,\"bytes_base64\":"),
        ("\"key\":", "\"key\" :"),
        ("\"at\":\"\"", "\"at\":\"now\""),
        ("\"rows\":1", "\"rows\":1.0"),
        ("/v2", "/v3"),
        ("raw_page_header", "table_header"),
        (
            "row:00000000000000000000:part:00000000000000000000",
            "row:00000000000000000000:part:00000000000000000001",
        ),
    ] {
        let changed = replace(&bytes, old, new)?;
        check!(
            RawPageV2::decode(&changed, &object(&changed)?, &binding).is_err(),
            "mutation {old}"
        );
    }
    let no_lf = bytes
        .get(..bytes.len().checked_sub(1).ok_or("length")?)
        .ok_or("tail")?;
    let reference = ObjectRefV2::new(
        DigestV2::of_bytes(no_lf),
        u32::try_from(no_lf.len())?,
        3,
        u64::try_from(no_lf.len())?,
    )?;
    check!(RawPageV2::decode(no_lf, &reference, &binding).is_err());
    let crlf = replace(&bytes, "\n", "\r\n")?;
    check!(RawPageV2::decode(&crlf, &object(&crlf)?, &binding).is_err());
    Ok(())
}

#[test]
fn missing_extra_reordered_and_malformed_fragments_refuse() -> TestResult {
    let binding = capture()?.table_binding(AuthorityTableV2::IdentityVerdicts)?;
    let page = RawPageV2::new(
        &binding,
        RawPagePositionV2::new(0, 0)?,
        vec![row(&binding, "a", MAX_RAW_VALUE_BYTES)?],
    )?;
    let bytes = page.encode_jsonl()?;
    let lines: Vec<&[u8]> = bytes.split_inclusive(|byte| *byte == b'\n').collect();
    let header = lines.first().ok_or("header")?;
    let row = lines.get(1).ok_or("row")?;
    let first = lines.get(2).ok_or("first part")?;
    let last = lines.get(3).ok_or("last part")?;
    for pieces in [
        vec![*header, *row, *first],
        vec![*header, *row, *last, *first],
        vec![*header, *row, *first, *first, *last],
    ] {
        let changed = pieces.concat();
        check!(RawPageV2::decode(&changed, &object(&changed)?, &binding).is_err());
    }
    for (old, new) in [
        ("\"parts\":2", "\"parts\":1"),
        ("\"parts\":2", "\"parts\":3"),
        ("\"value_bytes\":1048576", "\"value_bytes\":1048577"),
        ("\"bytes_base64\":\"", "\"bytes_base64\":\"A"),
    ] {
        let changed = replace(&bytes, old, new)?;
        check!(RawPageV2::decode(&changed, &object(&changed)?, &binding).is_err());
    }
    Ok(())
}

#[test]
fn seal_rejects_duplicate_binding_policy_inventory_and_table_fields() -> TestResult {
    let (binding, seal) = seal()?;
    let bytes = seal.encode_jsonl()?;
    for (old, new) in [
        (
            "\"store_schema\":1",
            "\"store_schema\":1,\"store_schema\":1",
        ),
        (
            "\"kind\":\"unbound\"",
            "\"kind\":\"unbound\",\"kind\":\"unbound\"",
        ),
        (
            "\"evidence_generation\":9",
            "\"evidence_generation\":9,\"evidence_generation\":9",
        ),
        ("\"authority_policy\":1", "\"authority_policy\":2"),
        (
            "\"table\":\"athletes\"",
            "\"table\":\"athletes\",\"table\":\"athletes\"",
        ),
        ("\"role\":\"authority\"", "\"role\":\"affected\""),
        ("\"pages\":0", "\"pages\":0,\"pages\":0"),
        ("\"records\":1", "\"records\":1,\"records\":1"),
        ("full-section-v1", "raw-row-v2"),
        ("/v2", "/v1"),
    ] {
        let changed = replace(&bytes, old, new)?;
        check!(
            RawGenerationSealV2::decode(&changed, &object(&changed)?, &binding).is_err(),
            "mutation {old}"
        );
    }
    let changed_binding = CaptureBindingV2::new(
        census_domain::model::StoreIdentity::parse(&"2".repeat(64))?,
        FormatBindingV2::new(1, 1, 1)?,
        RunBindingV2::Unbound,
        CapturePositionV2::new(9, 7, 12, [3, 4, 5, 6, 7, 8]),
    )?;
    check!(RawGenerationSealV2::decode(&bytes, &object(&bytes)?, &changed_binding).is_err());
    Ok(())
}

#[test]
fn wrong_root_and_unexpected_v1_root_never_fall_back() -> TestResult {
    let (binding, page) = page()?;
    let bytes = page.encode_jsonl()?;
    let actual = object(&bytes)?;
    let forged = ObjectRefV2::new(
        DigestV2::parse(&"0".repeat(64))?,
        actual.bytes(),
        actual.records(),
        actual.physical_bytes(),
    )?;
    check!(RawPageV2::decode(&bytes, &forged, &binding).is_err());
    let legacy = format!("{{\"key\":\"header\",\"at\":\"\",\"payload\":{{\"kind\":\"header\",\"value\":{{\"version\":1,\"authority_policy\":1,\"enrichment_policy\":1,\"lineage\":\"{}\",\"counts\":[0,0,0,0]}}}}}}\n", "1".repeat(64));
    let bytes = legacy.as_bytes();
    let identity = census_domain::model::StoreIdentity::parse(&"1".repeat(64))?;
    let retained = crate::public_identity::context::from_bytes(
        bytes,
        DigestV2::of_bytes(bytes).as_str(),
        &identity,
    )?;
    check!(retained.athletes.is_empty());
    check!(RawPageV2::decode(bytes, &object(bytes)?, &binding).is_err());
    check!(RawGenerationSealV2::decode(bytes, &object(bytes)?, &capture()?).is_err());
    Ok(())
}

#[test]
fn declared_raw_bytes_refuse_before_fragment_decode_or_value_allocation() -> TestResult {
    let (binding, page) = page()?;
    let bytes = page.encode_jsonl()?;
    let mut input = records::Records::new(&bytes, &object(&bytes)?, limits::ObjectLimits::PAGE)?;
    let header: wire::RawPageHeader = input.next("header", "raw_page_header")?;
    let mut row: wire::RawRow = input.next("row:00000000000000000000", "raw_row")?;
    row.value_bytes = u32::try_from(MAX_RAW_VALUE_BYTES)?;
    row.parts = 2;
    let mut changed = records::encode("header", "raw_page_header", &header)?;
    changed.extend(records::encode(
        "row:00000000000000000000",
        "raw_row",
        &row,
    )?);
    changed.extend_from_slice(b"invalid fragment\n");
    check!(
        matches!(RawPageV2::decode(&changed, &object(&changed)?, &binding),
        Err(crate::StoreError::Refused { detail }) if detail == "raw public identity generation V2: raw page allocation exceeds declared byte total")
    );
    Ok(())
}

#[test]
fn historical_affected_v1_bytes_decode_only_in_the_original_branch() -> TestResult {
    let legacy = format!("{{\"key\":\"header\",\"at\":\"\",\"payload\":{{\"kind\":\"header\",\"value\":{{\"schema\":\"local-identity-affected-records/v1\",\"store\":\"{}\",\"authority_context\":\"{}\",\"evidence_generation\":1,\"derived_generation\":0,\"snapshot_sequence\":1,\"counts\":[0,0]}}}}}}\n", "1".repeat(64), "2".repeat(64));
    let bytes = legacy.as_bytes();
    crate::public_identity::local_adjudication::affected::from_bytes(
        bytes,
        DigestV2::of_bytes(bytes).as_str(),
    )?;
    check!(RawGenerationSealV2::decode(bytes, &object(bytes)?, &capture()?).is_err());
    let binding = capture()?.table_binding(AuthorityTableV2::Performances)?;
    check!(RawPageV2::decode(bytes, &object(bytes)?, &binding).is_err());
    Ok(())
}

#[test]
fn duplicate_nested_bound_run_fields_refuse_without_value_roundtrip() -> TestResult {
    let (binding, empty) = seal()?;
    let manifest = census_domain::model::RunManifest {
        store_identity: binding.0.store.as_str().to_owned(),
        run: census_domain::model::CensusRun::new(
            census_domain::model::SchoolYear::new(2026).ok_or("season")?,
            1,
        )
        .ok_or("run")?,
        cohort: census_domain::model::GradYear::CO2027,
        jurisdictions: vec![census_domain::UsJurisdiction::Michigan],
    };
    let bound = CaptureBindingV2::new(
        binding.0.store.clone(),
        FormatBindingV2::new(1, 1, 1)?,
        RunBindingV2::Bound(manifest),
        CapturePositionV2::new(9, 7, 12, [3, 4, 5, 6, 7, 8]),
    )?;
    check!(RawGenerationSealV2::new(bound.clone(), empty.tables().clone()).is_err());
    let tables = AuthorityTableV2::ALL
        .into_iter()
        .map(|table| {
            let manifest = RawTableManifestV2::new(&bound.table_binding(table)?, Vec::new())?;
            let bytes = manifest.encode_jsonl()?;
            manifest.reference(object(&bytes)?).map_err(Into::into)
        })
        .collect::<TestResult<Vec<_>>>()?;
    let seal = RawGenerationSealV2::new(bound.clone(), tables.try_into().map_err(|_| "tables")?)?;
    let bytes = seal.encode_jsonl()?;
    for (old, new) in [
        ("\"manifest\":{", "\"manifest\":{},\"manifest\":{"),
        ("\"cohort\":2027", "\"cohort\":2027,\"cohort\":2027"),
        ("\"season\":2026", "\"season\":2026,\"season\":2026"),
        ("\"revision\":1", "\"revision\":1,\"revision\":1"),
    ] {
        let changed = replace(&bytes, old, new)?;
        check!(RawGenerationSealV2::decode(&changed, &object(&changed)?, &bound).is_err());
    }
    Ok(())
}

#[test]
fn duplicate_payload_value_and_raw_part_fields_refuse() -> TestResult {
    let (binding, page) = page()?;
    let bytes = page.encode_jsonl()?;
    for (old, new) in [
        ("\"payload\":{", "\"payload\":{},\"payload\":{"),
        ("\"value\":{", "\"value\":{},\"value\":{"),
        ("\"bytes_base64\":\"", "\"bytes_base64\":\"\",\"bytes_base64\":\""),
        ("\"value_sha256\":\"", "\"value_sha256\":\"0000000000000000000000000000000000000000000000000000000000000000\",\"value_sha256\":\""),
    ] {
        let changed = replace(&bytes, old, new)?;
        check!(RawPageV2::decode(&changed, &object(&changed)?, &binding).is_err());
    }
    Ok(())
}
