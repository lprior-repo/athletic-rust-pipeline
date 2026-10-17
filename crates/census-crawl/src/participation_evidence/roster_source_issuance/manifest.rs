use crate::milesplit::CurrentRosterAssertion;
use crate::participation_evidence::{bound, codec, hash, invalid, json};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::roster_affiliation_authority::RosterParserClaimV1;
use census_store::{EvidenceSubjectAttachmentStoreError, StoreResult};
use std::sync::{Arc, Mutex, OnceLock};
mod claim;
mod format;
mod validate;

pub use format::RosterParserWorkingTreeV1;

const COMPILED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/roster-parser-manifest-v1.json"));
const VCS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/roster-parser-vcs-v1.json"));
#[derive(Clone)]
pub struct CompiledRosterParserManifestV1 {
    retained: Arc<RetainedRosterParserManifestV1>,
    vcs: Arc<format::Vcs>,
}

#[derive(Debug)]
pub struct RetainedRosterParserManifestV1 {
    original: Vec<u8>,
    value: format::Manifest,
}

pub struct CheckedRosterParserManifestV1 {
    retained: Arc<RetainedRosterParserManifestV1>,
}

static COMPILED_MANIFEST: OnceLock<CompiledRosterParserManifestV1> = OnceLock::new();
static COMPILED_INITIALIZATION: Mutex<()> = Mutex::new(());

fn compile_manifest() -> StoreResult<CompiledRosterParserManifestV1> {
    let retained = RetainedRosterParserManifestV1::decode(COMPILED)?;
    bound(VCS.len(), 1024, "roster parser VCS evidence")?;
    let vcs: format::Vcs = serde_json::from_slice(VCS).map_err(json)?;
    validate::vcs(&vcs, &hash(COMPILED))?;
    if retained.value.semantic_policy != CurrentRosterAssertion::INTERPRETATION_REVISION {
        return Err(invalid(
            "compiled roster semantic policy differs from running parser",
        ));
    }
    Ok(CompiledRosterParserManifestV1 {
        retained: Arc::new(retained),
        vcs: Arc::new(vcs),
    })
}

pub fn compiled_roster_parser_manifest() -> StoreResult<CompiledRosterParserManifestV1> {
    if let Some(compiled) = COMPILED_MANIFEST.get() {
        return Ok(compiled.clone());
    }
    let _initialization = COMPILED_INITIALIZATION
        .lock()
        .map_err(|_| invalid("compiled roster parser initialization lock poisoned"))?;
    if let Some(compiled) = COMPILED_MANIFEST.get() {
        return Ok(compiled.clone());
    }
    COMPILED_MANIFEST
        .set(compile_manifest()?)
        .map_err(|_| invalid("compiled roster parser already initialized"))?;
    COMPILED_MANIFEST
        .get()
        .cloned()
        .ok_or_else(|| invalid("compiled roster parser initialization missing"))
}

impl CompiledRosterParserManifestV1 {
    pub fn canonical_bytes(&self) -> &[u8] {
        self.retained.canonical_bytes()
    }
    pub fn parser_claim(&self) -> StoreResult<RosterParserClaimV1> {
        self.retained.parser_claim()
    }
    pub fn git_commit(&self) -> &str {
        &self.vcs.commit
    }
    pub fn working_tree(&self) -> RosterParserWorkingTreeV1 {
        self.vcs.state
    }
    pub fn vcs_evidence_bytes(&self) -> &'static [u8] {
        VCS
    }
    pub fn retained(&self) -> &RetainedRosterParserManifestV1 {
        &self.retained
    }
    pub fn require_claim(&self, claim: &RosterParserClaimV1) -> StoreResult<()> {
        if self.parser_claim()? != *claim {
            return Err(invalid(
                "roster parser claim differs from compiled source manifest",
            ));
        }
        Ok(())
    }
}

