use crate::{Store, MAX_JOURNAL_VALUE_BYTES};

use super::process::bounded_case;
use super::support::{
    canonical, envelope, equal, fixture_dir, inject, refuse_preserving, Refusal, TestResult,
    Witness, KEY, LOWER, PHASE,
};

fn one_case(name: &str, raw: &[u8], expected: Refusal) -> TestResult {
    refuse_preserving(&[canonical(raw)], expected)
        .map_err(|error| format!("lineage case {name}: {error}").into())
}

#[test]
fn invalid_json_preserves_the_exact_decode_key_and_original_record() -> TestResult {
    for raw in [
        b"{".as_slice(),
        b"{\"key\":\"store-identity\",\"payload\":".as_slice(),
        b"{\"key\":\"store-identity\",\"payload\":\"bad\",}".as_slice(),
        b"{\"key\":42,\"payload\":\"bad\",}".as_slice(),
        b"{\"key\":\"store-identity\",\"key\":\"store-identity\",}".as_slice(),
        b"{\"type\":\"chunked-manifest\",}".as_slice(),
    ] {
        one_case("invalid JSON", raw, Refusal::Decode)?;
    }
    Ok(())
}

#[test]
fn invalid_utf8_and_surrogate_escapes_in_unknown_metadata_remain_decode_errors() -> TestResult {
    let mut raw = format!("{{\"key\":\"store-identity\",\"payload\":\"{LOWER}\",\"metadata\":\"")
        .into_bytes();
    raw.push(0xff);
    raw.extend_from_slice(b"\"}");
    one_case("unknown metadata invalid UTF8", &raw, Refusal::Decode)?;
    let surrogate =
        format!(r#"{{"key":"store-identity","payload":"{LOWER}","metadata":"\ud800"}}"#);
    one_case(
        "unknown metadata invalid surrogate",
        surrogate.as_bytes(),
        Refusal::Decode,
    )
}

#[test]
fn malformed_inline_shapes_refuse_without_minting_or_replacing_records() -> TestResult {
    let malformed = [
        "{}".to_owned(),
        "null".to_owned(),
        "42".to_owned(),
        "true".to_owned(),
        format!("\"{LOWER}\""),
        "[]".to_owned(),
        format!("[\"store-identity\",\"{LOWER}\"]"),
        format!("[\"store-identity\",\"{LOWER}\",null]"),
        "{\"key\":\"store-identity\"}".to_owned(),
        "{\"key\":\"store-identity\",\"payload\":null}".to_owned(),
        "{\"key\":\"store-identity\",\"payload\":0}".to_owned(),
        "{\"key\":\"store-identity\",\"payload\":true}".to_owned(),
        "{\"key\":\"store-identity\",\"payload\":{}}".to_owned(),
        format!("{{\"key\":\"store-identity\",\"payload\":[\"{LOWER}\"]}}"),
        format!("{{\"payload\":\"{LOWER}\"}}"),
        format!("{{\"key\":null,\"payload\":\"{LOWER}\"}}"),
        format!("{{\"key\":42,\"payload\":\"{LOWER}\"}}"),
        format!("{{\"key\":[],\"payload\":\"{LOWER}\"}}"),
    ];
    for raw in malformed {
        one_case(&raw, raw.as_bytes(), Refusal::MalformedEnvelope)?;
    }
    Ok(())
}

#[test]
fn invalid_identity_bytes_remain_domain_errors_without_normalization() -> TestResult {
    let invalid = [
        String::new(),
        "a".repeat(63),
        "a".repeat(65),
        "g".repeat(64),
        format!(" {LOWER}"),
        format!("{LOWER} "),
        format!("0x{LOWER}"),
        format!("{}-", "a".repeat(63)),
        "é".repeat(32),
        format!("{}é", "a".repeat(62)),
    ];
    for identity in invalid {
        one_case(&identity, &envelope(&identity), Refusal::InvalidIdentity)?;
    }
    Ok(())
}

#[test]
fn duplicate_identity_bearing_fields_refuse_even_when_their_values_match() -> TestResult {
    let duplicates = [
        format!(
            "{{\"key\":\"store-identity\",\"key\":\"store-identity\",\"payload\":\"{LOWER}\"}}"
        ),
        format!("{{\"key\":\"foreign\",\"key\":\"store-identity\",\"payload\":\"{LOWER}\"}}"),
        format!("{{\"key\":\"store-identity\",\"key\":\"foreign\",\"payload\":\"{LOWER}\"}}"),
        format!("{{\"key\":\"store-identity\",\"payload\":\"{LOWER}\",\"payload\":\"{LOWER}\"}}"),
        format!("{{\"key\":\"store-identity\",\"payload\":null,\"payload\":\"{LOWER}\"}}"),
        format!("{{\"key\":\"store-identity\",\"payload\":\"{LOWER}\",\"payload\":null}}"),
    ];
    for raw in duplicates {
        one_case(&raw, raw.as_bytes(), Refusal::MalformedEnvelope)?;
    }
    Ok(())
}

#[test]
fn physical_and_envelope_keys_must_both_be_canonical() -> TestResult {
    for raw in [
        format!("{{\"key\":\"foreign\",\"payload\":\"{LOWER}\"}}"),
        format!("{{\"key\":\"\",\"payload\":\"{LOWER}\"}}"),
    ] {
        one_case(
            "foreign envelope key",
            raw.as_bytes(),
            Refusal::UnexpectedKey,
        )?;
    }
    let foreign = (Store::journal_key(PHASE, "foreign"), envelope(LOWER));
    refuse_preserving(&[foreign], Refusal::UnexpectedKey)?;
    let invalid_foreign = (Store::journal_key(PHASE, "foreign"), b"{".to_vec());
    refuse_preserving(&[invalid_foreign], Refusal::UnexpectedKey)?;
    let mut non_utf8 = Store::journal_key(PHASE, "");
    non_utf8.extend_from_slice(&[0xff, 0xfe]);
    refuse_preserving(&[(non_utf8, envelope(LOWER))], Refusal::UnexpectedKey)?;
    let mut hostile_key = Store::journal_key(PHASE, "");
    let extra = 65_535_usize
        .checked_sub(hostile_key.len())
        .ok_or("the phase prefix exceeds Fjall's physical key ceiling")?;
    hostile_key.extend_from_slice(&vec![b'x'; extra]);
    refuse_preserving(&[(hostile_key, envelope(LOWER))], Refusal::UnexpectedKey)
}

#[test]
fn a_second_physical_record_refuses_before_decoding_the_first_value() -> TestResult {
    let extra = (Store::journal_key(PHASE, "extra"), envelope(LOWER));
    refuse_preserving(
        &[canonical(&envelope(LOWER)), extra.clone()],
        Refusal::Multiple,
    )?;
    refuse_preserving(&[canonical(b"{"), extra.clone()], Refusal::Multiple)?;
    let oversize_bytes = MAX_JOURNAL_VALUE_BYTES
        .checked_add(1)
        .ok_or("size overflow")?;
    refuse_preserving(
        &[canonical(&vec![b'{'; oversize_bytes]), extra],
        Refusal::Multiple,
    )?;
    refuse_preserving(
        &[
            (Store::journal_key(PHASE, "a-foreign"), envelope(LOWER)),
            (Store::journal_key(PHASE, "b-foreign"), b"{".to_vec()),
        ],
        Refusal::Multiple,
    )
}

#[test]
fn many_extra_rows_and_hostile_manifest_counts_never_trigger_chunk_reassembly() -> TestResult {
    bounded_case(
        concat!(
            module_path!(),
            "::many_extra_rows_and_hostile_manifest_counts_never_trigger_chunk_reassembly"
        ),
        || {
            let hostile = br#"{"type":"chunked-manifest","version":1,"key":"store-identity","chunks":18446744073709551615,"total_bytes":18446744073709551615}"#;
            one_case(
                "hostile manifest",
                hostile,
                Refusal::UnsupportedRepresentation,
            )?;
            let mut records = vec![canonical(hostile)];
            for index in 0..128 {
                records.push((
                    Store::journal_key(PHASE, &format!("foreign-{index:03}")),
                    b"{".to_vec(),
                ));
            }
            refuse_preserving(&records, Refusal::Multiple)
        },
    )
}

#[test]
fn chunk_alias_and_object_forms_are_refused_as_unsupported_representations() -> TestResult {
    let forms = [
        "{\"type\":\"chunked-manifest\",\"version\":1,\"key\":\"store-identity\",\"chunks\":1,\"total_bytes\":64}".to_owned(),
        "{\"type\":\"chunk\",\"version\":1,\"key\":\"store-identity\",\"index\":0,\"total\":1,\"data\":\"not-base64\"}".to_owned(),
        format!("{{\"type\":\"journal-alias\",\"version\":2,\"key\":\"store-identity\",\"state\":\"ready\",\"object\":\"{LOWER}\"}}"),
        format!("{{\"type\":\"journal-manifest\",\"version\":2,\"key\":\"store-identity\",\"payload\":\"{LOWER}\"}}"),
        format!("{{\"type\":\"journal-index\",\"key\":\"store-identity\",\"payload\":\"{LOWER}\"}}"),
        format!("{{\"type\":\"journal-chunk\",\"key\":\"store-identity\",\"payload\":\"{LOWER}\"}}"),
        format!("{{\"type\":null,\"key\":\"store-identity\",\"payload\":\"{LOWER}\"}}"),
        format!("{{\"type\":{{}},\"key\":\"store-identity\",\"payload\":\"{LOWER}\"}}"),
    ];
    for raw in forms {
        one_case(&raw, raw.as_bytes(), Refusal::UnsupportedRepresentation)?;
    }
    let physical_chunk = crate::journal::chunk_key(&Store::journal_key(PHASE, KEY), 0)?;
    refuse_preserving(&[(physical_chunk, b"{}".to_vec())], Refusal::UnexpectedKey)
}

fn padded_envelope(bytes: usize) -> TestResult<Vec<u8>> {
    let base = format!("{{\"key\":\"store-identity\",\"payload\":\"{LOWER}\",\"padding\":\"\"}}");
    let padding = bytes
        .checked_sub(base.len())
        .ok_or("requested padded size is too small")?;
    let raw = format!(
        "{{\"key\":\"store-identity\",\"payload\":\"{LOWER}\",\"padding\":\"{}\"}}",
        "p".repeat(padding)
    )
    .into_bytes();
    equal(raw.len(), bytes)?;
    Ok(raw)
}

#[test]
fn exact_raw_envelope_size_limit_is_accepted_and_next_byte_is_refused() -> TestResult {
    let dir = fixture_dir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let raw = padded_envelope(MAX_JOURNAL_VALUE_BYTES)?;
    inject(&store, &[canonical(&raw)])?;
    let before = Witness::capture(&store)?;
    equal(store.store_identity()?.as_str(), LOWER)?;
    equal(Witness::capture(&store)?, before)?;
    let bytes = MAX_JOURNAL_VALUE_BYTES
        .checked_add(1)
        .ok_or("size overflow")?;
    let expected = Refusal::OversizedEnvelope {
        bytes,
        max: MAX_JOURNAL_VALUE_BYTES,
    };
    refuse_preserving(&[canonical(&padded_envelope(bytes)?)], expected)?;
    refuse_preserving(&[canonical(&vec![b'{'; bytes])], expected)
}

#[test]
fn unrelated_journal_phases_do_not_prevent_creation_or_get_rewritten() -> TestResult {
    let dir = fixture_dir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let unrelated = Store::journal_key("export-lineage-v10", KEY);
    inject(&store, &[(unrelated.clone(), b"{".to_vec())])?;
    let before = Witness::capture(&store)?;
    let identity = store.store_identity()?;
    let after = Witness::capture(&store)?;
    before.unchanged_evidence(&after)?;
    equal(
        after.sequence,
        before.sequence.checked_add(1).ok_or("sequence overflow")?,
    )?;
    let retained = store
        .journal
        .get(&unrelated)
        .map_err(|source| crate::StoreError::Read { source })?
        .ok_or("the unrelated journal record disappeared")?;
    equal(retained.as_ref(), b"{".as_slice())?;
    equal(store.snapshot().store_identity()?, identity)?;
    equal(Witness::capture(&store)?, after)
}
