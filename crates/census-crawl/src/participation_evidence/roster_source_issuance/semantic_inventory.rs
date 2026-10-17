use super::manifest::CheckedRosterParserManifestV1;
use super::source::{budget, domain, original};
use crate::participation_evidence::provider_school::CheckedProviderSchoolPacketV1;
use crate::participation_evidence::{bound, hash, invalid, json};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::{RosterSourceEvidenceV1, StoreResult};

mod capture;
mod encoding;
pub(in crate::participation_evidence) mod kernel;
mod replay;
pub(super) use replay::compare_original_raw_v1;
mod statements;
mod wire;

const SCHEMA: &str = "roster-source-semantic-inventory-v1";
const INTERPRETATION: &str = "milesplit-scoped-current-roster/v1";
const LIMIT: usize = 64 * 1024;

pub struct CapturedRosterSemanticInventoryV1<'a> {
    evidence: &'a RosterSourceEvidenceV1,
    bytes: Vec<u8>,
    inventory: wire::Inventory,
}

impl CapturedRosterSemanticInventoryV1<'_> {
    pub(super) fn value(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&impl serde::Serialize> {
        self.evidence.require_original_grant(grant)?;
        Ok(&self.inventory)
    }
    pub fn bytes(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&[u8]> {
        self.evidence.require_original_grant(grant)?;
        Ok(&self.bytes)
    }
}

pub(super) fn capture<'a>(
    evidence: &'a RosterSourceEvidenceV1,
    checked: &CheckedProviderSchoolPacketV1,
    parser: &CheckedRosterParserManifestV1,
    metadata: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<CapturedRosterSemanticInventoryV1<'a>> {
    let result = original(evidence, grant, |grant| {
        if crate::milesplit::CurrentRosterAssertion::INTERPRETATION_REVISION != INTERPRETATION {
            return Err(invalid("current roster semantic contract needs explicit adapter").into());
        }
        if checked.reference().as_str() != evidence.retained_root(grant)?.root().key {
            return Err(invalid("semantic inventory packet differs from retained root").into());
        }
        let inventory = capture::inventory(
            checked,
            parser.retained().canonical_bytes(),
            metadata,
            grant,
        )?;
        let bytes = encoding::encode(&inventory, grant)?;
        Ok(CapturedRosterSemanticInventoryV1 {
            evidence,
            bytes,
            inventory,
        })
    });
    if result.is_err() {
        grant.poison();
    }
    result
}

pub struct DecodedUntrustedRosterSemanticInventoryV1 {
    bytes: Vec<u8>,
    inventory: wire::Inventory,
}

impl DecodedUntrustedRosterSemanticInventoryV1 {
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn member_count(&self) -> usize {
        self.inventory.members.len()
    }
}

pub fn decode_untrusted_roster_semantic_inventory_v1(
    bytes: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<DecodedUntrustedRosterSemanticInventoryV1> {
    let result = (|| {
        grant.ensure_available().map_err(domain)?;
        bound(bytes.len(), LIMIT, "roster semantic inventory")?;
        grant.charge_bytes(bytes.len()).map_err(domain)?;
        grant
            .charge_steps(bytes.len().div_ceil(32).max(1))
            .map_err(domain)?;
        let inventory: wire::Inventory = serde_json::from_slice(bytes).map_err(json)?;
        if inventory.schema != SCHEMA || inventory.interpretation != INTERPRETATION || inventory.provider_semantic_contract != crate::participation_evidence::provider_school::PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1 {
            return Err(invalid("unsupported roster semantic inventory"));
        }
        let canonical = encoding::encode(&inventory, grant)?;
        if canonical != bytes {
            return Err(invalid("noncanonical roster semantic inventory"));
        }
        Ok(DecodedUntrustedRosterSemanticInventoryV1 {
            bytes: canonical,
            inventory,
        })
    })();
    if result.is_err() {
        grant.poison();
    }
    result
}

#[cfg(test)]
mod tests;

pub(in crate::participation_evidence) fn require_fixed_roster_semantic_kernel_v1() -> StoreResult<()>
{
    kernel::compiled().map(|_| ())
}
