use super::tests::{fixture, proposal_fixture};
use super::*;
use serde_json::{json, Value};

type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn limits(component_bytes: u64) -> TestResult<RosterAuthorityLimitsV1> {
    Ok(RosterAuthorityLimitsV1::new(64 * 1024, component_bytes)?)
}

fn sample(proposal: bool) -> TestResult<Value> {
    if proposal {
        proposal_fixture()
    } else {
        fixture()
    }
}

fn set_bytes(value: &mut Value, path: &str, bytes: usize) -> TestResult {
    *value.pointer_mut(path).ok_or("reference byte field")? = json!(bytes);
    Ok(())
}

fn decode(value: &Value, proposal: bool, limits: RosterAuthorityLimitsV1) -> super::Result<()> {
    let bytes = serde_json::to_vec(value)?;
    if proposal {
        decode_roster_issuance_proposal_v1(&bytes, limits).map(|_| ())
    } else {
        decode_roster_issuance_claim(&bytes, limits).map(|_| ())
    }
}

fn capacity<T>(
    result: super::Result<T>,
    resource: &str,
    needed: u64,
    available: u64,
) -> TestResult {
    let Err(RosterAuthorityError::Capacity {
        resource: actual_resource,
        needed: actual_needed,
        available: actual_available,
    }) = result
    else {
        return Err("exact capacity refusal required".into());
    };
    check!(eq; actual_resource, resource);
    check!(eq; actual_needed, needed);
    check!(eq; actual_available, available);
    Ok(())
}

#[test]
fn proposal_and_issuance_accept_only_interpretation_up_to_256k() -> TestResult {
    let max = MAX_ROSTER_SEMANTIC_INTERPRETATION_BYTES;
    for proposal in [false, true] {
        for size in [64 * 1024 + 1, max] {
            let mut raw = sample(proposal)?;
            set_bytes(&mut raw, "/capture/interpretation/bytes", size)?;
            decode(&raw, proposal, limits(8 * 1024 * 1024)?)?;
        }
        let mut raw = sample(proposal)?;
        set_bytes(&mut raw, "/capture/interpretation/bytes", max + 1)?;
        capacity(
            decode(&raw, proposal, limits(8 * 1024 * 1024)?),
            "referenced object bytes",
            u64::try_from(max + 1)?,
            u64::try_from(max)?,
        )?;
    }
    Ok(())
}

#[test]
fn every_neighbor_metadata_role_stays_at_64k() -> TestResult {
    for proposal in [false, true] {
        for path in [
            "/parser/artifact/bytes",
            "/parser/schema/bytes",
            "/parser/configuration/bytes",
            "/parser/dependencies/bytes",
            "/capture/manifest/bytes",
            "/capture/retained_source_admission/bytes",
            "/capture/verification_grant/bytes",
            "/binding/school_projection/bytes",
            "/binding/program_projection/bytes",
        ] {
            let mut raw = sample(proposal)?;
            set_bytes(&mut raw, path, 64 * 1024)?;
            decode(&raw, proposal, limits(8 * 1024 * 1024)?)?;
            set_bytes(&mut raw, path, 64 * 1024 + 1)?;
            capacity(
                decode(&raw, proposal, limits(8 * 1024 * 1024)?),
                "referenced object bytes",
                64 * 1024 + 1,
                64 * 1024,
            )?;
        }
    }
    let mut raw = fixture()?;
    set_bytes(&mut raw, "/semantic_review_evidence/bytes", 64 * 1024 + 1)?;
    capacity(
        decode(&raw, false, limits(8 * 1024 * 1024)?),
        "referenced object bytes",
        64 * 1024 + 1,
        64 * 1024,
    )
}

#[test]
fn expanded_interpretation_uses_the_existing_aggregate_component_budget() -> TestResult {
    for proposal in [false, true] {
        let mut raw = sample(proposal)?;
        set_bytes(
            &mut raw,
            "/capture/interpretation/bytes",
            MAX_ROSTER_SEMANTIC_INTERPRETATION_BYTES,
        )?;
        let others: u64 = if proposal { 12 } else { 13 };
        let total = u64::try_from(MAX_ROSTER_SEMANTIC_INTERPRETATION_BYTES)? + others * 64;
        decode(&raw, proposal, limits(total)?)?;
        capacity(
            decode(&raw, proposal, limits(total - 1)?),
            "component bytes",
            total,
            total - 1,
        )?;
    }
    Ok(())
}

fn member_claim() -> TestResult<LocallyCheckedRosterIssuanceV1> {
    Ok(decode_roster_issuance_claim(
        &serde_json::to_vec(&fixture()?)?,
        limits(8 * 1024 * 1024)?,
    )?)
}

#[test]
fn member_interpretation_cap_precedes_json_decode_and_witness_stays_record_bounded() -> TestResult {
    let claim = member_claim()?;
    let max = MAX_ROSTER_SEMANTIC_INTERPRETATION_BYTES;
    let oversized = vec![b'!'; max + 1];
    capacity(
        check_roster_member_witness_v1(&claim, &oversized, b"!", limits(8 * 1024 * 1024)?),
        "interpretation bytes",
        u64::try_from(max + 1)?,
        u64::try_from(max)?,
    )?;
    let exact = oversized
        .get(..max)
        .ok_or("exact interpretation boundary")?;
    check!(matches!(
        check_roster_member_witness_v1(&claim, exact, b"!", limits(8 * 1024 * 1024)?),
        Err(RosterAuthorityError::Invalid(
            "interpretation length mismatch"
        ))
    ));
    let witness = vec![b'!'; 64 * 1024 + 1];
    capacity(
        check_roster_member_witness_v1(&claim, b"{}", &witness, limits(8 * 1024 * 1024)?),
        "witness bytes",
        64 * 1024 + 1,
        64 * 1024,
    )
}

#[test]
fn member_interpretation_and_witness_keep_original_combined_grant() -> TestResult {
    let claim = member_claim()?;
    let interpretation = vec![b'!'; MAX_ROSTER_SEMANTIC_INTERPRETATION_BYTES];
    let total = u64::try_from(interpretation.len())? + 1;
    check!(matches!(
        check_roster_member_witness_v1(&claim, &interpretation, b"!", limits(total)?),
        Err(RosterAuthorityError::Invalid(
            "interpretation length mismatch"
        ))
    ));
    capacity(
        check_roster_member_witness_v1(&claim, &interpretation, b"!", limits(total - 1)?),
        "member evidence bytes",
        total,
        total - 1,
    )
}

#[test]
fn record_and_witness_capacity_do_not_expand_with_component_role() -> TestResult {
    capacity(
        RosterAuthorityLimitsV1::new(64 * 1024 + 1, 8 * 1024 * 1024),
        "record bytes",
        64 * 1024 + 1,
        64 * 1024,
    )?;
    capacity(
        RosterAuthorityLimitsV1::new(64 * 1024, 8 * 1024 * 1024 + 1),
        "component bytes",
        8 * 1024 * 1024 + 1,
        8 * 1024 * 1024,
    )
}