impl RetainedRosterParserManifestV1 {
    pub fn decode(bytes: &[u8]) -> StoreResult<Self> {
        bound(
            bytes.len(),
            format::MAX_MANIFEST,
            "retained roster parser manifest",
        )?;
        let value: format::Manifest = serde_json::from_slice(bytes).map_err(json)?;
        validate::manifest(&value)?;
        let original = codec::encode(&value, format::MAX_MANIFEST)?;
        if original != bytes {
            return Err(invalid("retained roster parser manifest is not canonical"));
        }
        Ok(Self { original, value })
    }
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.original
    }
    pub fn semantic_policy(&self) -> &str {
        &self.value.semantic_policy
    }
    pub fn parser_claim(&self) -> StoreResult<RosterParserClaimV1> {
        claim::derive(&self.value, &self.original)
    }
    pub fn schema_bytes(&self) -> StoreResult<Vec<u8>> {
        codec::encode(&self.value.schema, format::MAX_MANIFEST)
    }
    pub fn configuration_bytes(&self) -> StoreResult<Vec<u8>> {
        codec::encode(&self.value.configuration, format::MAX_MANIFEST)
    }
    pub fn dependencies_bytes(&self) -> StoreResult<Vec<u8>> {
        codec::encode(&self.value.dependencies, format::MAX_MANIFEST)
    }
}

impl CheckedRosterParserManifestV1 {
    pub fn retained(&self) -> &RetainedRosterParserManifestV1 {
        &self.retained
    }
}

pub fn assess_retained_roster_parser_manifest(
    bytes: &[u8],
    claim: &RosterParserClaimV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<CheckedRosterParserManifestV1> {
    let assessed = assess(bytes, claim, grant);
    if assessed.is_err() {
        grant.poison();
    }
    assessed
}

fn assess(
    bytes: &[u8],
    claim: &RosterParserClaimV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<CheckedRosterParserManifestV1> {
    bound(
        bytes.len(),
        format::MAX_MANIFEST,
        "roster parser replay bytes",
    )?;
    grant
        .ensure_available()
        .map_err(EvidenceSubjectAttachmentStoreError::Domain)?;
    let input_bytes = bytes
        .len()
        .checked_mul(4)
        .ok_or(census_store::StoreError::CounterOverflow)?;
    grant
        .charge_bytes(input_bytes)
        .map_err(EvidenceSubjectAttachmentStoreError::Domain)?;
    grant
        .charge_steps(1)
        .map_err(EvidenceSubjectAttachmentStoreError::Domain)?;
    if bytes == COMPILED {
        let compiled = compiled_roster_parser_manifest()?;
        compiled.require_claim(claim)?;
        return Ok(CheckedRosterParserManifestV1 {
            retained: Arc::clone(&compiled.retained),
        });
    }
    let additional_bytes = COMPILED
        .len()
        .checked_mul(4)
        .ok_or(census_store::StoreError::CounterOverflow)?;
    grant
        .charge_bytes(additional_bytes)
        .map_err(EvidenceSubjectAttachmentStoreError::Domain)?;
    grant
        .charge_steps(511)
        .map_err(EvidenceSubjectAttachmentStoreError::Domain)?;
    let retained = RetainedRosterParserManifestV1::decode(bytes)?;
    let compiled = compiled_roster_parser_manifest()?;
    match retained.value.version {
        1 => require_v1(&retained, &compiled, claim)?,
        _ => return Err(invalid("unsupported retained roster parser adapter")),
    }
    Ok(CheckedRosterParserManifestV1 {
        retained: Arc::new(retained),
    })
}

fn require_v1(
    retained: &RetainedRosterParserManifestV1,
    compiled: &CompiledRosterParserManifestV1,
    claim: &RosterParserClaimV1,
) -> StoreResult<()> {
    if retained.canonical_bytes() != compiled.canonical_bytes() {
        return Err(invalid(
            "retained roster parser source provenance has no supported adapter",
        ));
    }
    compiled.require_claim(claim)
}

#[cfg(test)]
mod tests;
