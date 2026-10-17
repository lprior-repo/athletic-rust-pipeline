use super::*;
use census_crawl::participation_evidence::roster_source_issuance::semantic_inventory::decode_untrusted_roster_semantic_inventory_v1;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Interpretation {
    schema: String,
    original_capture_interpretation: Box<RawValue>,
    semantic_inventory: Box<RawValue>,
}

pub(super) fn mutate(bytes: &[u8], mutation: Mutation) -> TestResult<Vec<u8>> {
    let mut interpretation: Interpretation = serde_json::from_slice(bytes)?;
    check!(eq; serde_json::to_vec(&interpretation)?, bytes);
    check!(eq; interpretation.schema, "roster-retained-capture-interpretation-v2");
    let original = interpretation.semantic_inventory.get();
    let decoded = decode_untrusted_roster_semantic_inventory_v1(
        original.as_bytes(),
        &mut AttachmentReplayGrantV1::new(),
    )?;
    check!(eq; decoded.member_count(), 1);
    let value: Value = serde_json::from_str(original)?;
    let changed = match mutation {
        Mutation::OmitMember => {
            let start = original
                .rfind("\"members\":[")
                .ok_or("members final field")?;
            format!(
                "{}\"members\":[]}}",
                original.get(..start).ok_or("inventory prefix")?
            )
        }
        Mutation::ChangeName => {
            let name = value.pointer("/members/0/name").ok_or("member name")?;
            let needle = format!("\"name\":{}", serde_json::to_string(name)?);
            check!(eq; original.matches(&needle).count(), 1);
            original.replacen(&needle, "\"name\":\"Forged Controlled Person\"", 1)
        }
        Mutation::DuplicateMember => {
            let start = original
                .rfind("\"members\":[")
                .ok_or("members final field")?;
            let tail = original.get(start..).ok_or("member suffix")?;
            let row = tail
                .strip_prefix("\"members\":[")
                .and_then(|value| value.strip_suffix("]}"))
                .ok_or("single member bytes")?;
            format!(
                "{}\"members\":[{row},{row}]}}",
                original.get(..start).ok_or("inventory prefix")?
            )
        }
    };
    check!(changed != original);
    let decoded = decode_untrusted_roster_semantic_inventory_v1(
        changed.as_bytes(),
        &mut AttachmentReplayGrantV1::new(),
    )?;
    let expected = match mutation {
        Mutation::OmitMember => 0,
        Mutation::ChangeName => 1,
        Mutation::DuplicateMember => 2,
    };
    check!(eq; decoded.member_count(), expected);
    interpretation.semantic_inventory = serde_json::from_str(&changed)?;
    Ok(serde_json::to_vec(&interpretation)?)
}
