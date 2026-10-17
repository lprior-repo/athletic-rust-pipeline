use super::source::{budget, domain};
use crate::milesplit::{Roster, RosterVerdict};
use crate::participation_evidence::provider_school::CheckedProviderSchoolPacketV1;
use crate::participation_evidence::{bound, hash, invalid, json};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::StoreResult;

mod rows;
mod wire;
mod kernel;
mod replay;
pub(super) use replay::compare;
const SCHEMA: &str = "roster-source-semantic-inventory-v2";
const POLICY: &str = "milesplit-row-local-current-roster/v2";
const LIMIT: usize = 64 * 1024;
const MAX_ROWS: usize = 256;

pub(super) fn checked_roster<'a>(
    checked: &'a CheckedProviderSchoolPacketV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<&'a Roster> {
    rows::inventory(checked.roster_verdict(), grant)?;
    checked.roster_verdict().roster().ok_or_else(|| invalid("row-local roster quarantined"))
}

pub(super) fn capture(
    checked: &CheckedProviderSchoolPacketV1,
    parser: &[u8],
    metadata: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    let rows = rows::inventory(checked.roster_verdict(), grant)?;
    let roster = checked.roster_verdict().roster().ok_or_else(|| invalid("row-local roster quarantined"))?;
    let context = encode(&(checked.school(), checked.observation(), checked.directory_proof(), &roster.team), grant)?;
    let kernel = kernel::compiled()?;
    grant.charge_bytes(kernel.len()).map_err(domain)?;
    let value = wire::Inventory {
        schema: SCHEMA.into(), policy: POLICY.into(),
        provider_semantic_contract: crate::participation_evidence::provider_school::PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1.into(),
        kernel_document: std::str::from_utf8(kernel).map_err(|_| invalid("row-local kernel UTF8"))?.into(),
        packet: checked.reference().as_str().into(), parser_manifest: hash(parser),
        metadata: hash(metadata), context: hash(&context),
        disposition: match checked.roster_verdict() {
            RosterVerdict::Complete { .. } => wire::Disposition::Complete,
            RosterVerdict::Partial { .. } => wire::Disposition::Partial,
            RosterVerdict::Quarantined { .. } => return Err(invalid("row-local roster quarantined")),
        }, rows,
    };
    encode(&value, grant)
}

fn encode(value: &impl serde::Serialize, grant: &mut AttachmentReplayGrantV1) -> StoreResult<Vec<u8>> {
    budget::charge_bounded(value, grant, LIMIT)?;
    grant.charge_steps(1).map_err(domain)?;
    let bytes = serde_json::to_vec(value).map_err(json)?;
    bound(bytes.len(), LIMIT, "row-local semantic inventory")?;
    Ok(bytes)
}

#[cfg(test)]
mod tests;
