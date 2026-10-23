use super::*;
use crate::milesplit::RosterRejectionKind;
use crate::participation_evidence::provider_school::{
    OwnedProviderSchoolPacketV1, OWNED_PROVIDER_SCHOOL_SEMANTIC_CONTRACT,
};

const ORIGINAL: &[u8] =
    include_bytes!("../../provider_school/fixtures/quincy-owned-school-packet-20261022.json");
const REFERENCE: &str = "e1bef9656d153bf2d66fae2d0595f7e38cf2ff6833ee15bc107287bf6be7cde7";

#[test]
fn genuine_quincy_numeric_only_identity_preserves_rejection_and_all201_dispositions() -> TestResult
{
    check!(eq; ORIGINAL.len(), 1_056_933);
    check!(eq; hash(ORIGINAL), REFERENCE);
    let packet = OwnedProviderSchoolPacketV1::decode(ORIGINAL)?;
    check!(eq; packet.reference()?.as_str(), REFERENCE);
    check!(eq; packet.encode()?, ORIGINAL);
    let checked = packet.read()?;
    check!(eq; checked.semantic_contract(), OWNED_PROVIDER_SCHOOL_SEMANTIC_CONTRACT);
    let rows = rows::inventory(
        checked.roster_verdict(),
        &mut AttachmentReplayGrantV1::new(),
    )?;
    check!(eq; rows.len(), 201);
    let first = rows.first().ok_or("original Quincy row absent")?;
    check!(eq; first.provider_id, "14789618");
    check!(eq; first.row.ordinal, 0);
    check!(eq; first.row.byte_offset, 76_856);
    check!(eq; first.row.byte_length, 1910);
    check!(matches!(
        first.outcome,
        wire::Outcome::Rejected {
            reason: RosterRejectionKind::MissingName
        }
    ));
    check!(rows.iter().all(|row| !row.provider_id.is_empty()));
    check!(eq; rows.iter().filter(|row| row.provider_id == "14789618").count(), 1);
    let parser = super::super::super::manifest::compiled_roster_parser_manifest()?;
    let bytes = capture(
        &checked,
        parser.canonical_bytes(),
        b"controlled metadata; no source approval",
        &mut AttachmentReplayGrantV1::new(),
    )?;
    let inventory: wire::Inventory = serde_json::from_slice(&bytes)?;
    check!(eq; inventory.packet, REFERENCE);
    check!(eq; inventory.provider_semantic_contract, OWNED_PROVIDER_SCHOOL_SEMANTIC_CONTRACT);
    check!(matches!(inventory.disposition, wire::Disposition::Partial));
    check!(eq; inventory.rows.len(), 201);
    check!(eq; packet.encode()?, ORIGINAL);
    Ok(())
}
