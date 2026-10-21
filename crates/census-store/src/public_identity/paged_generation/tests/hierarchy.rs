use super::*;
use census_domain::model::StoreIdentity;

#[test]
fn raw_page_block_table_and_seal_roundtrip_with_nonself_referential_totals() -> TestResult {
    let (binding, page) = page()?;
    let bytes = page.encode_jsonl()?;
    let page_ref = page.reference(object(&bytes)?)?;
    let block = RawPageBlockV2::new(&binding, 0, vec![page_ref.clone()])?;
    let bytes = block.encode_jsonl()?;
    let block_object = object(&bytes)?;
    check!(eq;
        RawPageBlockV2::decode(&bytes, &block_object, &binding)?,
        block
    );
    let block_ref = block.reference(block_object)?;
    check!(eq;block_ref.inventory().object_count(), 2);
    check!(eq;block_ref.inventory().pages(), 1);
    check!(eq;block_ref.inventory().rows(), 1);
    page_ref.check_page(&page)?;
    block_ref.check_block(&block)?;
    let table = RawTableManifestV2::new(&binding, vec![block_ref])?;
    let bytes = table.encode_jsonl()?;
    let table_object = object(&bytes)?;
    check!(eq;
        RawTableManifestV2::decode(&bytes, &table_object, &binding)?,
        table
    );
    let table_ref = table.reference(table_object)?;
    table_ref.check_table(&table)?;
    check!(eq;table_ref.inventory().object_count(), 3);
    let (capture, empty) = seal()?;
    let mut tables = empty.tables().clone();
    *tables.get_mut(3).ok_or("verdict table")? = table_ref;
    let seal = RawGenerationSealV2::new(capture.clone(), tables)?;
    check!(eq;seal.declared_inventory().object_count(), 8);
    let bytes = seal.encode_jsonl()?;
    let reference = object(&bytes)?;
    check!(eq;seal.inventory_including_seal(&reference)?.object_count(), 9);
    check!(eq;
        RawGenerationSealV2::decode(&bytes, &reference, &capture)?,
        seal
    );
    Ok(())
}

#[test]
fn six_empty_tables_are_explicit_and_fixed_order() -> TestResult {
    let (binding, seal) = seal()?;
    let bytes = seal.encode_jsonl()?;
    let decoded = RawGenerationSealV2::decode(&bytes, &object(&bytes)?, &binding)?;
    check!(eq;decoded.tables().len(), 6);
    check!(eq;decoded.declared_inventory().pages(), 0);
    check!(eq;decoded.declared_inventory().rows(), 0);
    check!(eq;decoded.declared_inventory().object_count(), 6);
    let mut swapped = decoded.tables().clone();
    swapped.swap(0, 1);
    check!(RawGenerationSealV2::new(binding.clone(), swapped).is_err());
    let mut value: serde_json::Value = serde_json::from_slice(&bytes)?;
    let tables = value
        .pointer_mut("/payload/value/tables")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("tables")?;
    tables.pop().ok_or("sixth table")?;
    let mut missing = serde_json::to_vec(&value)?;
    missing.push(b'\n');
    check!(RawGenerationSealV2::decode(&missing, &object(&missing)?, &binding).is_err());
    Ok(())
}

fn synthetic_page(page: u32) -> TestResult<RawPageReferenceV2> {
    Ok(RawPageReferenceV2(wire::PageRef {
        page,
        first_ordinal: u64::from(page),
        rows: 1,
        raw_bytes: 300,
        object: ObjectRefV2::new(DigestV2::of_bytes(&page.to_be_bytes()), 500, 3, 1000)?.0,
    }))
}

