use super::*;
use census_domain::model::serialized_bytes;
use serde_json::{json, Value};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod capacity;
mod fixture;

#[test]
fn controlled_semantic_inventory_codec_retains_all_rows_and_typed_gaps() -> TestResult {
    for published_owner in [true, false] {
        let inventory = fixture::inventory(published_owner)?;
        check!(eq; inventory.members.len(), 2);
        let first = inventory.members.first().ok_or("controlled first row")?;
        let last = inventory.members.last().ok_or("controlled last row")?;
        check!(eq; first.provider_id, "42");
        check!(eq; last.provider_id, "43");
        check!(eq; first.sports, [true, false, false]);
        check!(eq; last.sports, [true, false, false]);
        match (&inventory.owners, published_owner) {
            (wire::Owners::Available { declarations }, true) => check!(eq; declarations.len(), 1),
            (
                wire::Owners::Unavailable {
                    reason: wire::WitnessGap::MissingPublishedOwner,
                },
                false,
            ) => {}
            _ => return Err("controlled owner disposition differs".into()),
        }
        let bytes = encoding::encode(&inventory, &mut AttachmentReplayGrantV1::new())?;
        let decoded = decode_untrusted_roster_semantic_inventory_v1(
            &bytes,
            &mut AttachmentReplayGrantV1::new(),
        )?;
        check!(eq; decoded.member_count(), 2);
        check!(eq; decoded.canonical_bytes(), bytes.as_slice());
    }
    Ok(())
}

#[test]
fn controlled_semantic_codec_refuses_unknown_version_fields_and_noncanonical_bytes() -> TestResult {
    let inventory = fixture::inventory(true)?;
    let bytes = encoding::encode(&inventory, &mut AttachmentReplayGrantV1::new())?;
    let original: Value = serde_json::from_slice(&bytes)?;
    let mut unknown_schema = original.clone();
    *unknown_schema.get_mut("schema").ok_or("schema")? =
        json!("roster-source-semantic-inventory-v999");
    let mut unknown_contract = original.clone();
    *unknown_contract
        .get_mut("interpretation")
        .ok_or("interpretation")? = json!("future-contract");
    let mut unknown_field = original;
    unknown_field
        .as_object_mut()
        .ok_or("inventory object")?
        .insert("unreviewed_authority".into(), json!(true));
    let mut whitespace = bytes;
    whitespace.push(b'\n');
    for changed in [
        serialized_bytes(&unknown_schema)?,
        serialized_bytes(&unknown_contract)?,
        serialized_bytes(&unknown_field)?,
        whitespace,
    ] {
        let mut grant = AttachmentReplayGrantV1::new();
        check!(decode_untrusted_roster_semantic_inventory_v1(&changed, &mut grant).is_err());
        check!(grant.ensure_available().is_err());
    }
    Ok(())
}

#[test]
fn controlled_semantic_codec_alone_does_not_certify_complete_inventory() -> TestResult {
    let mut inventory = fixture::inventory(true)?;
    check!(eq; inventory.members.len(), 2);
    inventory.members.pop().ok_or("controlled omitted row")?;
    let bytes = encoding::encode(&inventory, &mut AttachmentReplayGrantV1::new())?;
    let untrusted =
        decode_untrusted_roster_semantic_inventory_v1(&bytes, &mut AttachmentReplayGrantV1::new())?;
    check!(eq; untrusted.member_count(), 1);
    Ok(())
}
