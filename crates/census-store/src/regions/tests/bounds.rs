use super::*;
use census_domain::model::{RegionAssertionV1, RegionEvidenceV1, MAX_REGION_PAYLOAD_BYTES};

fn full_payload() -> TestResult<RegionMembershipContractV1> {
    let original = contract(7)?;
    let template = original
        .input()
        .assertions
        .first()
        .ok_or("missing synthetic assertion")?;
    let mut input = original.input().clone();
    input.assertions.clear();
    input.assertions.try_reserve_exact(256)?;
    for index in 0..256 {
        let mut assertion = template.input().clone();
        let mut evidence = assertion.provenance.input().clone();
        evidence.locator = format!("synthetic-boundary:{index:04}");
        evidence.evidence.note = Some("x".to_string());
        assertion.provenance = RegionEvidenceV1::new(evidence)?;
        input.assertions.push(RegionAssertionV1::new(assertion));
    }
    let base = census_domain::model::serialized_bytes(&input)?.len();
    let mut remaining = MAX_REGION_PAYLOAD_BYTES
        .checked_sub(base)
        .ok_or("base fixture exceeds payload budget")?;
    let capacity = 256usize.checked_mul(4095).ok_or("note capacity overflow")?;
    if remaining > capacity {
        return Err("fixture cannot reach payload ceiling".into());
    }
    for assertion in &mut input.assertions {
        let extra = remaining.min(4095);
        let length = extra.checked_add(1).ok_or("note length overflow")?;
        let mut changed = assertion.input().clone();
        let mut evidence = changed.provenance.input().clone();
        evidence.evidence.note = Some("x".repeat(length));
        changed.provenance = RegionEvidenceV1::new(evidence)?;
        *assertion = RegionAssertionV1::new(changed);
        remaining = remaining
            .checked_sub(extra)
            .ok_or("remaining bytes underflow")?;
    }
    equal(remaining, 0)?;
    let payload = RegionMembershipContractV1::new(input)?;
    equal(
        census_domain::model::serialized_bytes(&payload)?.len(),
        MAX_REGION_PAYLOAD_BYTES,
    )?;
    Ok(payload)
}

#[test]
fn genuine_one_mib_payload_registers_repeats_and_survives_reopen() -> TestResult {
    let directory = tempfile::tempdir()?;
    let payload = full_payload()?;
    let reference = payload.reference()?;
    {
        let store = Store::open_with(directory.path(), CACHE_BYTES)?;
        seed(&store)?;
        equal(
            store.register_region_contract(&payload)?,
            RegionRegistration::Registered(reference.clone()),
        )?;
        let raw = store
            .db
            .snapshot()
            .get(&store.meta, key(&reference))?
            .ok_or("registered full payload missing")?;
        if raw.len() <= MAX_REGION_PAYLOAD_BYTES || raw.len() > MAX_REGION_RECORD_BYTES {
            return Err(format!("full payload record length={}", raw.len()).into());
        }
        equal(store.region_contract(&reference)?, payload.clone())?;
        let before = Witness::capture(&store)?;
        equal(
            store.register_region_contract(&payload)?,
            RegionRegistration::Repeated(reference.clone()),
        )?;
        equal(Witness::capture(&store)?, before)?;
        store.flush()?;
    }
    let reopened = Store::open_with(directory.path(), CACHE_BYTES)?;
    equal(reopened.region_contract(&reference)?, payload)
}

#[test]
fn oversized_nonsyntax_is_refused_before_record_decoding_without_writes() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open_with(directory.path(), CACHE_BYTES)?;
    let payload = contract(7)?;
    let length = MAX_REGION_RECORD_BYTES
        .checked_add(1)
        .ok_or("record length overflow")?;
    let bytes = vec![b'!'; length];
    inject(&store, &payload.reference()?, &bytes)?;
    for error in unchanged_refusals(&store, &payload)? {
        match error {
            RegionRegistryError::Oversized { bytes, max } => {
                equal(bytes, length)?;
                equal(max, MAX_REGION_RECORD_BYTES)?;
            }
            other => return Err(format!("oversized nonsyntax={other:?}").into()),
        }
    }
    Ok(())
}

#[test]
fn exact_record_ceiling_admits_legal_whitespace_but_next_byte_refuses() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open_with(directory.path(), CACHE_BYTES)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    let mut bytes = serde_json::to_vec(&envelope_value(&payload)?)?;
    bytes.resize(MAX_REGION_RECORD_BYTES, b' ');
    inject(&store, &reference, &bytes)?;
    equal(store.region_contract(&reference)?, payload.clone())?;
    let before = Witness::capture(&store)?;
    equal(
        store.register_region_contract(&payload)?,
        RegionRegistration::Repeated(reference.clone()),
    )?;
    equal(Witness::capture(&store)?, before)?;
    bytes.push(b' ');
    inject(&store, &reference, &bytes)?;
    let length = MAX_REGION_RECORD_BYTES
        .checked_add(1)
        .ok_or("record length overflow")?;
    for error in unchanged_refusals(&store, &payload)? {
        match error {
            RegionRegistryError::Oversized { bytes, max } => {
                equal(bytes, length)?;
                equal(max, MAX_REGION_RECORD_BYTES)?;
            }
            other => return Err(format!("one byte over record ceiling={other:?}").into()),
        }
    }
    Ok(())
}