#[test]
fn exact_block_and_table_fanout_and_one_over() -> TestResult {
    let binding = capture()?.table_binding(AuthorityTableV2::IdentityVerdicts)?;
    let blocks = (0..MAX_REFERENCE_FANOUT)
        .map(|index| {
            let first = u32::from(index) * u32::from(MAX_REFERENCE_FANOUT);
            let pages = (0..MAX_REFERENCE_FANOUT)
                .map(|offset| synthetic_page(first + u32::from(offset)))
                .collect::<TestResult<Vec<_>>>()?;
            let block = RawPageBlockV2::new(&binding, index, pages)?;
            let bytes = block.encode_jsonl()?;
            check!(bytes.len() <= usize::try_from(MAX_DIRECTORY_JSONL_BYTES)?);
            block.reference(object(&bytes)?).map_err(Into::into)
        })
        .collect::<TestResult<Vec<_>>>()?;
    let table = RawTableManifestV2::new(&binding, blocks.clone())?;
    let bytes = table.encode_jsonl()?;
    check!(eq;
        RawTableManifestV2::decode(&bytes, &object(&bytes)?, &binding)?,
        table
    );
    check!(eq;
        table.reference(object(&bytes)?)?.inventory().pages(),
        MAX_TABLE_PAGES
    );
    let mut too_many = blocks;
    too_many.push(too_many.first().cloned().ok_or("block")?);
    check!(RawTableManifestV2::new(&binding, too_many).is_err());
    let pages = (0..=MAX_REFERENCE_FANOUT)
        .map(|page| synthetic_page(u32::from(page)))
        .collect::<TestResult<Vec<_>>>()?;
    check!(RawPageBlockV2::new(&binding, 0, pages).is_err());
    check!(RawPageBlockV2::new(&binding, 0, Vec::new()).is_err());
    Ok(())
}

#[test]
fn omitted_duplicate_reordered_and_wrong_inventory_refs_refuse() -> TestResult {
    let binding = capture()?.table_binding(AuthorityTableV2::IdentityVerdicts)?;
    let first = synthetic_page(0)?;
    let second = synthetic_page(1)?;
    check!(RawPageBlockV2::new(&binding, 0, vec![second.clone()]).is_err());
    check!(RawPageBlockV2::new(&binding, 0, vec![first.clone(), first.clone()]).is_err());
    check!(RawPageBlockV2::new(&binding, 0, vec![second.clone(), first.clone()]).is_err());
    let block = RawPageBlockV2::new(&binding, 0, vec![first, second])?;
    let bytes = block.encode_jsonl()?;
    for (old, new) in [
        ("\"rows\":2", "\"rows\":3"),
        ("\"pages\":2", "\"pages\":3"),
        ("\"pages\":2", "\"pages\":257"),
        ("\"page\":1", "\"page\":2"),
        ("\"first_ordinal\":1", "\"first_ordinal\":0"),
    ] {
        let changed = replace(&bytes, old, new)?;
        check!(RawPageBlockV2::decode(&changed, &object(&changed)?, &binding).is_err());
    }
    let reference = block.reference(object(&bytes)?)?;
    let table = RawTableManifestV2::new(&binding, vec![reference])?;
    let bytes = table.encode_jsonl()?;
    for (old, new) in [
        ("exhausted", "partial"),
        ("\"blocks\":1", "\"blocks\":257"),
        ("\"object_count\":3", "\"object_count\":4"),
        ("\"revision\":6", "\"revision\":7"),
        ("\"kind\":\"derived_map\"", "\"kind\":\"observation_log\""),
    ] {
        let changed = replace(&bytes, old, new)?;
        check!(RawTableManifestV2::decode(&changed, &object(&changed)?, &binding).is_err());
    }
    Ok(())
}

