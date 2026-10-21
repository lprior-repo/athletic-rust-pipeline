use super::tests::{fixture, proposal_fixture};
use super::*;
use serde_json::{json, Value};

type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const MANIFEST: &str = "/binding/physical_projection_inventory";

#[derive(Clone, Copy)]
enum Claim {
    Proposal,
    Issuance,
    Record,
}

impl Claim {
    fn sample(self) -> TestResult<Value> {
        match self {
            Self::Proposal => proposal_fixture(),
            Self::Issuance => fixture(),
            Self::Record => Ok(json!({
                "version":1, "lifecycle":"issuance_only_v1", "operation":"manifest-capacity",
                "proposal":fixture()?, "store":"c".repeat(64), "snapshot_revision":1,
                "parent_inventory":reference(15), "negative_inventory":reference(16),
                "negative_actions":0, "approval_envelope":reference(17),
                "review_provenance":reference(18), "issuance_receipt":reference(19)
            })),
        }
    }

    fn path(self, suffix: &str) -> String {
        if matches!(self, Self::Record) {
            format!("/proposal{suffix}")
        } else {
            suffix.to_owned()
        }
    }

    fn decode(self, value: &Value, component_bytes: u64) -> super::Result<()> {
        let bytes = serde_json::to_vec(value)?;
        let limits = RosterAuthorityLimitsV1::new(64 * 1024, component_bytes)?;
        match self {
            Self::Proposal => decode_roster_issuance_proposal_v1(&bytes, limits).map(|_| ()),
            Self::Issuance => decode_roster_issuance_claim(&bytes, limits).map(|_| ()),
            Self::Record => decode_roster_record_claim(&bytes, limits).map(|_| ()),
        }
    }

    fn other_references(self) -> u64 {
        match self {
            Self::Proposal => 12,
            Self::Issuance => 13,
            Self::Record => 18,
        }
    }
}

fn reference(id: u32) -> Value {
    json!({"digest":format!("{id:064x}"), "bytes":64})
}

fn set(value: &mut Value, path: &str, replacement: Value) -> TestResult {
    *value.pointer_mut(path).ok_or("capacity fixture path")? = replacement;
    Ok(())
}

fn manifest_bytes(value: &mut Value, claim: Claim, size: usize) -> TestResult {
    set(
        value,
        &claim.path(&format!("{MANIFEST}/bytes")),
        json!(size),
    )
}

fn capacity(result: super::Result<()>, resource: &str, needed: u64, available: u64) -> TestResult {
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
fn all_claims_admit_only_physical_inventory_up_to_existing_one_mib_bound() -> TestResult {
    let max = MAX_ROSTER_PHYSICAL_PROJECTION_MANIFEST_BYTES;
    for claim in [Claim::Proposal, Claim::Issuance, Claim::Record] {
        for size in [64 * 1024 + 1, 68_462, max] {
            let mut value = claim.sample()?;
            manifest_bytes(&mut value, claim, size)?;
            claim.decode(&value, MAX_ROSTER_AUTHORITY_COMPONENT_BYTES)?;
        }
        let mut value = claim.sample()?;
        manifest_bytes(&mut value, claim, max + 1)?;
        capacity(
            claim.decode(&value, MAX_ROSTER_AUTHORITY_COMPONENT_BYTES),
            "referenced object bytes",
            u64::try_from(max + 1)?,
            u64::try_from(max)?,
        )?;
    }
    Ok(())
}

#[test]
fn physical_inventory_uses_original_aggregate_component_grant() -> TestResult {
    let max = MAX_ROSTER_PHYSICAL_PROJECTION_MANIFEST_BYTES;
    for claim in [Claim::Proposal, Claim::Issuance, Claim::Record] {
        let mut value = claim.sample()?;
        manifest_bytes(&mut value, claim, max)?;
        let total = u64::try_from(max)? + claim.other_references() * 64;
        claim.decode(&value, total)?;
        capacity(
            claim.decode(&value, total - 1),
            "component bytes",
            total,
            total - 1,
        )?;
    }
    Ok(())
}

#[test]
fn duplicate_manifest_reference_is_counted_once_and_conflicting_length_refuses() -> TestResult {
    let max = MAX_ROSTER_PHYSICAL_PROJECTION_MANIFEST_BYTES;
    for claim in [Claim::Proposal, Claim::Issuance, Claim::Record] {
        let mut value = claim.sample()?;
        manifest_bytes(&mut value, claim, max)?;
        let manifest = value
            .pointer(&claim.path(MANIFEST))
            .ok_or("manifest reference")?
            .clone();
        set(&mut value, &claim.path("/capture/body"), manifest)?;
        let total = u64::try_from(max)? + (claim.other_references() - 1) * 64;
        claim.decode(&value, total)?;
        capacity(
            claim.decode(&value, total - 1),
            "component bytes",
            total,
            total - 1,
        )?;
        set(
            &mut value,
            &claim.path("/capture/body/bytes"),
            json!(max - 1),
        )?;
        check!(matches!(
            claim.decode(&value, MAX_ROSTER_AUTHORITY_COMPONENT_BYTES),
            Err(RosterAuthorityError::ConflictingObjectReference)
        ));
    }
    Ok(())
}

#[test]
fn manifest_alignment_preserves_all_record_proof_metadata_limits() -> TestResult {
    let claim = Claim::Record;
    for path in [
        "/parent_inventory/bytes",
        "/negative_inventory/bytes",
        "/approval_envelope/bytes",
        "/review_provenance/bytes",
        "/issuance_receipt/bytes",
    ] {
        let mut value = claim.sample()?;
        manifest_bytes(
            &mut value,
            claim,
            MAX_ROSTER_PHYSICAL_PROJECTION_MANIFEST_BYTES,
        )?;
        set(&mut value, path, json!(64 * 1024))?;
        claim.decode(&value, MAX_ROSTER_AUTHORITY_COMPONENT_BYTES)?;
        set(&mut value, path, json!(64 * 1024 + 1))?;
        capacity(
            claim.decode(&value, MAX_ROSTER_AUTHORITY_COMPONENT_BYTES),
            "referenced object bytes",
            64 * 1024 + 1,
            64 * 1024,
        )?;
    }
    Ok(())
}
