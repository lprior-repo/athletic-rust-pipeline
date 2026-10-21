use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};

#[test]
fn fragments_preserve_original_whitespace_escape_spelling_and_non_ascii() -> TestResult {
    let binding = capture()?.table_binding(AuthorityTableV2::IdentityVerdicts)?;
    let key = crate::keys::observation_key(crate::Table::IdentityVerdicts, "a", 0);
    let original = serde_json::to_string_pretty(&verdict("a"))?.replace("unknown", "\\u0075nknown");
    let mut raw = original.into_bytes();
    raw.resize(MAX_RAW_VALUE_BYTES, b' ');
    let page = RawPageV2::new(
        &binding,
        RawPagePositionV2::new(0, 0)?,
        vec![RawPhysicalRowV2::new(&binding, &key, &raw)?],
    )?;
    let bytes = page.encode_jsonl()?;
    let reference = object(&bytes)?;
    check!(eq;reference.records(), 4);
    check!(eq;
        page.object_reference(reference.physical_bytes())?,
        reference
    );
    let decoded = RawPageV2::decode(&bytes, &reference, &binding)?;
    let recovered = decoded.rows().first().ok_or("missing row")?;
    check!(eq;recovered.physical_key(), key);
    check!(eq;recovered.raw_value(), raw);
    check!(bytes
        .split_inclusive(|byte| *byte == b'\n')
        .all(|line| line.len() <= MAX_RECORD_BYTES));
    Ok(())
}

#[test]
fn wrong_table_key_payload_id_and_sequence_refuse() -> TestResult {
    let binding = capture()?.table_binding(AuthorityTableV2::IdentityVerdicts)?;
    let value = serde_json::to_vec(&verdict("a"))?;
    for key in [
        crate::keys::observation_key(crate::Table::Athletes, "a", 0),
        crate::keys::observation_key(crate::Table::IdentityVerdicts, "b", 0),
        crate::keys::observation_key(crate::Table::IdentityVerdicts, "a", 1),
        Vec::new(),
    ] {
        check!(RawPhysicalRowV2::new(&binding, &key, &value).is_err());
    }
    Ok(())
}

#[test]
fn derived_generation_is_checked_before_entity_decode() -> TestResult {
    let binding = capture()?.table_binding(AuthorityTableV2::ReviewCases)?;
    let key = crate::keys::derived_key(crate::Table::ReviewCases, 8, b"case");
    let error = RawPhysicalRowV2::new(&binding, &key, b"{")
        .err()
        .ok_or("expected refusal")?;
    check!(error.to_string().contains("derived generation mismatch"));
    Ok(())
}

#[test]
fn standard_padded_base64_only_and_duplicate_raw_field_rejected() -> TestResult {
    check!(eq;raw::decode_base64("YQ==", 1)?, b"a");
    for encoded in ["YQ", "YR==", "YQ==\n", "_w==", "YQ==="] {
        check!(raw::decode_base64(encoded, 1).is_err());
    }
    let (binding, page) = page()?;
    let bytes = page.encode_jsonl()?;
    let changed = replace(
        &bytes,
        "\"value_bytes\":",
        "\"value_bytes\":1,\"value_bytes\":",
    )?;
    check!(RawPageV2::decode(&changed, &object(&changed)?, &binding).is_err());
    let encoded = STANDARD.encode(page.rows().first().ok_or("row")?.raw_value());
    let changed = replace(&bytes, &encoded, &STANDARD.encode(b" "))?;
    check!(RawPageV2::decode(&changed, &object(&changed)?, &binding).is_err());
    Ok(())
}

#[test]
fn original_whitespace_mutation_without_digest_update_refuses() -> TestResult {
    let binding = capture()?.table_binding(AuthorityTableV2::IdentityVerdicts)?;
    let original = row(&binding, "a", 1024)?;
    let source = STANDARD.encode(original.raw_value());
    let mut changed_value = original.raw_value().to_vec();
    *changed_value.last_mut().ok_or("raw tail")? = b'\t';
    let page = RawPageV2::new(&binding, RawPagePositionV2::new(0, 0)?, vec![original])?;
    let changed = replace(
        &page.encode_jsonl()?,
        &source,
        &STANDARD.encode(&changed_value),
    )?;
    check!(RawPageV2::decode(&changed, &object(&changed)?, &binding).is_err());
    Ok(())
}

#[test]
fn fragment_repartitioning_refuses_even_when_reconstructed_bytes_would_match() -> TestResult {
    let binding = capture()?.table_binding(AuthorityTableV2::IdentityVerdicts)?;
    let original = row(&binding, "a", MAX_RAW_VALUE_BYTES)?;
    let page = RawPageV2::new(
        &binding,
        RawPagePositionV2::new(0, 0)?,
        vec![original.clone()],
    )?;
    let boundary = RAW_FRAGMENT_BYTES - 1;
    let (first, second) = original.raw_value().split_at(boundary);
    let mut bytes = records::encode("header", "raw_page_header", &page.header)?;
    bytes.extend(records::encode(
        "row:00000000000000000000",
        "raw_row",
        &original.header()?,
    )?);
    bytes.extend(records::encode(
        "row:00000000000000000000:part:00000000000000000000",
        "raw_part",
        &wire::RawPart {
            bytes_base64: STANDARD.encode(first),
        },
    )?);
    bytes.extend(records::encode(
        "row:00000000000000000000:part:00000000000000000001",
        "raw_part",
        &wire::RawPart {
            bytes_base64: STANDARD.encode(second),
        },
    )?);
    check!(
        matches!(RawPageV2::decode(&bytes, &object(&bytes)?, &binding),
        Err(crate::StoreError::Refused { detail }) if detail == "raw public identity generation V2: raw fragment length")
    );
    Ok(())
}