#[test]
fn table_binding_reuses_only_unchanged_lineage_revision_and_layout() -> TestResult {
    let old = capture()?;
    let newer = CaptureBindingV2::new(
        StoreIdentity::parse(&"1".repeat(64))?,
        FormatBindingV2::new(1, 1, 1)?,
        RunBindingV2::Unbound,
        CapturePositionV2::new(10, 7, 1000, [3, 4, 5, 6, 7, 8]),
    )?;
    check!(eq;
        old.table_binding(AuthorityTableV2::Athletes)?,
        newer.table_binding(AuthorityTableV2::Athletes)?
    );
    let changed = CaptureBindingV2::new(
        StoreIdentity::parse(&"1".repeat(64))?,
        FormatBindingV2::new(1, 1, 1)?,
        RunBindingV2::Unbound,
        CapturePositionV2::new(10, 8, 1000, [3, 4, 6, 7, 7, 8]),
    )?;
    check!(ne;
        old.table_binding(AuthorityTableV2::ReviewCases)?,
        changed.table_binding(AuthorityTableV2::ReviewCases)?
    );
    check!(ne;
        old.table_binding(AuthorityTableV2::IdentityVerdicts)?,
        changed.table_binding(AuthorityTableV2::IdentityVerdicts)?
    );
    let (_, page) = page()?;
    let bytes = page.encode_jsonl()?;
    check!(RawPageV2::decode(
        &bytes,
        &object(&bytes)?,
        &changed.table_binding(AuthorityTableV2::IdentityVerdicts)?
    )
    .is_err());
    Ok(())
}

#[test]
fn page_boundaries_enforce_greedy_full_pages_and_physical_versions() -> TestResult {
    let binding = capture()?.table_binding(AuthorityTableV2::Athletes)?;
    let athlete = crate::public_identity_tests::support::subject("paged", "11")?;
    let value = serde_json::to_vec(&athlete)?;
    let row = |sequence| {
        RawPhysicalRowV2::new(
            &binding,
            &crate::keys::observation_key(crate::Table::Athletes, athlete.id.as_str(), sequence),
            &value,
        )
    };
    let rows = (0..512).map(row).collect::<crate::StoreResult<Vec<_>>>()?;
    let first = RawPageV2::new(&binding, RawPagePositionV2::new(0, 0)?, rows)?;
    let next = RawPageV2::new(&binding, RawPagePositionV2::new(1, 512)?, vec![row(512)?])?;
    first.check_successor(&next)?;
    check!(eq;first.rows().len(), 512);
    check!(eq;next.rows().len(), 1);
    let sparse = RawPageV2::new(&binding, RawPagePositionV2::new(0, 0)?, vec![row(0)?])?;
    let shifted = RawPageV2::new(&binding, RawPagePositionV2::new(1, 1)?, vec![row(1)?])?;
    check!(sparse.check_successor(&shifted).is_err());
    let duplicate = RawPageV2::new(&binding, RawPagePositionV2::new(1, 512)?, vec![row(511)?])?;
    check!(first.check_successor(&duplicate).is_err());
    Ok(())
}

#[test]
fn impossible_zero_raw_block_inventory_refuses_at_table_boundary() -> TestResult {
    let binding = capture()?.table_binding(AuthorityTableV2::IdentityVerdicts)?;
    let block = RawPageBlockV2::new(&binding, 0, vec![synthetic_page(0)?])?;
    let bytes = block.encode_jsonl()?;
    let reference = block.reference(object(&bytes)?)?;
    let table = RawTableManifestV2::new(&binding, vec![reference])?;
    let bytes = table.encode_jsonl()?;
    let mut lines = bytes.split_inclusive(|byte| *byte == b'\n');
    let mut header: wire::TableHeader = {
        let mut reader =
            records::Records::new(&bytes, &object(&bytes)?, limits::ObjectLimits::DIRECTORY)?;
        reader.next("header", "table_header")?
    };
    header.inventory.raw_bytes = 0;
    lines.next().ok_or("header")?;
    let block_line = lines.next().ok_or("block")?;
    let mut reader = records::Records::new(
        block_line,
        &object(block_line)?,
        limits::ObjectLimits::DIRECTORY,
    )?;
    let mut block: wire::BlockRef = reader.next("block:00000000000000000000", "block_ref")?;
    block.inventory.raw_bytes = 0;
    let mut changed = records::encode("header", "table_header", &header)?;
    changed.extend(records::encode(
        "block:00000000000000000000",
        "block_ref",
        &block,
    )?);
    let result = RawTableManifestV2::decode(&changed, &object(&changed)?, &binding);
    let error = result.err().ok_or("impossible table accepted")?;
    check!(error.to_string().contains("block reference inventory"));
    Ok(())
}
